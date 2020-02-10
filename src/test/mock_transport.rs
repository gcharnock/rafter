use std::cell::RefCell;
use std::collections::VecDeque;
use crate::transport::{OutgoingRaftMessage, IncomingRaftMessage, RaftRPC, Transport, AppendEntriesResponse, RequestVote, AppendEntries, RequestVoteResponse};
use std::rc::Rc;
use crate::Raft;

pub struct MockTransport<'s> {
    raft: RefCell<Option<Rc<Raft<'s, u32>>>>,
    send_queue: RefCell<VecDeque<OutgoingRaftMessage<u32>>>,
    recv_queue: RefCell<VecDeque<IncomingRaftMessage<u32>>>,
}

impl<'s> MockTransport<'s> {
    pub fn new() -> Self {
        Self {
            raft: RefCell::new(None),
            send_queue: RefCell::new(VecDeque::new()),
            recv_queue: RefCell::new(VecDeque::new()),
        }
    }

    pub fn inject_raft(&self, raft: Rc<Raft<'s, u32>>) {
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

    pub fn expect_append_entries(&self, node_id: u32) -> AppendEntries {
        let msg = self.send_queue.borrow_mut().pop_front().unwrap();
        assert_eq!(msg.send_to, node_id);
        if let RaftRPC::AppendEntries(append_entries) = msg.rpc {
            append_entries;
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

    pub fn send_to(&self, message: IncomingRaftMessage<u32>) {
        self.recv_queue.borrow_mut().push_back(message);
    }
}

impl<'s> Transport<u32> for MockTransport<'s> {
    fn send_msg(&self, msg: OutgoingRaftMessage<u32>) {
        self.send_queue.borrow_mut().push_back(msg);
    }

    fn read_msg(&self) -> Option<IncomingRaftMessage<u32>> {
        self.recv_queue.borrow_mut().pop_front()
    }
}
