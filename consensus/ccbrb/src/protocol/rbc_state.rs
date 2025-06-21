use crypto::hash::Hash;
use reed_solomon_rs::fec::fec::*;
use std::collections::{HashMap, HashSet};

#[derive(PartialEq, Debug)]
pub enum Status {
    WAITING,
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

    // CCBRB specific fields
    pub fragments_data: HashMap<(u64, Hash), Vec<Share>>,
    pub fragments_hashes: HashMap<(u64, Hash), Vec<Vec<u8>>>,
    pub e: usize,
    pub sent_ready: HashSet<(u64, Hash, Vec<u8>)>,
    pub sent_echo: HashSet<(u64, Hash, Vec<u8>)>, // optional
}

impl RBCState {
    pub fn new() -> Self {
        Self {
            received_echo_count: HashMap::default(),
            received_readys: HashMap::default(),
            echo_senders: HashMap::default(),
            ready_senders: HashMap::default(),
            fragment: Share {
                number: 0,
                data: vec![],
            },
            output_message: vec![],
            status: Status::WAITING,

            fragments_data: HashMap::default(),
            fragments_hashes: HashMap::default(),
            e: 0,
            sent_ready: HashSet::default(),
            sent_echo: HashSet::default(),
        }
    }

    pub fn get_max_echo_count(&self) -> (usize, Option<Hash>) {
        let mut mode_content: Option<Hash> = None;
        let mut max_count = 0;
        for (content, &count) in self.received_echo_count.iter() {
            if count > max_count {
                max_count = count;
                mode_content = Some(content.clone());
            }
        }
        (max_count, mode_content)
    }

    pub fn get_max_ready_count(&self) -> (usize, Option<Hash>) {
        let mut mode_content: Option<Hash> = None;
        let mut max_count = 0;
        for (content, count) in self.received_readys.iter() {
            if count.len() > max_count {
                max_count = count.len();
                mode_content = Some(content.clone());
            }
        }
        (max_count, mode_content)
    }
}

impl Default for RBCState {
    fn default() -> Self {
        Self::new()
    }
}
