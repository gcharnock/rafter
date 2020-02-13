use crate::{Raft, RaftConfig};
use crate::RaftStatus::{Follower, Candidate, Leader};
use crate::transport::{RaftRPC, RequestVote, IncomingRaftMessage, AppendEntries, RequestVoteResponse};

use crate::test::mock_time_oracle::MockTimeOracle;
use crate::test::{PEER_A, MIN_TIMEOUT, PEER_B};
use crate::test::setup_test;

#[test]
fn candidate_is_not_voted_for() {
    let test = setup_test(3);
    test.time_oracle.push_duration(*MIN_TIMEOUT);
    Raft::start(test.raft.clone());
    test.raft.state.write().unwrap().status = Candidate(0);
    test.raft.state.write().unwrap().term_number = 1;

    test.transport.send_to(IncomingRaftMessage {
        recv_from: PEER_A,
        term: 1,
        rpc: RaftRPC::RequestVoteResponse(RequestVoteResponse {
            vote_granted: false
        }),
    });
    assert_eq!(test.raft.state.read().unwrap().status, Candidate(0));
}


#[test]
fn candidate_wins_election() {
    let test = setup_test(3);
    test.time_oracle.push_duration(*MIN_TIMEOUT);
    Raft::start(test.raft.clone());
    test.raft.state.write().unwrap().status = Candidate(1);
    test.raft.state.write().unwrap().term_number = 1;

    test.transport.send_to(IncomingRaftMessage {
        recv_from: PEER_A,
        term: 1,
        rpc: RaftRPC::RequestVoteResponse(RequestVoteResponse {
            vote_granted: true
        }),
    });
    assert_eq!(test.raft.state.read().unwrap().status, Leader);
    test.transport.expect_append_entries(PEER_A);
    test.transport.expect_append_entries(PEER_B);
}


#[test]
fn candidate_voted_for_once_quorum_3() {
    let test = setup_test(5);
    test.time_oracle.push_duration(*MIN_TIMEOUT);
    Raft::start(test.raft.clone());
    test.raft.state.write().unwrap().status = Candidate(0);
    test.raft.state.write().unwrap().term_number = 1;

    test.transport.send_to(IncomingRaftMessage {
        recv_from: PEER_A,
        term: 1,
        rpc: RaftRPC::RequestVoteResponse(RequestVoteResponse {
            vote_granted: true
        }),
    });
    assert_eq!(test.raft.state.read().unwrap().status, Candidate(1));
}

