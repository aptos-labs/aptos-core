use crate::raikou::sim_types;

// To avoid having numerous generic parameters while still having some agility,
// a bunch of type aliases are used.

// Common types:

pub type Round = i64; // Round number.
pub type BatchSN = i64; // Sequence number of a batch.
pub type Prefix = usize;

// Aptos types:
//
// pub type HashValue = aptos_crypto::hash::HashValue;
// pub type BatchInfo = aptos_consensus_types::proof_of_store::BatchInfo;
// pub type AC = aptos_consensus_types::proof_of_store::ProofOfStore;
// pub type BlockPayload = ...

// Simulator types:

pub type HashValue = u64;
pub type Txn = ();
pub type BatchInfo = sim_types::BatchInfo;
pub type AC = sim_types::AC;
pub type Payload = sim_types::BlockPayload;
