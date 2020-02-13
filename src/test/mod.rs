use std::rc::Rc;
use std::time::Duration;

use crate::{Raft, RaftConfig};
use crate::RaftStatus::{Follower, Candidate, Leader};
use crate::transport::{RaftRPC, RequestVote, IncomingRaftMessage, AppendEntries, RequestVoteResponse};

use self::mock_time_oracle::MockTimeOracle;
use self::mock_transport::MockTransport;
use self::logging_setup::start_logger;

mod mock_time_oracle;
mod mock_transport;

mod test_follower;
mod test_candidate;
mod test_leader;


lazy_static! {
        static ref DELTA_100MS: Duration = Duration::new(0, 100 * 1000 * 1000);
        static ref MIN_TIMEOUT: Duration = *DELTA_100MS * 10;
        static ref MAX_TIMEOUT: Duration = *DELTA_100MS * 20;
    }

pub const SELF_ID: u32 = 0;
pub const PEER_A: u32 = 1;
pub const PEER_B: u32 = 2;
pub const PEER_C: u32 = 3;
pub const PEER_D: u32 = 4;

struct Test<'a> {
    time_oracle: Rc<MockTimeOracle<'a>>,
    transport: Rc<MockTransport<'a>>,
    raft: Rc<Raft<'a, u32>>,
}


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

fn setup_test(size: u32) -> Box<Test<'static>> {
    start_logger();
    let time_oracle = Rc::new(MockTimeOracle::new());

    let transport = Rc::new(MockTransport::new());

    let peer_ids = if size == 3 {
        vec!(PEER_A, PEER_B)
    } else if size == 5 {
        vec!(PEER_A, PEER_B, PEER_C, PEER_D)
    } else {
        panic!("Unsupported size");
    };
    let raft_config = RaftConfig::<u32>::new(
        peer_ids,
        *MIN_TIMEOUT,
        *MAX_TIMEOUT,
    );
    let raft = Raft::<u32>::new(
        SELF_ID,
        raft_config,
        time_oracle.clone(),
        transport.clone());

    let raft = Rc::new(raft);
    transport.inject_raft(raft.clone());

    return Box::new(Test { raft, transport, time_oracle });
}


