pub trait Transport<NodeId, Log> {
    fn send_msg(&self, msg: OutgoingRaftMessage<NodeId, Log>);
    fn read_msg(&self) -> Option<IncomingRaftMessage<NodeId, Log>>;
}

#[derive(Debug)]
pub struct RequestVote {
}

#[derive(Debug)]
pub struct RequestVoteResponse {
    pub vote_granted: bool
}


#[derive(Debug)]
pub struct AppendEntries<Log> {
    pub prev_log_index: u64,
    pub prev_log_term: u64,
    pub entries: Vec<Log>,
    pub leader_commit: u64
}

#[derive(Debug)]
pub struct AppendEntriesResponse {
    pub success: bool
}

#[derive(Debug)]
pub enum RaftRPC<Log> {
    AppendEntries(AppendEntries<Log>),
    AppendEntriesResponse(AppendEntriesResponse),
    RequestVote(RequestVote),
    RequestVoteResponse(RequestVoteResponse),
}

#[derive(Debug)]
pub struct OutgoingRaftMessage<NodeId, Log> {
    pub rpc: RaftRPC<Log>,
    pub term: u64,
    pub send_to: NodeId,
}

#[derive(Debug)]
pub struct IncomingRaftMessage<NodeId, Log> {
    pub rpc: RaftRPC<Log>,
    pub term: u64,
    pub recv_from: NodeId,
}

