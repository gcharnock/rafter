use crate::{Raft, RaftConfig};
use crate::RaftStatus::{Follower, Candidate};
use crate::transport::{RaftRPC, RequestVote, IncomingRaftMessage, AppendEntries, RequestVoteResponse};

use crate::test::mock_time_oracle::MockTimeOracle;
use crate::test::{PEER_A, MIN_TIMEOUT, PEER_B, DELTA_100MS};
use crate::test::setup_test;

#[test]
fn follower_remains_follower() {
    let test = setup_test(3);
    test.time_oracle.push_duration(*MIN_TIMEOUT);
    Raft::start(test.raft.clone());

    test.time_oracle.add_time(*MIN_TIMEOUT / 2);
    test.transport.send_to(IncomingRaftMessage {
        recv_from: PEER_A,
        term: 0,
        rpc: RaftRPC::AppendEntries(AppendEntries {}),
    });

    test.time_oracle.add_time(*MIN_TIMEOUT / 2);
    test.transport.send_to(IncomingRaftMessage {
        recv_from: PEER_A,
        term: 0,
        rpc: RaftRPC::AppendEntries(AppendEntries {}),
    });

    assert_eq!(test.raft.state.read().unwrap().status, Follower);
}

#[test]
fn append_entries_correct_term_number() {
    let test = setup_test(3);
    test.time_oracle.push_duration(*MIN_TIMEOUT);
    Raft::start(test.raft.clone());

    test.transport.send_to(IncomingRaftMessage {
        recv_from: PEER_A,
        term: 0,
        rpc: RaftRPC::AppendEntries(AppendEntries {}),
    });

    let response =
        test.transport.expect_append_entries_response(PEER_A);
    assert!(response.success);
}

#[test]
fn append_entries_updates_term_number() {
    let test = setup_test(3);
    test.time_oracle.push_duration(*MIN_TIMEOUT);
    Raft::start(test.raft.clone());

    test.transport.send_to(IncomingRaftMessage {
        recv_from: PEER_A,
        term: 2,
        rpc: RaftRPC::AppendEntries(AppendEntries {}),
    });

    assert_eq!(test.raft.state.read().unwrap().term_number, 2);
}


#[test]
fn append_entries_incorrect_term_number() {
    let test = setup_test(3);
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

    //then
    test.transport.expect_append_entries_response(PEER_A);
}

#[test]
fn follower_becomes_candidate() {
    let test = setup_test(3);
    test.time_oracle.push_duration(*MIN_TIMEOUT);
    Raft::start(test.raft.clone());

    test.time_oracle.push_duration(*DELTA_100MS * 15);


    test.time_oracle.add_time(*DELTA_100MS);
    assert_eq!(test.raft.state.read().unwrap().status, Follower);
    assert_eq!(test.raft.state.read().unwrap().term_number, 0);

    test.time_oracle.add_time(*DELTA_100MS * 14);

    assert_eq!(test.raft.state.read().unwrap().status, Candidate(1));
    assert_eq!(test.raft.state.read().unwrap().term_number, 1);

    test.transport.expect_request_vote_message(1);
    test.transport.expect_request_vote_message(2);
}

#[test]
fn follower_state_reset_on_new_term() {
    let test = setup_test(3);
    test.time_oracle.push_duration(*DELTA_100MS * 15);
    test.transport.send_to(IncomingRaftMessage {
        recv_from: PEER_A,
        term: 1,
        rpc: RaftRPC::RequestVote(RequestVote {}),
    });
    test.transport.expect_vote(PEER_A);

    test.transport.send_to(IncomingRaftMessage {
        recv_from: PEER_B,
        term: 2,
        rpc: RaftRPC::AppendEntries(AppendEntries {}),
    });

    let state = test.raft.state.read().unwrap();
    assert_eq!(state.has_voted_this_term, false);
    assert_eq!(state.term_number, 2);
    assert_eq!(state.status, Follower);
}

#[test]
fn follows_refuses_vote_bad_term() {
    let test = setup_test(3);
    test.time_oracle.push_duration(*MIN_TIMEOUT);
    Raft::start(test.raft.clone());
    test.raft.state.write().unwrap().term_number = 1;

    test.transport.send_to(IncomingRaftMessage {
        recv_from: PEER_A,
        term: 0,
        rpc: RaftRPC::RequestVote(RequestVote {}),
    });

    let vote = test.transport.expect_vote(1);
    assert!(!vote.vote_granted)
}

#[test]
fn follower_does_not_double_vote() {
    let test = setup_test(3);
    test.time_oracle.push_duration(*DELTA_100MS * 15);
    Raft::start(test.raft.clone());

    test.transport.send_to(IncomingRaftMessage {
        recv_from: PEER_A,
        term: 1,
        rpc: RaftRPC::RequestVote(RequestVote {}),
    });
    test.transport.expect_vote(PEER_A);

    test.transport.send_to(IncomingRaftMessage {
        recv_from: PEER_B,
        term: 1,
        rpc: RaftRPC::RequestVote(RequestVote {}),
    });
    let vote = test.transport.expect_vote(PEER_B);
    assert!(!vote.vote_granted);
}


#[test]
fn follower_grants_vote() {
    let test = setup_test(3);
    test.time_oracle.push_duration(*DELTA_100MS * 15);
    Raft::start(test.raft.clone());

    test.transport.send_to(IncomingRaftMessage {
        recv_from: PEER_A,
        term: 1,
        rpc: RaftRPC::RequestVote(RequestVote {}),
    });

    let vote = test.transport.expect_vote(1);
    assert!(vote.vote_granted)
}

