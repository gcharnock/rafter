#[macro_use]
extern crate log;

use crate::time_oracle::TimeOracle;
use std::sync::{Arc, RwLock};
use crate::transport::{Transport, RequestVote};
use crate::config::RaftConfig;
use std::rc::Rc;

pub mod config;
pub mod time_oracle;
pub mod transport;


#[derive(Debug, Eq, PartialEq)]
pub enum RaftStatus {
    Leader,
    Follower,
    Candidate,
}

pub struct RaftState {
    status: RaftStatus,
    has_voted_this_term: bool,
    term_number: u64,
}

pub struct Raft<'s, NodeId> {
    node_id: NodeId,
    state: Arc<RwLock<RaftState>>,
    time_oracle: Rc<dyn TimeOracle<'s>>,
    raft_config: RaftConfig<NodeId>,
    transport: Rc<dyn Transport<NodeId>>,
}

impl<'s, NodeId: Copy + 's> Raft<'s, NodeId> {
    pub fn new(node_id: NodeId,
               raft_config: RaftConfig<NodeId>,
               time_oracle: Rc<dyn TimeOracle<'s>>,
               transport: Rc<dyn Transport<NodeId>>)
               -> Raft<'s, NodeId> {
        Raft {
            node_id,
            state: Arc::new(RwLock::new(RaftState {
                status: RaftStatus::Follower,
                has_voted_this_term: false,
                term_number: 0,
            })),
            time_oracle,
            raft_config,
            transport,
        }
    }

    pub fn start(this: Rc<Self>) {
        info!("Started a Raft instance");
        let that = this.clone();
        let callback = move || that.election_expire();
        let callback_box = Box::new(callback);
        let election_timeout = this.time_oracle.get_random_duration(
            this.raft_config.get_min_election_timeout(),
            this.raft_config.get_max_election_timeout());
        this.time_oracle.set_timer(election_timeout, callback_box);
    }

    fn election_expire(&self) {
        debug!("election_expire");
        let mut state = self.state.write().unwrap();
        state.status = RaftStatus::Candidate;
        state.term_number += 1;
        for peer in self.raft_config.get_peer_ids().iter() {
            self.transport.request_vote(*peer, RequestVote {
                term: state.term_number,
                node_id: self.node_id,
            });
        }
    }
}

#[cfg(test)]
mod logging_setup {
    extern crate env_logger;

    use log::LevelFilter::Trace;
    use std::io::Write;

    pub fn start_logger() {
        let _ = env_logger::builder()
            .format(|buf, record|
                writeln!(buf, "{}:{} {} - {}",
                         record.file().unwrap_or("<UNKNOWN>"),
                         record.line().unwrap_or(0),
                         record.level(),
                         record.args()))
            .filter_level(Trace)
            .is_test(true)
            .try_init();
    }
}


#[cfg(test)]
#[macro_use]
extern crate lazy_static;

#[cfg(test)]
mod tests {
    use crate::{Raft, RaftConfig, time_oracle, transport};
    use std::time::Duration;
    use crate::RaftStatus::{Follower, Candidate};
    use crate::logging_setup::start_logger;
    use std::rc::Rc;
    use crate::transport::{RaftMessage, RaftRPC, RequestVote};

    lazy_static! {
        static ref DELTA_100MS: Duration = Duration::new(0, 100 * 1000 * 1000);
        static ref MIN_TIMEOUT: Duration = *DELTA_100MS * 10;
        static ref MAX_TIMEOUT: Duration = *DELTA_100MS * 20;
    }

    #[test]
    fn isolated_node() {
        start_logger();

        let time_oracle
            = Rc::new(time_oracle::MockTimeOracle::new());
        time_oracle.push_duration(*DELTA_100MS * 15);

        let transport =
            Rc::new(transport::MockTransport::new());

        let raft_config = RaftConfig::<u32>::new(
            3,
            vec!(1, 2),
            *MIN_TIMEOUT,
            *MAX_TIMEOUT,
        );
        let raft = Raft::<u32>::new(
            0,
            raft_config,
            time_oracle.clone(),
            transport.clone());

        let raft = Rc::new(raft);
        Raft::start(raft.clone());

        time_oracle.add_time(*DELTA_100MS);
        assert_eq!(raft.state.read().unwrap().status, Follower);
        assert_eq!(raft.state.read().unwrap().term_number, 0);

        time_oracle.add_time(*DELTA_100MS * 14);

        assert_eq!(raft.state.read().unwrap().status, Candidate);
        assert_eq!(raft.state.read().unwrap().term_number, 1);

        transport.expect_request_vote_message(1);
        transport.expect_request_vote_message(2);

        drop(time_oracle);
        drop(transport);
    }

    #[test]
    fn follower_grants_vote() {
        start_logger();

        let time_oracle
            = Rc::new(time_oracle::MockTimeOracle::new());
        time_oracle.push_duration(*DELTA_100MS * 15);

        let transport =
            Rc::new(transport::MockTransport::new());

        let raft_config = RaftConfig::<u32>::new(
            3,
            vec!(1, 2),
            *MIN_TIMEOUT,
            *MAX_TIMEOUT,
        );
        let raft = Raft::<u32>::new(
            0,
            raft_config,
            time_oracle.clone(),
            transport.clone());

        let raft = Rc::new(raft);
        Raft::start(raft.clone());

        transport.send_to(RaftMessage {
            address: 0,
            rpc: RaftRPC::<u32>::RequestVote(RequestVote {
                node_id: 1,
                term: 1,
            }),
        });

        //transport.expect_vote()
    }
}
