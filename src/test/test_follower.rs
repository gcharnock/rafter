use crate::{RaftStatus};
use crate::transport::{RaftRPC, RequestVote, IncomingRaftMessage, AppendEntries};

use crate::test::{PEER_A, PEER_B};
use crate::test::setup_test;

#[test]
fn follower_remains_follower() {
    let mut raft = setup_test(3);

    raft.on_raft_message(IncomingRaftMessage {
        recv_from: PEER_A,
        term: 0,
        rpc: RaftRPC::AppendEntries(AppendEntries {
            prev_log_index: 0,
            prev_log_term: 0,
            entries: vec![],
            leader_commit: 0
        }),
    });

    raft.on_raft_message(IncomingRaftMessage {
        recv_from: PEER_A,
        term: 0,
        rpc: RaftRPC::AppendEntries(AppendEntries {
            prev_log_index: 0,
            prev_log_term: 0,
            entries: vec![],
            leader_commit: 0
        }),
    });

    assert!(raft.state.status.is_follower());
}

#[test]
fn append_entries_correct_term_number() {
    let mut raft = setup_test(3);

    raft.on_raft_message(IncomingRaftMessage {
        recv_from: PEER_A,
        term: 0,
        rpc: RaftRPC::AppendEntries(AppendEntries {
            prev_log_index: 0,
            prev_log_term: 0,
            entries: vec![],
            leader_commit: 0
        }),
    });

    let response =
        raft.raft_io.expect_append_entries_response(PEER_A);
    assert!(response.success);
}

#[test]
fn append_entries_updates_term_number() {
    let mut raft = setup_test(3);

    raft.on_raft_message(IncomingRaftMessage {
        recv_from: PEER_A,
        term: 2,
        rpc: RaftRPC::AppendEntries(AppendEntries {
            prev_log_index: 0,
            prev_log_term: 0,
            entries: vec![],
            leader_commit: 0
        }),
    });

    assert_eq!(raft.state.term_number, 2);
}


#[test]
fn append_entries_incorrect_term_number() {
    let mut raft = setup_test(3);

    //given
    raft.state.term_number = 1;

    //if
    raft.on_raft_message(IncomingRaftMessage {
        recv_from: PEER_A,
        term: 0,
        rpc: RaftRPC::AppendEntries(AppendEntries {
            prev_log_index: 0,
            prev_log_term: 0,
            entries: vec![],
            leader_commit: 0
        }),
    });

    //then
    raft.raft_io.expect_append_entries_response(PEER_A);
}

#[test]
fn follower_becomes_candidate() {
    let mut raft = setup_test(3);

    assert!(raft.state.status.is_follower());
    assert_eq!(raft.state.term_number, 0);

    raft.on_election_expire();

    match raft.state.status {
        RaftStatus::Candidate(vote_count) => {
            assert_eq!(vote_count, 1)
        },
        _ => panic!()
    }
    assert_eq!(raft.state.term_number, 1);

    raft.raft_io.expect_request_vote_message(1);
    raft.raft_io.expect_request_vote_message(2);
}

#[test]
fn follower_state_reset_on_new_term() {
    let mut raft = setup_test(3);
    raft.on_raft_message(IncomingRaftMessage {
        recv_from: PEER_A,
        term: 1,
        rpc: RaftRPC::RequestVote(RequestVote {}),
    });
    raft.raft_io.expect_vote(PEER_A);

    raft.on_raft_message(IncomingRaftMessage {
        recv_from: PEER_B,
        term: 2,
        rpc: RaftRPC::AppendEntries(AppendEntries {
            prev_log_index: 0,
            prev_log_term: 0,
            entries: vec![],
            leader_commit: 0
        }),
    });

    let state = &raft.state;
    assert!(state.voted_for.is_none());
    assert_eq!(state.term_number, 2);
    assert!(state.status.is_follower());
}

#[test]
fn follows_refuses_vote_bad_term() {
    let mut raft = setup_test(3);
    raft.state.term_number = 1;

    raft.on_raft_message(IncomingRaftMessage {
        recv_from: PEER_A,
        term: 0,
        rpc: RaftRPC::RequestVote(RequestVote {}),
    });

    let vote = raft.raft_io.expect_vote(1);
    assert!(!vote.vote_granted)
}

#[test]
fn follower_does_not_double_vote() {
    let mut raft = setup_test(3);

    raft.on_raft_message(IncomingRaftMessage {
        recv_from: PEER_A,
        term: 1,
        rpc: RaftRPC::RequestVote(RequestVote {}),
    });
    raft.raft_io.expect_vote(PEER_A);

    raft.on_raft_message(IncomingRaftMessage {
        recv_from: PEER_B,
        term: 1,
        rpc: RaftRPC::RequestVote(RequestVote {}),
    });
    let vote = raft.raft_io.expect_vote(PEER_B);
    assert!(!vote.vote_granted);
}


#[test]
fn follower_grants_vote() {
    let mut raft = setup_test(3);

    raft.on_raft_message(IncomingRaftMessage {
        recv_from: PEER_A,
        term: 1,
        rpc: RaftRPC::RequestVote(RequestVote {}),
    });

    let vote = raft.raft_io.expect_vote(1);
    assert!(vote.vote_granted)
}


