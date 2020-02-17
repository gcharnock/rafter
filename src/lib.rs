#![allow(unused_variables)]

#[macro_use]
extern crate log;


use crate::transport::{RequestVote, RaftRPC, IncomingRaftMessage, OutgoingRaftMessage, AppendEntriesResponse, AppendEntries, RequestVoteResponse, ClientRequest};
use crate::config::RaftConfig;
use std::fmt::Debug;
use crate::RaftStatus::{Leader, Candidate, Follower};
use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::time::Duration;

pub mod config;
pub mod time_oracle;
pub mod transport;

#[cfg(test)]
#[macro_use]
extern crate lazy_static;

#[cfg(test)]
mod test;

fn quorum_size(group_size: usize) -> usize {
    return group_size / 2 + 1;
}

#[cfg(test)]
mod quorum_test {
    use crate::quorum_size;

    #[test]
    fn test_quorum_size() {
        assert_eq!(quorum_size(1), 1);
        assert_eq!(quorum_size(2), 2);
        assert_eq!(quorum_size(3), 2);
        assert_eq!(quorum_size(4), 3);
        assert_eq!(quorum_size(5), 3);
        assert_eq!(quorum_size(6), 4);
        assert_eq!(quorum_size(7), 4);
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct FollowerModel {
    next_index: u64,
    match_index: u64,
}

pub struct LeaderState<NodeId>
    where NodeId: std::hash::Hash {
    follower_models: HashMap<NodeId, FollowerModel>,
}

impl<NodeId> LeaderState<NodeId>
    where NodeId: std::hash::Hash + Eq {
    fn new() -> Self {
        Self {
            follower_models: HashMap::new()
        }
    }
}

pub enum RaftStatus<NodeId>
    where NodeId: std::hash::Hash + Eq {
    Leader(LeaderState<NodeId>),
    Follower,
    Candidate(u32),
}

impl<NodeId> RaftStatus<NodeId>
    where NodeId: std::hash::Hash + Eq {
    pub fn is_leader(&self) -> bool {
        if let Leader(_) = self {
            return true;
        }
        false
    }

    pub fn is_candidate(&self) -> bool {
        if let Candidate(_) = self {
            return true;
        }
        false
    }

    pub fn is_follower(&self) -> bool {
        if let Follower = self {
            return true;
        }
        false
    }
}

pub struct RaftState<NodeId>
    where NodeId: std::hash::Hash + Eq {
    status: RaftStatus<NodeId>,
    voted_for: Option<NodeId>,
    term: u64,
    commit_index: u64,
    last_applied: u64,
}

pub trait StateMachine<Log> {}

pub enum ClientResponse<NodeId> {
    Success,
    RedirectTo(NodeId),
}

pub trait RaftIO<NodeId, Log> {
    fn send_client_response(&mut self, response: ClientResponse<NodeId>);
    fn apply_to_state_machine(&mut self, log: Log);
    fn send_msg(&self, msg: OutgoingRaftMessage<NodeId, Log>);
    fn reset_timer(&self, delay: Duration);
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub struct LogEntry<Cmd> {
    pub term: u64,
    pub command: Cmd,
}

pub struct Raft<NodeId, Cmd, IO>
    where NodeId: Eq + Hash {
    node_id: NodeId,
    state: RaftState<NodeId>,
    raft_config: RaftConfig<NodeId>,
    log: VecDeque<LogEntry<Cmd>>,
    raft_io: IO,
    election_timeout: Duration,
}

impl<NodeId, Cmd, IO> Raft<NodeId, Cmd, IO>
    where NodeId: Copy + Debug + Hash + Eq,
          IO: RaftIO<NodeId, Cmd>,
          Cmd: Debug {
    pub fn new(node_id: NodeId,
               raft_config: RaftConfig<NodeId>,
               raft_io: IO,
    )
               -> Self {
        Self {
            node_id,
            state: RaftState {
                status: RaftStatus::Follower,
                voted_for: None,
                term: 0,
                commit_index: 0,
                last_applied: 0,
            },
            raft_config,
            log: VecDeque::new(),
            raft_io,
            election_timeout: Duration::new(1, 0),
        }
    }

    pub fn on_election_expire(&mut self) {
        debug!("election_expire");
        if self.state.status.is_leader() {
            for peer_id in self.raft_config.get_peer_ids().iter() {
                debug!("sending heartbeat to {:?}", peer_id);
                self.append_entries(*peer_id);
            }
        } else {
            {
                // Vote for self
                self.state.status = RaftStatus::Candidate(1);
                self.state.term += 1;
            }
            for peer in self.raft_config.get_peer_ids().iter() {
                self.request_vote(*peer, RequestVote {});
            }
        }
    }

    fn become_leader(&mut self) {
        debug!("Becoming leader");
        let mut leader_state = LeaderState::new();
        for node_id in self.raft_config.get_peer_ids().iter() {
            leader_state.follower_models.insert(*node_id, FollowerModel {
                match_index: 0,
                next_index: 0,
            });
        };
        self.state.status = RaftStatus::Leader(leader_state);
        self.on_leader_timeout();
        self.raft_io.reset_timer(self.raft_config.get_min_election_timeout() / 2);
    }

    pub fn on_leader_timeout(&mut self) {
        debug!("leader expire");
        if let Leader(leader_state) = &self.state.status {
            for (follower_id, model) in leader_state.follower_models.iter() {
                self.append_entries(*follower_id);
            }
        } else {
            panic!("not leader");
        }
    }

    pub fn on_client_request(&mut self, client_request: ClientRequest<Cmd>) {}

    pub fn on_raft_message(&mut self, message: IncomingRaftMessage<NodeId, Cmd>) {
        if message.term > self.state.term {
            debug!("updating term number {} -> {}", self.state.term, message.term);
            self.state.term = message.term;
            self.state.voted_for = None;
        } else if message.term < self.state.term {
            match message.rpc {
                RaftRPC::AppendEntries(_) => {
                    self.append_entries_response(message.recv_from, false);
                }
                RaftRPC::RequestVote(_) => {
                    self.vote(message.recv_from, false);
                }
                _ => ()
            }
            return;
        }

        match message.rpc {
            RaftRPC::AppendEntries(mut rpc) =>
                self.on_append_entries(message.recv_from, message.term, rpc),
            RaftRPC::AppendEntriesResponse(_) => unimplemented!(),
            RaftRPC::RequestVote(request) => {
                let correct_term = message.term >= self.state.term;
                let has_vote_left = self.state.voted_for.is_none();
                let vote_granted = correct_term && has_vote_left;
                if vote_granted {
                    self.state.voted_for = Some(message.recv_from);
                }
                info!("Got vote request from {:?} vote_grated={}, correct_term={}, has_vote_left={}",
                      message.recv_from, vote_granted, correct_term, has_vote_left);
                self.vote(message.recv_from, vote_granted);
            }
            RaftRPC::RequestVoteResponse(response) => {
                self.on_request_vote_response(response);
            }
        }
    }

    fn on_append_entries(&mut self, recv_from: NodeId, term: u64, rpc: AppendEntries<Cmd>) {
        debug!("AppendEntries");
        self.raft_io.reset_timer(self.election_timeout);


        // commit index conflict
        while self.state.commit_index > rpc.prev_log_index {
            self.log.pop_front();
            self.state.commit_index -= 1;
        }

        let term = self.state.term;
        for (i, entry) in rpc.entries.into_iter().enumerate() {
            self.append_entry(rpc.prev_log_index as usize + i, LogEntry {
                command: entry,
                term
            });
        }

        self.append_entries_response(recv_from, true);
    }

    fn append_entry(&mut self, global_index: usize, entry: LogEntry<Cmd>) {
        debug!("Append entry, {}, {:?}", global_index, entry);
        if let Some(log_at) = self.log.get(global_index) {
            debug!("log_at.term = {}, entry.term= {}", log_at.term, entry.term);
            if log_at.term != entry.term {
                self.log.truncate(global_index);
            } else {
                unreachable!();
            }
        }
        self.log.push_back(entry);
    }

    fn on_request_vote_response(&mut self, response: RequestVoteResponse) {
        let mut state = &mut self.state;
        match &mut state.status {
            RaftStatus::Leader(_) => { debug!("Ignoring spurious vote response "); }
            RaftStatus::Follower => { debug!("Ignoring spurious vote response ") }
            RaftStatus::Candidate(ref mut vote_count) => {
                if response.vote_granted {
                    *vote_count += 1;
                    let target_votes =
                        quorum_size(self.raft_config.get_peer_count()) as u32;
                    debug!(" received vote as candidate. Vote count is {}/{} ", *vote_count, target_votes);
                    if *vote_count >= target_votes {
                        info!(" Election won, becoming leader ");
                        state.status = Leader(LeaderState::new());
                        drop(state);
                        for node_id in self.raft_config.get_peer_ids().iter() {
                            self.append_entries(*node_id);
                        }
                    }
                }
            }
        }
    }

    fn append_entries(&self, node_id: NodeId) {
        self.raft_io.send_msg(OutgoingRaftMessage {
            send_to: node_id,
            term: self.state.term,
            rpc: RaftRPC::AppendEntries(AppendEntries {
                prev_log_index: 0,
                prev_log_term: 0,
                entries: vec![],
                leader_commit: 0,
            }),
        });
    }

    fn request_vote(&self, send_to: NodeId, request_vote: RequestVote) {
        self.raft_io.send_msg(OutgoingRaftMessage {
            send_to,
            term: self.state.term,
            rpc: RaftRPC::RequestVote(request_vote),
        });
    }

    fn vote(&self, send_to: NodeId, vote_granted: bool) {
        self.raft_io.send_msg(OutgoingRaftMessage {
            send_to,
            term: self.state.term,
            rpc: RaftRPC::RequestVoteResponse(RequestVoteResponse {
                vote_granted,
            }),
        });
    }

    fn append_entries_response(&self, send_to: NodeId, success: bool) {
        self.raft_io.send_msg(OutgoingRaftMessage {
            send_to,
            term: self.state.term,
            rpc: RaftRPC::AppendEntriesResponse(AppendEntriesResponse {
                success,
            }),
        });
    }
}




