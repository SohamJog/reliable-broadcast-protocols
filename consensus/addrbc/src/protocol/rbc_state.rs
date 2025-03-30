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
    /*

    fn get_max_echo_count(
        rbc_context: &mut crate::RBCContext,
        instance_id: usize,
    ) -> (usize, Option<Hash>) {
        let mut mode_content: Option<Hash> = None;
        let mut max_count = 0;

        for (content, &count) in rbc_context.received_echo_count.iter() {
            if count > max_count {
                max_count = count;
                mode_content = Some(content.clone());
            }
        }
        (max_count, mode_content)
    }
     */

    pub fn get_max_echo_count (&self) -> (usize, Option<Hash>) {
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
    pub fn get_max_ready_count (&self) -> (usize, Option<Hash>) {
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
