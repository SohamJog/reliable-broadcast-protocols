use std::collections::HashMap;

use crypto::{aes_hash::{Proof}, hash::Hash};

pub struct RBCState{
    pub echos: HashMap<Hash, HashMap<usize,Vec<u8>>>,
    pub echo_root: Option<Hash>,

    pub readys: HashMap<Hash, HashMap<usize,Vec<u8>>>,


    pub votes: HashMap<Hash, HashMap<usize,Vec<u8>>>, 
    pub sent_vote: bool,       
    pub sent_ready: bool,      
    pub ready_quorum_reached: bool,     // 2f+1 

    
    pub fragment: Option<(Vec<u8>, Proof)>,
    pub message: Option<Vec<u8>>,

    pub terminated: bool
}

impl RBCState{
    
    pub fn new()-> RBCState{
        RBCState { 
            echos: HashMap::default(), 
            echo_root: None, 
            
            readys: HashMap::default(), 
            votes: HashMap::default(),
            sent_vote: false,
            sent_ready: false,
            ready_quorum_reached: false,
            
            fragment: None, 
            message: None,

            terminated: false
        }
    }
}

impl Default for RBCState {
    fn default() -> Self {
        Self::new()
    }
}