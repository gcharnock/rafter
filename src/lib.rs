#[macro_use]
extern crate log;

use crate::time_oracle::TimeOracle;
use std::sync::{Arc, RwLock};
use crate::transport::{Transport, RequestVote, RaftRPC, IncomingRaftMessage, OutgoingRaftMessage, AppendEntriesResponse, AppendEntries, RequestVoteResponse};
use crate::config::RaftConfig;
use std::rc::Rc;
use std::fmt::Debug;
use crate::RaftStatus::Leader;

pub mod config;
pub mod time_oracle;
pub mod transport;

#[cfg(test)]
#[macro_use]
extern crate lazy_static;

#[cfg(test)]
mod test;


#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum RaftStatus {
    Leader,
    Follower,
    Candidate(u32),
}

pub struct RaftState {
    status: RaftStatus,
    has_voted_this_term: bool,
    term_number: u64,
}

pub struct Raft<'s, NodeId> {
    node_id: NodeId,
    state: Arc<RwLock<RaftState>>,
    time_oracle: Rc<dyn TimeOracle<'s>>,
    raft_config: RaftConfig<NodeId>,
    transport: Rc<dyn Transport<NodeId>>,
}

impl<'s, NodeId: Copy + Debug + 's> Raft<'s, NodeId> {
    pub fn new(node_id: NodeId,
               raft_config: RaftConfig<NodeId>,
               time_oracle: Rc<dyn TimeOracle<'s>>,
               transport: Rc<dyn Transport<NodeId>>)
               -> Raft<'s, NodeId> {
        Raft {
            node_id,
            state: Arc::new(RwLock::new(RaftState {
                status: RaftStatus::Follower,
                has_voted_this_term: false,
                term_number: 0,
            })),
            time_oracle,
            raft_config,
            transport,
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
        if self.state.read().unwrap().status == Leader {
            for peer_id in self.raft_config.get_peer_ids().iter() {
                debug!("sending heartbeat to {:?}", peer_id);
                self.append_entries(*peer_id);
            }
        } else {
            {
                let mut state = self.state.write().unwrap();
                state.status = RaftStatus::Candidate(0);
                state.term_number += 1;
            }
            for peer in self.raft_config.get_peer_ids().iter() {
                self.request_vote(*peer, RequestVote {});
            }
        }
    }

    fn receive_rec(&self, message: IncomingRaftMessage<NodeId>) {
        let self_term_number = self.state.read().unwrap().term_number;
        if message.term > self_term_number {
            debug!("updating term number {} -> {}", self_term_number, message.term);
            self.state.write().unwrap().term_number = message.term;
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
                let vote_granted = message.term >= self_term_number;
                info!("Got vote request from {:?}, vote granted = {}", message.recv_from, vote_granted);
                self.vote(message.recv_from, vote_granted);
            }
            RaftRPC::RequestVoteResponse(response) => {
                if response.vote_granted {
                    self.state.write().unwrap().status = Leader;
                    for node_id in self.raft_config.get_peer_ids().iter() {
                        self.append_entries(*node_id);
                    }
                }
            }
        }
    }

    fn append_entries(&self, node_id: NodeId) {
        self.transport.send_msg(OutgoingRaftMessage {
            send_to: node_id,
            term: self.state.read().unwrap().term_number,
            rpc: RaftRPC::AppendEntries(AppendEntries {}),
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




