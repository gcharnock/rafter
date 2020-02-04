
#[macro_use]
extern crate log;

use std::time::Duration;
use crate::time_oracle::TimeOracle;
use std::sync::{Arc, RwLock};

mod time_oracle;


struct RaftConfig {
    min_election_timeout: Duration,
    max_election_timeout: Duration,
    pool_size: u16,
}

impl RaftConfig {
    fn new(pool_size: u16, min_election_timeout: Duration, max_election_timeout: Duration) -> RaftConfig {
        if min_election_timeout > max_election_timeout {
            panic!("min_electrion_timeout should be smaller than max_election_timeout");
        }
        RaftConfig {
            min_election_timeout,
            max_election_timeout,
            pool_size,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
enum RaftStatus {
    Leader,
    Follower,
    Candidate,
}

enum RaftRPC {
    RequestVote,
    AppendEntries,
}

struct RaftState {
    status: RaftStatus,
    has_voted_this_term: bool,
    term_number: u64,
}

struct Raft<'a> {
    state: Arc<RwLock<RaftState>>,
    time_oracle: &'a dyn TimeOracle,
    raft_config: RaftConfig,
}

impl Raft<'_> {
    pub fn new<'a>(raft_config: RaftConfig, time_oracle: &'a dyn TimeOracle) -> Raft<'a> {
        Raft {
            state: Arc::new(RwLock::new(RaftState {
                status: RaftStatus::Follower,
                has_voted_this_term: false,
                term_number: 0,
            })),
            time_oracle,
            raft_config,
        }
    }

    pub fn start(&self) {
        info!("Started a Raft instance");
        let state_clone = self.state.clone();
        let callback = move || Raft::election_expire(state_clone);
        self.time_oracle.set_timer(self.raft_config.min_election_timeout, Box::new(callback));
    }

    fn election_expire(raft_state: Arc<RwLock<RaftState>>) {
        debug!("election_expire");
        let mut state = raft_state.write().unwrap();
        state.status = RaftStatus::Candidate;
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
    use crate::{Raft, RaftConfig, time_oracle};
    use std::time::Duration;
    use crate::RaftStatus::{Follower, Candidate};
    use crate::logging_setup::start_logger;

    #[test]
    fn one_server() {
        start_logger();
        let delta_100ms = Duration::new(0, 100 * 1000 * 1000);
        let min_timeout = delta_100ms * 10;
        let max_timeout = delta_100ms * 20;

        let raft_config = RaftConfig::new(
            1, min_timeout, max_timeout,
        );
        let time_oracle
            = time_oracle::MockTimeOracle::new();
        let raft = Raft::new(raft_config, &time_oracle);
        raft.start();

        time_oracle.add_time(delta_100ms);
        assert_eq!(raft.state.read().unwrap().status, Follower);
        time_oracle.add_time(max_timeout);
        assert_eq!(raft.state.read().unwrap().status, Candidate);
    }
}
