use std::cell::RefCell;
use std::collections::VecDeque;
use crate::transport::{OutgoingRaftMessage, IncomingRaftMessage, RaftRPC, Transport, AppendEntriesResponse, RequestVote, AppendEntries, RequestVoteResponse};
use std::rc::Rc;
use crate::{Raft, StateMachine};
use crate::test::{TestRaft, TestLog, TestId};

pub struct MockSateMachine {
    committed: Vec<TestLog>
}

impl MockSateMachine {
    pub fn new() -> Self {
        Self {
            committed: Vec::new()
        }
    }
}

impl StateMachine<u32> for MockSateMachine {
    fn apply(&mut self, log: u32) {
        self.committed.push(log);
    }
}


pub struct MockTransport<'s> {
    raft: RefCell<Option<Rc<TestRaft<'s>>>>,
    send_queue: RefCell<VecDeque<OutgoingRaftMessage<TestId, TestLog>>>,
    recv_queue: RefCell<VecDeque<IncomingRaftMessage<TestId, TestLog>>>,
}

impl<'s> MockTransport<'s> {
    pub fn new() -> Self {
        Self {
            raft: RefCell::new(None),
            send_queue: RefCell::new(VecDeque::new()),
            recv_queue: RefCell::new(VecDeque::new()),
        }
    }

    pub fn inject_raft(&self, raft: Rc<Raft<'s, u32, u32, MockSateMachine>>) {
        // This creates a circular reference, that would be bad in non-test code.
        *self.raft.borrow_mut() = Some(raft);
    }

    pub fn expect_request_vote_message(&self, node_id: u32) -> RequestVote {
        let msg = self.send_queue.borrow_mut().pop_front().unwrap();
        assert_eq!(msg.send_to, node_id);
        if let RaftRPC::RequestVote(request_vote) = msg.rpc {
            return request_vote;
        }
        panic!("Bad message type");
    }

    pub fn expect_append_entries(&self, node_id: u32) -> AppendEntries<TestLog> {
        let msg = self.send_queue.borrow_mut().pop_front().unwrap();
        assert_eq!(msg.send_to, node_id);
        if let RaftRPC::AppendEntries(append_entries) = msg.rpc {
            return append_entries;
        }
        panic!("Bad message type");
    }

    pub fn expect_append_entries_response(&self, node_id: u32) -> AppendEntriesResponse {
        let msg = self.send_queue.borrow_mut().pop_front().unwrap();
        assert_eq!(msg.send_to, node_id);
        if let RaftRPC::AppendEntriesResponse(result) = msg.rpc {
            return result;
        }
        panic!("Bad message type");
    }

    pub fn expect_vote(&self, node_id: u32) -> RequestVoteResponse {
        let msg = self.send_queue.borrow_mut().pop_front().unwrap();
        assert_eq!(msg.send_to, node_id);
        if let RaftRPC::RequestVoteResponse(vote) = msg.rpc {
            return vote;
        }
        panic!("Bad message type")
    }

    pub fn send_to(&self, message: IncomingRaftMessage<TestId, TestLog>) {
        self.recv_queue.borrow_mut().push_back(message);
        if let Some(ref raft) = *self.raft.borrow() {
            raft.loop_iter();
        }
    }
}

impl<'s> Transport<TestId, TestLog> for MockTransport<'s> {
    fn send_msg(&self, msg: OutgoingRaftMessage<TestId, TestLog>) {
        self.send_queue.borrow_mut().push_back(msg);
    }

    fn read_msg(&self) -> Option<IncomingRaftMessage<TestId, TestLog>> {
        self.recv_queue.borrow_mut().pop_front()
    }
}
