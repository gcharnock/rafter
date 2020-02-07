use std::cell::RefCell;
use std::collections::VecDeque;

pub trait Transport<NodeId> {
    fn append_entries(&self, node_id: NodeId);
    fn request_vote(&self, node_id: NodeId, request_vote: RequestVote);
    fn vote(&self, node_id: NodeId);

    fn read_msg(&self) -> Option<IncomingRaftMessage<NodeId>>;
}

#[derive(Debug)]
pub struct RequestVote {
    pub term: u64,
}

#[derive(Debug)]
pub enum RaftRPC {
    AppendEntries,
    RequestVote(RequestVote),
    Vote,
}

#[derive(Debug)]
pub struct OutgoingRaftMessage<NodeId> {
    pub rpc: RaftRPC,
    pub send_to: NodeId,
}

#[derive(Debug)]
pub struct IncomingRaftMessage<NodeId> {
    pub rpc: RaftRPC,
    pub recv_from: NodeId,
}

pub struct MockTransport {
    send_queue: RefCell<VecDeque<OutgoingRaftMessage<u32>>>,
    recv_queue: RefCell<VecDeque<IncomingRaftMessage<u32>>>,
}

impl MockTransport {
    pub fn new(node_id: u32) -> Self {
        Self {
            send_queue: RefCell::new(VecDeque::new()),
            recv_queue: RefCell::new(VecDeque::new()),
        }
    }

    pub fn expect_request_vote_message(&self, node_id: u32) {
        let msg = self.send_queue.borrow_mut().pop_front().unwrap();
        assert_eq!(msg.send_to, node_id);
        if let RaftRPC::RequestVote(_) = msg.rpc {
            return;
        }
        panic!("Bad message type");
    }

    pub fn expect_request_append_entries(&self, node_id: u32) {
        let msg = self.send_queue.borrow_mut().pop_front().unwrap();
        assert_eq!(msg.send_to, node_id);
        if let RaftRPC::AppendEntries = msg.rpc {
            return;
        }
        panic!("Bad message type");
    }

    pub fn expect_vote(&self, node_id: u32) {
        let msg = self.send_queue.borrow_mut().pop_front().unwrap();
        assert_eq!(msg.send_to, node_id);
        if let RaftRPC::Vote = msg.rpc {
            return;
        }
        panic!("Bad message type")
    }

    pub fn send_to(&self, message: IncomingRaftMessage<u32>) {
        self.recv_queue.borrow_mut().push_back(message);
    }
}

impl Transport<u32> for MockTransport {
    fn append_entries(&self, node_id: u32) {
        let mut queue = self.send_queue.borrow_mut();
        queue.push_back(OutgoingRaftMessage {
            send_to: node_id,
            rpc: RaftRPC::AppendEntries,
        });
    }

    fn request_vote(&self, node_id: u32, request_vote: RequestVote) {
        info!("reqeust vote message sent to {:?}, {:?}", node_id, request_vote);
        let mut queue = self.send_queue.borrow_mut();
        queue.push_back(OutgoingRaftMessage {
            send_to: node_id,
            rpc: RaftRPC::RequestVote(request_vote),
        });
    }

    fn vote(&self, node_id: u32) {
        info!("Vote message sent to {:?}", node_id);
        let mut queue = self.send_queue.borrow_mut();
        queue.push_back(OutgoingRaftMessage {
            send_to: node_id,
            rpc: RaftRPC::Vote,
        });
    }

    fn read_msg(&self) -> Option<IncomingRaftMessage<u32>> {
        let mut queue = self.recv_queue.borrow_mut();
        return queue.pop_front();
    }
}
