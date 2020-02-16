use crate::{LeaderState};
use crate::RaftStatus;

use crate::test::{setup_test, PEER_A, PEER_B};

#[test]
fn leader_sends_append_entires() {
    let mut raft = setup_test(3);
    raft.become_leader();
    raft.state.status = RaftStatus::Leader(LeaderState::new());
    raft.on_leader_timeout();

    raft.raft_io.expect_append_entries(vec![PEER_A, PEER_B]);
}
