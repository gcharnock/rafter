use std::cell::RefCell;
use std::collections::VecDeque;

pub trait Transport<NodeId> {
    fn append_entries(&self, node_id: NodeId);
    fn request_vote(&self, node_id: NodeId);
}


pub enum RaftRPC {
    AppendEntries,
    RequestVote,
}

pub struct RaftMessage<NodeId> {
    rpc: RaftRPC,
    address: NodeId,
}

pub struct MockTransport {
    send_queue: RefCell<VecDeque<RaftMessage<u32>>>,
    recv_queue: RefCell<VecDeque<RaftMessage<u32>>>,
}

impl MockTransport {
    pub fn new() -> Self {
        Self {
            send_queue: RefCell::new(VecDeque::new()),
            recv_queue: RefCell::new(VecDeque::new()),
        }
    }

    pub fn expect_request_vote_message(&self, node_id: u32) {
        let msg = self.send_queue.borrow_mut().pop_front().unwrap();
        assert_eq!(msg.address, node_id);
        if let RaftRPC::RequestVote = msg.rpc {
            return;
        }
        panic!("Bad message type");
    }

    pub fn expect_request_append_entries(&self, node_id: u32) {
        let msg = self.send_queue.borrow_mut().pop_front().unwrap();
        assert_eq!(msg.address, node_id);
        if let RaftRPC::AppendEntries = msg.rpc {
            return;
        }
        panic!("Bad message type");
    }
}

impl Transport<u32> for MockTransport {
    fn append_entries(&self, node_id: u32) {
        let mut queue = self.send_queue.borrow_mut();
        queue.push_back(RaftMessage {
            address: node_id,
            rpc: RaftRPC::AppendEntries,
        });
    }

    fn request_vote(&self, node_id: u32) {
        let mut queue = self.send_queue.borrow_mut();
        queue.push_back(RaftMessage {
            address: node_id,
            rpc: RaftRPC::RequestVote,
        });
    }
}
