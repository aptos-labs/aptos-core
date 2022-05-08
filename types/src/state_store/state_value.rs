// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    ledger_info::LedgerInfo,
    proof::{SparseMerkleRangeProof, StateStoreValueProof},
    state_store::state_key::StateKey,
    transaction::Version,
};
use anyhow::ensure;
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
    pub first_index: u64,     // The first account index in chunk
    pub last_index: u64,      // The last account index in chunk
    pub first_key: HashValue, // The first account key in chunk
    pub last_key: HashValue,  // The last account key in chunk
    pub raw_values: Vec<(HashValue, StateKeyAndValue)>, // The account blobs in the chunk
    pub proof: SparseMerkleRangeProof, // The proof to ensure the chunk is in the account states
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
pub struct StateValueWithProof {
    /// The transaction version at which this account state is seen.
    pub version: Version,
    /// Value represents the value in state store. If this field is not set, it
    /// means the key does not exist.
    pub value: Option<StateValue>,
    /// The proof the client can use to authenticate the value.
    pub proof: StateStoreValueProof,
}

impl StateValueWithProof {
    /// Constructor.
    pub fn new(version: Version, value: Option<StateValue>, proof: StateStoreValueProof) -> Self {
        Self {
            version,
            value,
            proof,
        }
    }

    /// Verifies the state store value with the proof, both carried by `self`.
    ///
    /// Two things are ensured if no error is raised:
    ///   1. This value exists in the ledger represented by `ledger_info`.
    ///   2. It belongs to state_key and is seen at the time the transaction at version
    /// `state_version` is just committed. To make sure this is the latest state, pass in
    /// `ledger_info.version()` as `state_version`.
    pub fn verify(
        &self,
        ledger_info: &LedgerInfo,
        version: Version,
        state_key: StateKey,
    ) -> anyhow::Result<()> {
        ensure!(
            self.version == version,
            "State version ({}) is not expected ({}).",
            self.version,
            version,
        );

        self.proof
            .verify(ledger_info, version, state_key.hash(), self.value.as_ref())
    }
}

#[derive(
    Clone, Debug, CryptoHasher, Eq, PartialEq, Serialize, Deserialize, Ord, PartialOrd, Hash,
)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
pub struct StateKeyAndValue {
    pub key: StateKey,
    pub value: StateValue,
}

impl StateKeyAndValue {
    pub fn new(key: StateKey, value: StateValue) -> Self {
        Self { key, value }
    }
}

impl CryptoHash for StateKeyAndValue {
    type Hasher = StateKeyAndValueHasher;

    fn hash(&self) -> HashValue {
        self.value.hash
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
