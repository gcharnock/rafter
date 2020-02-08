#[macro_use]
extern crate log;

use crate::time_oracle::{TimeOracle};
use std::sync::{Arc, RwLock};
use crate::transport::{Transport, RequestVote, RaftRPC, IncomingRaftMessage};
use crate::config::RaftConfig;
use std::rc::Rc;

pub mod config;
pub mod time_oracle;
pub mod transport;
pub mod tests;


#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum RaftStatus {
    Leader,
    Follower,
    Candidate,
}

pub struct RaftState {
    status: RaftStatus,
    has_voted_this_term: bool,
    term_number: u64
}

pub struct Raft<'s, NodeId> {
    node_id: NodeId,
    state: Arc<RwLock<RaftState>>,
    time_oracle: Rc<dyn TimeOracle<'s>>,
    raft_config: RaftConfig<NodeId>,
    transport: Rc<dyn Transport<NodeId>>,
}

impl<'s, NodeId: Copy + 's> Raft<'s, NodeId> {
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
        let mut state = self.state.write().unwrap();
        state.status = RaftStatus::Candidate;
        state.term_number += 1;
        for peer in self.raft_config.get_peer_ids().iter() {
            self.transport.request_vote(*peer, RequestVote {
                term: state.term_number
            });
        }
    }

    fn receive_rec(&self, message: IncomingRaftMessage<NodeId>) {
        match message.rpc {
            RaftRPC::AppendEntries => {
                debug!("AppendEntries");
                self.time_oracle.reset_timer();
            },
            RaftRPC::RequestVote(request) => {
                info!("Got vote request, will  vote for the candidate");
                self.transport.vote(message.recv_from);
            },
            RaftRPC::Vote => {

            },
        }
    }
}

#[cfg(test)]
mod logging_setup {
    extern crate env_logger;

    use log::LevelFilter::Trace;
    use std::io::Write;

    pub fn start_logger() {
        let _ = env_logger::builder()
            .format(|buf, record|
                writeln!(buf, "{}:{} {} - {}",
                         record.file().unwrap_or("<UNKNOWN>"),
                         record.line().unwrap_or(0),
                         record.level(),
                         record.args()))
            .filter_level(Trace)
            .is_test(true)
            .try_init();
    }
}


#[cfg(test)]
#[macro_use]
extern crate lazy_static;

