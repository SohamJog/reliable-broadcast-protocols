use std::fmt::Debug;

use crypto::hash::Hash;
use reed_solomon_rs::fec::fec::*;
use serde::{Deserialize, Serialize};
use types::Replica;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SendMsg {
    pub id: u64,
    pub d_j: Share,
    pub d_hashes: Vec<Hash>, // D = [H(d₁),...,H(dₙ)]
    pub origin: Replica,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EchoMsg {
    pub id: u64,
    pub d_i: Share,
    pub pi_i: Share, // Proof pi[i]
    pub c: Hash,
    pub origin: Replica,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReadyMsg {
    pub id: u64,
    pub c: Hash,
    pub pi_i: Share,
    pub origin: Replica,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ProtMsg {
    Init(SendMsg, Replica),
    Echo(EchoMsg, Replica),
    Ready(ReadyMsg, Replica),
}
