use crate::{Raft, RaftConfig};
use crate::RaftStatus::{Follower, Candidate, Leader};
use crate::transport::{RaftRPC, RequestVote, IncomingRaftMessage, AppendEntries, RequestVoteResponse};

use crate::test::mock_time_oracle::MockTimeOracle;
use crate::test::{PEER_A, MIN_TIMEOUT, PEER_B};
use crate::test::setup_test;

#[test]
fn leader_sends_append_entires() {
    let test = setup_test(3);
    test.time_oracle.push_duration(*MIN_TIMEOUT);
    Raft::start(test.raft.clone());
    test.raft.state.write().unwrap().status = Leader;

    test.time_oracle.add_time(*MIN_TIMEOUT);
    test.transport.expect_append_entries(1);
    test.transport.expect_append_entries(2);
}
