#[macro_use]
extern crate log;

use crate::time_oracle::TimeOracle;
use std::sync::{Arc, RwLock};
use crate::transport::Transport;
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

pub enum RaftRPC {
    RequestVote,
    AppendEntries,
}

pub struct RaftState {
    status: RaftStatus,
    has_voted_this_term: bool,
    term_number: u64,
}

pub struct Raft<'s, NodeId> {
    state: Arc<RwLock<RaftState>>,
    time_oracle: Rc<dyn TimeOracle<'s>>,
    raft_config: RaftConfig<NodeId>,
    transport: Rc<dyn Transport<NodeId>>,
}

impl<'s, NodeId: Copy + 's> Raft<'s, NodeId> {
    pub fn new(raft_config: RaftConfig<NodeId>,
               time_oracle: Rc<dyn TimeOracle<'s>>,
               transport: Rc<dyn Transport<NodeId>>)
               -> Raft<'s, NodeId> {
        Raft {
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
        for peer in self.raft_config.get_peer_ids().iter() {
            self.transport.request_vote(*peer);
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
mod tests {
    use crate::{Raft, RaftConfig, time_oracle, transport};
    use std::time::Duration;
    use crate::RaftStatus::{Follower, Candidate};
    use crate::logging_setup::start_logger;
    use std::rc::Rc;

    #[test]
    fn isolated_node() {
        start_logger();
        let delta_100ms = Duration::new(0, 100 * 1000 * 1000);
        let min_timeout = delta_100ms * 10;
        let max_timeout = delta_100ms * 20;

        let time_oracle
            = Rc::new(time_oracle::MockTimeOracle::new());
        time_oracle.push_duration(delta_100ms * 15);

        let transport =
            Rc::new(transport::MockTransport::new());

        let raft_config = RaftConfig::<u32>::new(
            3,
            vec!(1, 2),
            min_timeout,
            max_timeout,
        );
        let raft = Raft::<u32>::new(raft_config,
                                    time_oracle.clone(),
                                    transport.clone());
        let raft = Rc::new(raft);
        Raft::start(raft.clone());

        time_oracle.add_time(delta_100ms);
        assert_eq!(raft.state.read().unwrap().status, Follower);
        time_oracle.add_time(delta_100ms * 14);
        assert_eq!(raft.state.read().unwrap().status, Candidate);
        transport.expect_request_vote_message(1);
        transport.expect_request_vote_message(2);

        drop(time_oracle);
        drop(transport);
    }
}
