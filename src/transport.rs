pub trait Transport<NodeId> {
    fn send_msg(&self, msg: OutgoingRaftMessage<NodeId>);
    fn read_msg(&self) -> Option<IncomingRaftMessage<NodeId>>;
}

#[derive(Debug)]
pub struct RequestVote {
}

#[derive(Debug)]
pub struct RequestVoteResponse {
    pub vote_granted: bool
}


#[derive(Debug)]
pub struct AppendEntries {
}

#[derive(Debug)]
pub struct AppendEntriesResponse {
    pub success: bool
}

#[derive(Debug)]
pub enum RaftRPC {
    AppendEntries(AppendEntries),
    AppendEntriesResponse(AppendEntriesResponse),
    RequestVote(RequestVote),
    RequestVoteResponse(RequestVoteResponse),
}

#[derive(Debug)]
pub struct OutgoingRaftMessage<NodeId> {
    pub rpc: RaftRPC,
    pub term: u64,
    pub send_to: NodeId,
}

#[derive(Debug)]
pub struct IncomingRaftMessage<NodeId> {
    pub rpc: RaftRPC,
    pub term: u64,
    pub recv_from: NodeId,
}

