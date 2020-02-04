#[macro_use]
extern crate log;

use crate::time_oracle::TimeOracle;
use std::sync::{Arc, RwLock};
use crate::transport::Transport;
use crate::config::RaftConfig;

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

pub struct Raft<'a, NodeId> {
    state: Arc<RwLock<RaftState>>,
    time_oracle: &'a dyn TimeOracle,
    raft_config: RaftConfig<NodeId>,
    transport: &'a dyn Transport<NodeId>,
}

impl<NodeId> Raft<'_, NodeId> {
    pub fn new<'a>(raft_config: RaftConfig<NodeId>,
                   time_oracle: &'a dyn TimeOracle,
                   transport: &'a dyn Transport<NodeId>)
                   -> Raft<'a, NodeId> {
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

    pub fn start(&self) {
        info!("Started a Raft instance");
        let state_clone = self.state.clone();
        let callback =
            || Raft::<NodeId>::election_expire(&self.raft_config, state_clone, self.transport);
        let election_timeout = self.time_oracle.get_random_duration(
            self.raft_config.get_min_election_timeout(),
            self.raft_config.get_max_election_timeout());
        self.time_oracle.set_timer(election_timeout, Box::new(callback));
    }

    fn election_expire(raft_config: &RaftConfig<NodeId>,
                       raft_state: Arc<RwLock<RaftState>>,
                       transport: &dyn Transport<NodeId>) {
        debug!("election_expire");
        let mut state = raft_state.write().unwrap();
        state.status = RaftStatus::Candidate;
        for peer in raft_config.get_peer_ids().iter() {
            transport.request_vote(peer);
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

    #[test]
    fn isolated_node() {
        start_logger();
        let delta_100ms = Duration::new(0, 100 * 1000 * 1000);
        let min_timeout = delta_100ms * 10;
        let max_timeout = delta_100ms * 20;

        let raft_config = RaftConfig::<u32>::new(
            3,
            vec!(1, 2),
            min_timeout,
            max_timeout,
        );
        let time_oracle
            = time_oracle::MockTimeOracle::new();
        time_oracle.push_duration(delta_100ms * 15);

        let transport = transport::MockTransport::new();

        let raft = Raft::<u32>::new(raft_config, &time_oracle, &transport);
        raft.start();

        time_oracle.add_time(delta_100ms);
        assert_eq!(raft.state.read().unwrap().status, Follower);
        time_oracle.add_time(delta_100ms * 14);
        assert_eq!(raft.state.read().unwrap().status, Candidate);
        transport.expect_request_vote_message(0);
    }
}
