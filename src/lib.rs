#[macro_use]
extern crate log;

use crate::time_oracle::TimeOracle;
use std::sync::{Arc, RwLock};
use crate::transport::{Transport, RequestVote, RaftRPC, IncomingRaftMessage, OutgoingRaftMessage, AppendEntriesResponse, AppendEntries, RequestVoteResponse};
use crate::config::RaftConfig;
use std::rc::Rc;
use std::fmt::Debug;
use crate::RaftStatus::{Leader, Candidate, Follower};
use std::collections::{HashMap, VecDeque};

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
    term_number: u64,
    commit_index: u64,
    last_applied: u64,
}

pub trait StateMachine<Log> {
    fn apply(&mut self, log: Log);
}

pub struct Raft<'s, NodeId, Log, SM>
    where SM: StateMachine<Log>,
          Log: 's,
          NodeId: Copy + Debug + std::hash::Hash + Eq + 's {
    node_id: NodeId,
    state: Arc<RwLock<RaftState<NodeId>>>,
    time_oracle: Rc<dyn TimeOracle<'s>>,
    raft_config: RaftConfig<NodeId>,
    transport: Rc<dyn Transport<NodeId, Log>>,
    log: VecDeque<Log>,
    state_machine: SM,
}

impl<'s, NodeId, Log, SM> Raft<'s, NodeId, Log, SM>
    where NodeId: Copy + Debug + std::hash::Hash + Eq + 's,
          SM: StateMachine<Log> + 's {
    pub fn new(node_id: NodeId,
               raft_config: RaftConfig<NodeId>,
               time_oracle: Rc<dyn TimeOracle<'s>>,
               transport: Rc<dyn Transport<NodeId, Log>>,
               state_machine: SM,
    )
               -> Self {
        Self {
            node_id,
            state: Arc::new(RwLock::new(RaftState {
                status: RaftStatus::Follower,
                voted_for: None,
                term_number: 0,
                commit_index: 0,
                last_applied: 0,
            })),
            time_oracle,
            raft_config,
            transport,
            log: VecDeque::new(),
            state_machine,
        }
    }

    pub fn start(this: Rc<Self>) {
        info!("Started a Raft instance");
        let that = this.clone();
        let callback = move || that.election_expire();
        let callback_box = Box::new(callback);
        let election_timeout = this.time_oracle.get_random_duration(
            this.raft_config.get_min_election_timeout(),
            this.raft_config.get_max_election_timeout());
        this.time_oracle.set_timer(election_timeout, callback_box);
    }

    pub fn loop_iter(&self) {
        debug!("running raft logic");
        while let Some(msg) = self.transport.read_msg() {
            debug!("got message");
            self.receive_rec(msg);
        }
    }

    fn election_expire(&self) {
        debug!("election_expire");
        if self.state.read().unwrap().status.is_leader() {
            for peer_id in self.raft_config.get_peer_ids().iter() {
                debug!("sending heartbeat to {:?}", peer_id);
                self.append_entries(*peer_id);
            }
        } else {
            {
                let mut state = self.state.write().unwrap();
                // Vote for self
                state.status = RaftStatus::Candidate(1);
                state.term_number += 1;
            }
            for peer in self.raft_config.get_peer_ids().iter() {
                self.request_vote(*peer, RequestVote {});
            }
        }
    }

    fn receive_rec(&self, message: IncomingRaftMessage<NodeId, Log>) {
        let self_term_number = self.state.read().unwrap().term_number;
        if message.term > self_term_number {
            debug!("updating term number {} -> {}", self_term_number, message.term);
            let mut state = self.state.write().unwrap();
            state.term_number = message.term;
            state.voted_for = None;
        }

        match message.rpc {
            RaftRPC::AppendEntries(_) => {
                debug!("AppendEntries");
                self.time_oracle.reset_timer();
                self.append_entries_response(message.recv_from, true);
            }
            RaftRPC::AppendEntriesResponse(_) => {
                unimplemented!()
            }
            RaftRPC::RequestVote(request) => {
                let correct_term = message.term >= self_term_number;
                let has_vote_left = self.state.read().unwrap().voted_for.is_none();
                let vote_granted = correct_term && has_vote_left;
                if vote_granted {
                    self.state.write().unwrap().voted_for = Some(message.recv_from);
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

    fn on_request_vote_response(&self, response: RequestVoteResponse) {
        let mut state = self.state.write().unwrap();
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
                        (*state).status = Leader(LeaderState::new());
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
        self.transport.send_msg(OutgoingRaftMessage {
            send_to: node_id,
            term: self.state.read().unwrap().term_number,
            rpc: RaftRPC::AppendEntries(AppendEntries {
                prev_log_index: 0,
                prev_log_term: 0,
                entries: vec![],
                leader_commit: 0
            }),
        });
    }

    fn request_vote(&self, send_to: NodeId, request_vote: RequestVote) {
        self.transport.send_msg(OutgoingRaftMessage {
            send_to,
            term: self.state.read().unwrap().term_number,
            rpc: RaftRPC::RequestVote(request_vote),
        });
    }

    fn vote(&self, send_to: NodeId, vote_granted: bool) {
        self.transport.send_msg(OutgoingRaftMessage {
            send_to,
            term: self.state.read().unwrap().term_number,
            rpc: RaftRPC::RequestVoteResponse(RequestVoteResponse {
                vote_granted,
            }),
        });
    }

    fn append_entries_response(&self, send_to: NodeId, success: bool) {
        self.transport.send_msg(OutgoingRaftMessage {
            send_to,
            term: self.state.read().unwrap().term_number,
            rpc: RaftRPC::AppendEntriesResponse(AppendEntriesResponse {
                success,
            }),
        });
    }
}




