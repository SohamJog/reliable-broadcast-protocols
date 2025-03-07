use crypto::hash::Hash;
use reed_solomon_rs::fec::fec::*;
use std::collections::{HashMap, HashSet};

// Node cannot send ready before Echo
#[derive(PartialEq)]
pub enum Status {
    INIT,
    ECHO,
    READY,
    OUTPUT,
    TERMINATED,
}
pub struct RBCState {
    pub received_echo_count: HashMap<Hash, usize>,
    pub received_readys: HashMap<Hash, Vec<Share>>,
    pub echo_senders: HashMap<Hash, HashSet<usize>>,
    pub ready_senders: HashMap<Hash, HashSet<usize>>,
    pub fragment: Share,
    pub output_message: Vec<u8>,
    pub status: Status,
}

impl RBCState {
    pub fn new() -> RBCState {
        RBCState {
            received_echo_count: HashMap::default(),
            received_readys: HashMap::default(),
            echo_senders: HashMap::default(),
            ready_senders: HashMap::default(),
            fragment: Share {
                number: 0,
                data: vec![],
            },
            output_message: vec![],
            status: Status::INIT,
        }
    }
}

impl Default for RBCState {
    fn default() -> Self {
        Self::new()
    }
}
