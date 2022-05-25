// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{proof::SparseMerkleRangeProof, state_store::state_key::StateKey};
use aptos_crypto::{
    hash::{CryptoHash, CryptoHasher, SPARSE_MERKLE_PLACEHOLDER_HASH},
    HashValue,
};
use aptos_crypto_derive::CryptoHasher;
use serde::{Deserialize, Serialize};

#[derive(
    Clone,
    Debug,
    Default,
    CryptoHasher,
    Eq,
    PartialEq,
    Serialize,
    Deserialize,
    Ord,
    PartialOrd,
    Hash,
)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
pub struct StateValue {
    pub maybe_bytes: Option<Vec<u8>>,
    hash: HashValue,
}

impl StateValue {
    fn new(maybe_bytes: Option<Vec<u8>>) -> Self {
        let mut hasher = StateValueHasher::default();
        let hash = if let Some(bytes) = &maybe_bytes {
            hasher.update(bytes);
            hasher.finish()
        } else {
            HashValue::zero()
        };
        Self { maybe_bytes, hash }
    }

    pub fn empty() -> Self {
        StateValue::new(None)
    }
}

impl From<Vec<u8>> for StateValue {
    fn from(bytes: Vec<u8>) -> Self {
        StateValue::new(Some(bytes))
    }
}

impl CryptoHash for StateValue {
    type Hasher = StateValueHasher;

    fn hash(&self) -> HashValue {
        self.hash
    }
}

/// TODO(joshlind): add a proof implementation (e.g., verify()) and unit tests
/// for these once we start supporting them.
///
/// A single chunk of all state values at a specific version.
/// Note: this is similar to `StateSnapshotChunk` but all data is included
/// in the struct itself and not behind pointers/handles to file locations.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
pub struct StateValueChunkWithProof {
    pub first_index: u64,     // The first hashed state index in chunk
    pub last_index: u64,      // The last hashed state index in chunk
    pub first_key: HashValue, // The first hashed state key in chunk
    pub last_key: HashValue,  // The last hashed state key in chunk
    pub raw_values: Vec<(StateKey, StateValue)>, // The hashed state key and and raw state value.
    pub proof: SparseMerkleRangeProof, // The proof to ensure the chunk is in the hashed states
    pub root_hash: HashValue, // The root hash of the sparse merkle tree for this chunk
}

impl StateValueChunkWithProof {
    /// Returns true iff this chunk is the last chunk (i.e., there are no
    /// more state values to write to storage after this chunk).
    pub fn is_last_chunk(&self) -> bool {
        let right_siblings = self.proof.right_siblings();
        right_siblings
            .iter()
            .all(|sibling| *sibling == *SPARSE_MERKLE_PLACEHOLDER_HASH)
    }
}

#[cfg(test)]
mod tests {
    use crate::state_store::state_value::StateValue;

    #[test]
    fn test_empty_state_value() {
        StateValue::new(None);
    }
}
