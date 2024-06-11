use crate::raikou::sim_types;

// To avoid having numerous generic parameters while still maintaining some flexibility,
// a bunch of type aliases are used.

// Common types:

pub type BatchHash = HashValue;
pub type Round = i64; // Round number.
pub type Prefix = usize;

// Aptos types and functions:
//
// pub type HashValue = aptos_crypto::hash::HashValue;
// pub type BatchInfo = aptos_consensus_types::proof_of_store::BatchInfo;
// pub type AC = aptos_consensus_types::proof_of_store::ProofOfStore;
// pub type BlockPayload = ...

// Simulator types and functions:

pub use sim_types::hash;
pub type Txn = ();
pub type BatchId = sim_types::BatchId;
pub type HashValue = sim_types::HashValue;
pub type BatchInfo = sim_types::BatchInfo;
pub type AC = sim_types::AC;
pub type Payload = sim_types::Payload;
