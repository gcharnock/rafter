use crate::{RaftStatus, LogEntry};
use crate::transport::{RaftRPC, RequestVote, IncomingRaftMessage, AppendEntries};

use crate::test::{PEER_A, PEER_B, SELF_ID, TestId, TestLog, get_log};
use crate::test::setup_test;

#[test]
fn follower_remains_follower() {
    let mut raft = setup_test(3);

    //Given

    //if
    raft.on_raft_message(IncomingRaftMessage {
        recv_from: PEER_A,
        term: 0,
        rpc: RaftRPC::AppendEntries(AppendEntries {
            prev_log_index: 0,
            prev_log_term: 0,
            entries: vec![],
            leader_commit: 0,
        }),
    });

    //then
    raft.raft_io.expect_reset();
    assert!(raft.state.status.is_follower());
}

#[test]
fn append_entries_correct_term_number() {
    let mut raft = setup_test(3);

    //given

    // if
    raft.on_raft_message(IncomingRaftMessage {
        recv_from: PEER_A,
        term: 0,
        rpc: RaftRPC::AppendEntries(AppendEntries {
            prev_log_index: 0,
            prev_log_term: 0,
            entries: vec![],
            leader_commit: 0,
        }),
    });

    // then
    let response =
        raft.raft_io.expect_append_entries_response(PEER_A);
    assert!(response.success);
}

#[test]
fn append_entries_updates_term_number() {
    let mut raft = setup_test(3);

    // given

    // if
    raft.on_raft_message(IncomingRaftMessage {
        recv_from: PEER_A,
        term: 2,
        rpc: RaftRPC::AppendEntries(AppendEntries {
            prev_log_index: 0,
            prev_log_term: 0,
            entries: vec![],
            leader_commit: 0,
        }),
    });

    // then
    assert_eq!(raft.state.term, 2);
    let response =
        raft.raft_io.expect_append_entries_response(PEER_A);
    assert!(response.success);
}


#[test]
fn append_entries_incorrect_term_number() {
    let mut raft = setup_test(3);

    // given
    raft.state.term = 1;

    // if
    raft.on_raft_message(IncomingRaftMessage {
        recv_from: PEER_A,
        term: 0,
        rpc: RaftRPC::AppendEntries(AppendEntries {
            prev_log_index: 0,
            prev_log_term: 0,
            entries: vec![],
            leader_commit: 0,
        }),
    });

    // then
    raft.raft_io.assert_no_resets();
    let response =
        raft.raft_io.expect_append_entries_response(PEER_A);
    assert!(!response.success);
}

#[test]
fn follower_becomes_candidate() {
    // given
    let mut raft = setup_test(3);

    // if
    raft.on_election_expire();


    // then
    match raft.state.status {
        RaftStatus::Candidate(vote_count) => {
            assert_eq!(vote_count, 1)
        }
        _ => panic!()
    }
    assert_eq!(raft.state.term, 1);

    raft.raft_io.expect_request_vote_message(1);
    raft.raft_io.expect_request_vote_message(2);
}

#[test]
fn follower_state_reset_on_new_term() {
    // given
    let mut raft = setup_test(3);
    raft.on_raft_message(IncomingRaftMessage {
        recv_from: PEER_A,
        term: 1,
        rpc: RaftRPC::RequestVote(RequestVote {}),
    });
    raft.raft_io.expect_vote(PEER_A);

    // if
    raft.on_raft_message(IncomingRaftMessage {
        recv_from: PEER_B,
        term: 2,
        rpc: RaftRPC::AppendEntries(AppendEntries {
            prev_log_index: 0,
            prev_log_term: 0,
            entries: vec![],
            leader_commit: 0,
        }),
    });

    // then
    let state = &raft.state;
    assert!(state.voted_for.is_none());
    assert_eq!(state.term, 2);
    assert!(state.status.is_follower());
}

#[test]
fn follows_refuses_vote_bad_term() {
    // Given
    let mut raft = setup_test(3);
    raft.state.term = 1;

    // If
    raft.on_raft_message(IncomingRaftMessage {
        recv_from: PEER_A,
        term: 0,
        rpc: RaftRPC::RequestVote(RequestVote {}),
    });

    // then
    let vote = raft.raft_io.expect_vote(1);
    assert!(!vote.vote_granted)
}

#[test]
fn follower_does_not_double_vote() {
    // given
    let mut raft = setup_test(3);

    raft.on_raft_message(IncomingRaftMessage {
        recv_from: PEER_A,
        term: 1,
        rpc: RaftRPC::RequestVote(RequestVote {}),
    });
    raft.raft_io.expect_vote(PEER_A);

    // if
    raft.on_raft_message(IncomingRaftMessage {
        recv_from: PEER_B,
        term: 1,
        rpc: RaftRPC::RequestVote(RequestVote {}),
    });

    // then
    let vote = raft.raft_io.expect_vote(PEER_B);
    assert!(!vote.vote_granted);
}


#[test]
fn follower_grants_vote() {
    // given
    let mut raft = setup_test(3);

    // if
    raft.on_raft_message(IncomingRaftMessage {
        recv_from: PEER_A,
        term: 1,
        rpc: RaftRPC::RequestVote(RequestVote {}),
    });

    // then
    let vote = raft.raft_io.expect_vote(1);
    assert!(vote.vote_granted)
}

#[test]
fn new_logs_appended() {
    // given
    let mut raft = setup_test(3);

    // if
    raft.on_raft_message(IncomingRaftMessage {
        recv_from: PEER_A,
        term: 0,
        rpc: RaftRPC::AppendEntries(AppendEntries {
            prev_log_index: 0,
            prev_log_term: 0,
            entries: vec![1, 2, 3],
            leader_commit: 0,
        }),
    });

    // then
    assert_eq!(raft.log.into_iter().collect::<Vec<LogEntry<TestLog>>>(),
               vec![LogEntry {
                   term: 0,
                   command: 1,
               }, LogEntry {
                   term: 0,
                   command: 2,
               }, LogEntry {
                   term: 0,
                   command: 3,
               }]);
}


#[test]
fn delete_conflicting_logs() {
    // given
    let mut raft = setup_test(3);
    raft.log.push_back(LogEntry {
        term: 0,
        command: 1,
    });
    raft.state.commit_index = 1;

    // if
    raft.on_raft_message(IncomingRaftMessage {
        recv_from: PEER_A,
        term: 0,
        rpc: RaftRPC::AppendEntries(AppendEntries {
            prev_log_index: 0,
            prev_log_term: 0,
            entries: vec![2, 3, 4],
            leader_commit: 0,
        }),
    });

    // then
    assert_eq!(get_log(&mut raft),
               vec![LogEntry {
                   term: 0,
                   command: 2,
               }, LogEntry {
                   term: 0,
                   command: 3,
               }, LogEntry {
                   term: 0,
                   command: 4,
               }]);
}

#[test]
fn existing_entry_conflicts_new_entry() {
    // given
    let mut raft = setup_test(3);
    raft.log.push_back(LogEntry {
        term: 0,
        command: 1,
    });
    raft.state.commit_index = 1;

    // if
    raft.on_raft_message(IncomingRaftMessage {
        recv_from: PEER_A,
        term: 1,
        rpc: RaftRPC::AppendEntries(AppendEntries {
            prev_log_index: 1,
            prev_log_term: 1,
            entries: vec![2],
            leader_commit: 0,
        }),
    });

    // then
    assert_eq!(get_log(&mut raft), vec![LogEntry {
        term: 1,
        command: 2,
    }]);
}