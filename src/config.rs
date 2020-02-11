use std::time::Duration;

pub struct RaftConfig<NodeId> {
    min_election_timeout: Duration,
    max_election_timeout: Duration,
    peer_ids: Vec<NodeId>,
}

impl<NodeId> RaftConfig<NodeId> {
    pub fn new(peer_ids: Vec<NodeId>,
               min_election_timeout: Duration,
               max_election_timeout: Duration) -> Self {
        if min_election_timeout > max_election_timeout {
            panic!("min_electrion_timeout should be smaller than max_election_timeout");
        }
        Self {
            peer_ids,
            min_election_timeout,
            max_election_timeout,
        }
    }

    pub fn get_min_election_timeout(&self) -> Duration {
        self.min_election_timeout
    }

    pub fn get_max_election_timeout(&self) -> Duration {
        self.max_election_timeout
    }

    pub fn get_peer_ids(&self) -> &Vec<NodeId> {
        return &self.peer_ids;
    }
}
