use std::cell::{RefCell, Cell};
use std::collections::VecDeque;
use crate::transport::{OutgoingRaftMessage, RaftRPC, AppendEntriesResponse, RequestVote, AppendEntries, RequestVoteResponse};
use crate::{RaftIO, ClientResponse};
use crate::test::{TestLog, TestId};
use std::time::Duration;

pub struct MockRaftIO {
    send_queue: RefCell<VecDeque<OutgoingRaftMessage<TestId, TestLog>>>,
    committed: Vec<TestLog>,
    client_responses: Vec<ClientResponse<TestId>>,
    last_reset: Cell<Option<Duration>>
}

impl MockRaftIO {
    pub fn new() -> Self {
        Self {
            send_queue: RefCell::new(VecDeque::new()),
            committed: Vec::new(),
            client_responses: Vec::new(),
            last_reset: Cell::new(None)
        }
    }

    pub fn expect_reset(&mut self) -> Duration {
        let delay = self.last_reset.replace(None);
        delay.expect("no reset")
    }

    fn apply_to_state_machine(&mut self, log: u32) {
        self.committed.push(log);
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
}

impl<'s> RaftIO<TestId, TestLog> for MockRaftIO {
    fn send_client_response(&mut self, response: ClientResponse<u32>) {
        self.client_responses.push(response);
    }

    fn apply_to_state_machine(&mut self, log: u32) {
        self.committed.push(log);
    }

    fn send_msg(&self, msg: OutgoingRaftMessage<TestId, TestLog>) {
        self.send_queue.borrow_mut().push_back(msg);
    }

    fn reset_timer(&self, delay: Duration) {
        self.last_reset.set(Some(delay));
    }
}
