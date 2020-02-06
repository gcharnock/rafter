use std::cell::RefCell;
use std::collections::VecDeque;

pub trait Transport<NodeId> {
    fn append_entries(&self, node_id: NodeId);
    fn request_vote(&self, node_id: NodeId, request_vote: RequestVote<NodeId>);
    fn read_msg(&self) -> RaftMessage<NodeId>;
}

pub struct RequestVote<NodeId> {
    pub term: u64,
    pub node_id: NodeId,
}

pub enum RaftRPC<NodeId> {
    AppendEntries,
    RequestVote(RequestVote<NodeId>),
}

pub struct RaftMessage<NodeId> {
    pub rpc: RaftRPC<NodeId>,
    pub address: NodeId,
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
        if let RaftRPC::RequestVote(_) = msg.rpc {
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

    pub fn send_to(&self, message: RaftMessage<u32>) {
        self.recv_queue.borrow_mut().push_back(message);
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

    fn request_vote(&self, node_id: u32, request_vote: RequestVote<u32>) {
        let mut queue = self.send_queue.borrow_mut();
        queue.push_back(RaftMessage {
            address: node_id,
            rpc: RaftRPC::RequestVote(request_vote),
        });
    }

    fn read_msg(&self) -> RaftMessage<u32> {
        let mut queue = self.recv_queue.borrow_mut();
        return queue.pop_front().unwrap();
    }
}
