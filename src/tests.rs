#![cfg(test)]

use crate::{Raft, RaftConfig, time_oracle, transport};
use std::time::Duration;
use crate::RaftStatus::{Follower, Candidate};
use crate::logging_setup::start_logger;
use std::rc::Rc;
use crate::transport::{RaftRPC, RequestVote, IncomingRaftMessage, MockTransport};
use crate::time_oracle::{TimeOracle, MockTimeOracle};
use crate::transport::RaftRPC::AppendEntries;

lazy_static! {
        static ref DELTA_100MS: Duration = Duration::new(0, 100 * 1000 * 1000);
        static ref MIN_TIMEOUT: Duration = *DELTA_100MS * 10;
        static ref MAX_TIMEOUT: Duration = *DELTA_100MS * 20;
    }


struct Test<'a> {
    time_oracle: Rc<MockTimeOracle<'a>>,
    transport: Rc<MockTransport>,
    raft: Rc<Raft<'a, u32>>,
}

fn setup_test() -> Box<Test<'static>> {
    start_logger();
    let time_oracle
        = Rc::new(time_oracle::MockTimeOracle::new());

    let transport =
        Rc::new(transport::MockTransport::new(0));

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

    return Box::new(Test { raft, transport, time_oracle });
}

#[test]
fn follower_remains_follower() {
    let test = setup_test();
    test.time_oracle.push_duration(*MIN_TIMEOUT);
    Raft::start(test.raft.clone());

    test.time_oracle.add_time(*MIN_TIMEOUT/2);
    test.transport.send_to(IncomingRaftMessage {
        recv_from: 1,
        rpc: AppendEntries
    });
    test.raft.loop_iter();

    test.time_oracle.add_time(*MIN_TIMEOUT/2);
    test.transport.send_to(IncomingRaftMessage {
        recv_from: 1,
        rpc: AppendEntries
    });
    test.raft.loop_iter();

    assert_eq!(test.raft.state.read().unwrap().status, Follower);
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
    start_logger();

    let time_oracle
        = Rc::new(time_oracle::MockTimeOracle::new());
    time_oracle.push_duration(*DELTA_100MS * 15);

    let transport =
        Rc::new(transport::MockTransport::new(0));

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

    transport.send_to(IncomingRaftMessage {
        recv_from: 1,
        rpc: RaftRPC::RequestVote(RequestVote {
            term: 1,
        }),
    });
    raft.loop_iter();

    transport.expect_vote(1);
}
