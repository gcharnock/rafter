pub trait Transport<NodeId> {
    fn send_msg(&self, msg: OutgoingRaftMessage<NodeId>);
    fn read_msg(&self) -> Option<IncomingRaftMessage<NodeId>>;
}

#[derive(Debug)]
pub struct RequestVote {
    pub term: u64,
}

#[derive(Debug)]
pub struct RequestVoteResponse {
    pub term: u64,
    pub vote_grated: bool
}


#[derive(Debug)]
pub struct AppendEntries {
    pub term: u64,
}

#[derive(Debug)]
pub struct AppendEntriesResponse {
    pub term: u64,
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
    pub send_to: NodeId,
}

#[derive(Debug)]
pub struct IncomingRaftMessage<NodeId> {
    pub rpc: RaftRPC,
    pub recv_from: NodeId,
}

