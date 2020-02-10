use std::time::Duration;
use std::rc::Rc;

use crate::{Raft, RaftConfig};
use crate::RaftStatus::{Follower, Candidate};
use crate::transport::{RaftRPC, RequestVote, IncomingRaftMessage, AppendEntries};

use self::mock_time_oracle::MockTimeOracle;
use self::mock_transport::MockTransport;
use self::logging_setup::start_logger;

mod mock_time_oracle;
mod mock_transport;

lazy_static! {
        static ref DELTA_100MS: Duration = Duration::new(0, 100 * 1000 * 1000);
        static ref MIN_TIMEOUT: Duration = *DELTA_100MS * 10;
        static ref MAX_TIMEOUT: Duration = *DELTA_100MS * 20;
    }

const SELF_ID: u32 = 0;
const PEER_A: u32 = 1;
const PEER_B: u32 = 2;

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

fn setup_test() -> Box<Test<'static>> {
    start_logger();
    let time_oracle = Rc::new(MockTimeOracle::new());

    let mut transport = Rc::new(MockTransport::new());

    let raft_config = RaftConfig::<u32>::new(
        3,
        vec!(PEER_A, PEER_B),
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

#[test]
fn follower_remains_follower() {
    let test = setup_test();
    test.time_oracle.push_duration(*MIN_TIMEOUT);
    Raft::start(test.raft.clone());

    test.time_oracle.add_time(*MIN_TIMEOUT / 2);
    test.transport.send_to(IncomingRaftMessage {
        recv_from: PEER_A,
        term: 0,
        rpc: RaftRPC::AppendEntries(AppendEntries {}),
    });
    test.raft.loop_iter();

    test.time_oracle.add_time(*MIN_TIMEOUT / 2);
    test.transport.send_to(IncomingRaftMessage {
        recv_from: PEER_A,
        term: 0,
        rpc: RaftRPC::AppendEntries(AppendEntries {}),
    });
    test.raft.loop_iter();

    assert_eq!(test.raft.state.read().unwrap().status, Follower);
}

#[test]
fn append_entries_correct_term_number() {
    let test = setup_test();
    test.time_oracle.push_duration(*MIN_TIMEOUT);
    Raft::start(test.raft.clone());

    test.transport.send_to(IncomingRaftMessage {
        recv_from: PEER_A,
        term: 0,
        rpc: RaftRPC::AppendEntries(AppendEntries {}),
    });
    test.raft.loop_iter();
    let response =
        test.transport.expect_append_entries_response(PEER_A);
    assert!(response.success);
}

#[test]
fn append_entries_updates_term_number() {
    let test = setup_test();
    test.time_oracle.push_duration(*MIN_TIMEOUT);
    Raft::start(test.raft.clone());

    test.transport.send_to(IncomingRaftMessage {
        recv_from: PEER_A,
        term: 2,
        rpc: RaftRPC::AppendEntries(AppendEntries {}),
    });
    test.raft.loop_iter();
    assert_eq!(test.raft.state.read().unwrap().term_number, 2);
}


#[test]
fn append_entries_incorrect_term_number() {
    let test = setup_test();
    test.time_oracle.push_duration(*MIN_TIMEOUT);
    Raft::start(test.raft.clone());

    //given
    test.raft.state.write().unwrap().term_number = 1;

    //if
    test.transport.send_to(IncomingRaftMessage {
        recv_from: PEER_A,
        term: 0,
        rpc: RaftRPC::AppendEntries(AppendEntries {}),
    });
    test.raft.loop_iter();

    //then
    test.transport.expect_append_entries_response(PEER_A);
}

#[test]
fn follower_becomes_candidate() {
    let test = setup_test();
    test.time_oracle.push_duration(*MIN_TIMEOUT);
    Raft::start(test.raft.clone());

    test.time_oracle.push_duration(*DELTA_100MS * 15);


    test.time_oracle.add_time(*DELTA_100MS);
    assert_eq!(test.raft.state.read().unwrap().status, Follower);
    assert_eq!(test.raft.state.read().unwrap().term_number, 0);

    test.time_oracle.add_time(*DELTA_100MS * 14);

    assert_eq!(test.raft.state.read().unwrap().status, Candidate);
    assert_eq!(test.raft.state.read().unwrap().term_number, 1);

    test.transport.expect_request_vote_message(1);
    test.transport.expect_request_vote_message(2);
}

#[test]
fn follower_grants_vote() {
    let test = setup_test();
    test.time_oracle.push_duration(*DELTA_100MS * 15);
    Raft::start(test.raft.clone());

    test.transport.send_to(IncomingRaftMessage {
        recv_from: PEER_A,
        term: 1,
        rpc: RaftRPC::RequestVote(RequestVote {}),
    });
    test.raft.loop_iter();

    test.transport.expect_vote(1);
}
