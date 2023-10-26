// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    on_chain_config::CurrentTimeMicroseconds, proof::SparseMerkleRangeProof,
    state_store::state_key::StateKey, transaction::Version,
};
use aptos_crypto::{
    hash::{CryptoHash, SPARSE_MERKLE_PLACEHOLDER_HASH},
    HashValue,
};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use bytes::Bytes;
use once_cell::sync::OnceCell;
#[cfg(any(test, feature = "fuzzing"))]
use proptest::{arbitrary::Arbitrary, prelude::*};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
#[derive(
    BCSCryptoHash,
    Clone,
    CryptoHasher,
    Debug,
    Deserialize,
    Eq,
    PartialEq,
    Serialize,
    Ord,
    PartialOrd,
    Hash,
)]
pub enum StateValueMetadata {
    V0 {
        deposit: u64,
        creation_time_usecs: u64,
    },
}

// To avoid nested options when fetching a resource and its metadata.
pub type StateValueMetadataKind = Option<StateValueMetadata>;

impl StateValueMetadata {
    pub fn new(deposit: u64, creation_time_usecs: &CurrentTimeMicroseconds) -> Self {
        Self::V0 {
            deposit,
            creation_time_usecs: creation_time_usecs.microseconds,
        }
    }

    pub fn deposit(&self) -> u64 {
        match self {
            StateValueMetadata::V0 { deposit, .. } => *deposit,
        }
    }

    pub fn set_deposit(&mut self, amount: u64) {
        match self {
            StateValueMetadata::V0 { deposit, .. } => *deposit = amount,
        }
    }
}

#[derive(Clone, Debug, CryptoHasher)]
pub struct StateValue {
    inner: StateValueInner,
    hash: OnceCell<HashValue>,
}

impl PartialEq for StateValue {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl Eq for StateValue {}

#[derive(
    BCSCryptoHash,
    Clone,
    CryptoHasher,
    Debug,
    Deserialize,
    Eq,
    PartialEq,
    Serialize,
    Ord,
    PartialOrd,
    Hash,
)]
#[serde(rename = "StateValue")]
pub enum StateValueInner {
    V0(Bytes),
    WithMetadata {
        data: Bytes,
        metadata: StateValueMetadata,
    },
}

#[cfg(any(test, feature = "fuzzing"))]
impl Arbitrary for StateValue {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        any::<Vec<u8>>()
            .prop_map(|bytes| StateValue::new_legacy(bytes.into()))
            .boxed()
    }
}

impl<'de> Deserialize<'de> for StateValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let inner = StateValueInner::deserialize(deserializer)?;
        let hash = OnceCell::new();
        Ok(Self { inner, hash })
    }
}

impl Serialize for StateValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.inner.serialize(serializer)
    }
}

impl StateValue {
    pub fn new_legacy(bytes: Bytes) -> Self {
        Self::new_impl(StateValueInner::V0(bytes))
    }

    pub fn new_with_metadata(data: Bytes, metadata: StateValueMetadata) -> Self {
        Self::new_impl(StateValueInner::WithMetadata { data, metadata })
    }

    fn new_impl(inner: StateValueInner) -> Self {
        let hash = OnceCell::new();
        Self { inner, hash }
    }

    pub fn size(&self) -> usize {
        self.bytes().len()
    }

    pub fn bytes(&self) -> &Bytes {
        match &self.inner {
            StateValueInner::V0(data) | StateValueInner::WithMetadata { data, .. } => data,
        }
    }

    /// Applies a bytes-to-bytes transformation on the state value contents,
    /// leaving the state value metadata untouched.
    pub fn map_bytes<F: FnOnce(Bytes) -> anyhow::Result<Bytes>>(
        self,
        f: F,
    ) -> anyhow::Result<StateValue> {
        Ok(StateValue::new_impl(match self.inner {
            StateValueInner::V0(data) => StateValueInner::V0(f(data)?),
            StateValueInner::WithMetadata { data, metadata } => StateValueInner::WithMetadata {
                data: f(data)?,
                metadata,
            },
        }))
    }

    pub fn into_metadata(self) -> Option<StateValueMetadata> {
        match self.inner {
            StateValueInner::V0(_) => None,
            StateValueInner::WithMetadata { metadata, .. } => Some(metadata),
        }
    }

    pub fn into(self) -> (Option<StateValueMetadata>, Bytes) {
        match self.inner {
            StateValueInner::V0(bytes) => (None, bytes),
            StateValueInner::WithMetadata { data, metadata } => (Some(metadata), data),
        }
    }
}

// #[cfg(any(test, feature = "fuzzing"))]
impl From<Vec<u8>> for StateValue {
    fn from(bytes: Vec<u8>) -> Self {
        StateValue::new_legacy(bytes.into())
    }
}

#[cfg(any(test, feature = "fuzzing"))]
impl From<Bytes> for StateValue {
    fn from(bytes: Bytes) -> Self {
        StateValue::new_legacy(bytes)
    }
}

impl CryptoHash for StateValue {
    type Hasher = StateValueHasher;

    fn hash(&self) -> HashValue {
        *self.hash.get_or_init(|| CryptoHash::hash(&self.inner))
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

/// Indicates a state value becomes stale since `stale_since_version`.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
pub struct StaleStateValueIndex {
    /// The version since when the node is overwritten and becomes stale.
    pub stale_since_version: Version,
    /// The version identifying the value associated with this record.
    pub version: Version,
    /// The `StateKey` identifying the value associated with this record.
    pub state_key: StateKey,
}
