use crate::RaftStatus::{Candidate};
use crate::transport::{RaftRPC, IncomingRaftMessage, RequestVoteResponse};

use crate::test::{PEER_A, PEER_B};
use crate::test::setup_test;

#[test]
fn candidate_is_not_voted_for() {
    let mut raft = setup_test(3);
    raft.state.status = Candidate(0);
    raft.state.term_number = 1;

    raft.on_raft_message(IncomingRaftMessage {
        recv_from: PEER_A,
        term: 1,
        rpc: RaftRPC::RequestVoteResponse(RequestVoteResponse {
            vote_granted: false
        }),
    });
    assert!(raft.state.status.is_candidate());
}


#[test]
fn candidate_wins_election() {
    let mut raft = setup_test(3);
    raft.state.status = Candidate(1);
    raft.state.term_number = 1;

    raft.on_raft_message(IncomingRaftMessage {
        recv_from: PEER_A,
        term: 1,
        rpc: RaftRPC::RequestVoteResponse(RequestVoteResponse {
            vote_granted: true
        }),
    });
    assert!(raft.state.status.is_leader());
    raft.raft_io.expect_append_entries(PEER_A);
    raft.raft_io.expect_append_entries(PEER_B);
}


#[test]
fn candidate_voted_for_once_quorum_3() {
    let mut raft = setup_test(5);
    raft.on_election_expire();
    raft.state.status = Candidate(0);
    raft.state.term_number = 1;

    raft.on_raft_message(IncomingRaftMessage {
        recv_from: PEER_A,
        term: 1,
        rpc: RaftRPC::RequestVoteResponse(RequestVoteResponse {
            vote_granted: true
        }),
    });
    assert!(raft.state.status.is_candidate());
}

