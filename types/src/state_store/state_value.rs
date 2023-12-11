// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    on_chain_config::CurrentTimeMicroseconds, proof::SparseMerkleRangeProof,
    state_store::state_key::StateKey, transaction::Version,
};
use anyhow::ensure;
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
    V1 {
        slot_deposit: u64,
        bytes_deposit: u64,
        creation_time_usecs: u64,
    },
    /// In memory place holder for those state values that were created before StateValueMetadata
    /// was a thing.
    Dummy,
}

impl StateValueMetadata {
    // FIXME(aldenhu): update tests and remove
    pub fn new_v0(deposit: u64, creation_time_usecs: &CurrentTimeMicroseconds) -> Self {
        Self::V0 {
            deposit,
            creation_time_usecs: creation_time_usecs.microseconds,
        }
    }

    pub fn new_placeholder(creation_time_usecs: &CurrentTimeMicroseconds) -> Self {
        Self::new_v0(0, creation_time_usecs)
    }

    pub fn into_persistable(self) -> Option<Self> {
        use StateValueMetadata::*;

        match self {
            V0 { .. } | V1 { .. } => Some(self),
            Dummy => None,
        }
    }

    pub fn is_dummy(&self) -> bool {
        use StateValueMetadata::*;

        match self {
            Dummy => true,
            V0 { .. } | V1 { .. } => false,
        }
    }

    pub fn creation_time_usecs(&self) -> u64 {
        use StateValueMetadata::*;

        match self {
            V0 {
                creation_time_usecs,
                ..
            }
            | V1 {
                creation_time_usecs,
                ..
            } => *creation_time_usecs,
            Dummy => 0,
        }
    }

    pub fn slot_deposit(&self) -> u64 {
        use StateValueMetadata::*;

        match self {
            V0 { deposit, .. } => *deposit,
            V1 { slot_deposit, .. } => *slot_deposit,
            Dummy => 0,
        }
    }

    pub fn bytes_deposit(&self) -> u64 {
        use StateValueMetadata::*;

        match self {
            V0 { .. } => 0,
            V1 { bytes_deposit, .. } => *bytes_deposit,
            Dummy => 0,
        }
    }

    pub fn total_deposit(&self) -> u64 {
        self.slot_deposit() + self.bytes_deposit()
    }

    pub fn set_slot_deposit(&mut self, amount: u64) {
        use StateValueMetadata::*;

        match self {
            V0 { deposit, .. } => *deposit = amount,
            V1 { slot_deposit, .. } => *slot_deposit = amount,
            Dummy => {
                unreachable!("Not allowed to set slot deposit on Dummy. Upgrade first.")
            },
        }
    }

    pub fn set_bytes_deposit(&mut self, amount: u64) {
        use StateValueMetadata::*;

        match self {
            V1 { slot_deposit, .. } => *slot_deposit = amount,
            V0 { .. } | Dummy => {
                unreachable!("Not allowed to set slot deposit on Dummy or V0. Upgrade first.")
            },
        }
    }

    pub fn assert_v1(&self) {
        assert!(matches!(self, Self::V1 { .. }), "Expecting V1.")
    }

    pub fn upgrade(&mut self) -> &mut Self {
        *self = Self::V1 {
            slot_deposit: self.slot_deposit(),
            bytes_deposit: self.bytes_deposit(),
            creation_time_usecs: self.creation_time_usecs(),
        };
        self
    }

    pub fn ensure_equivalent(&self, other: &Self) -> anyhow::Result<()> {
        match self.clone().upgrade() {
            StateValueMetadata::V1 {
                slot_deposit,
                bytes_deposit,
                creation_time_usecs,
            } => {
                ensure!(
                    *slot_deposit == other.slot_deposit()
                        && *bytes_deposit == other.bytes_deposit()
                        && *creation_time_usecs == other.creation_time_usecs(),
                    "Not equivalent: {:?} vs {:?}",
                    self,
                    other,
                );
            },
            StateValueMetadata::V0 { .. } | StateValueMetadata::Dummy => {
                unreachable!("StateValueMetadata::upgrade() failed.")
            },
        }

        Ok(())
    }
}

#[derive(Clone, Debug, CryptoHasher)]
pub struct StateValue {
    inner: InMemoryStateValue,
    hash: OnceCell<HashValue>,
}

impl PartialEq for StateValue {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl Eq for StateValue {}

#[derive(BCSCryptoHash, CryptoHasher, Deserialize, Serialize)]
#[serde(rename = "StateValue")]
enum PersistedStateValue {
    V0(Bytes),
    WithMetadata {
        data: Bytes,
        metadata: StateValueMetadata,
    },
}

impl PersistedStateValue {
    fn into_in_mem_form(self) -> InMemoryStateValue {
        match self {
            PersistedStateValue::V0(data) => InMemoryStateValue {
                data,
                metadata: StateValueMetadata::Dummy,
            },
            PersistedStateValue::WithMetadata { data, metadata } => {
                InMemoryStateValue { data, metadata }
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct InMemoryStateValue {
    data: Bytes,
    metadata: StateValueMetadata,
}

impl InMemoryStateValue {
    fn to_persistable_form(&self) -> PersistedStateValue {
        let Self { data, metadata } = self.clone();

        match metadata {
            StateValueMetadata::V0 { .. } | StateValueMetadata::V1 { .. } => {
                PersistedStateValue::WithMetadata { data, metadata }
            },
            StateValueMetadata::Dummy => PersistedStateValue::V0(data),
        }
    }
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
        let inner = PersistedStateValue::deserialize(deserializer)?.into_in_mem_form();
        let hash = OnceCell::new();
        Ok(Self { inner, hash })
    }
}

impl Serialize for StateValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.inner.to_persistable_form().serialize(serializer)
    }
}

impl StateValue {
    pub fn new_legacy(bytes: Bytes) -> Self {
        Self::new_with_metadata(bytes, StateValueMetadata::Dummy)
    }

    pub fn new_with_metadata(data: Bytes, metadata: StateValueMetadata) -> Self {
        let inner = InMemoryStateValue { data, metadata };
        let hash = OnceCell::new();
        Self { inner, hash }
    }

    pub fn size(&self) -> usize {
        self.bytes().len()
    }

    pub fn bytes(&self) -> &Bytes {
        &self.inner.data
    }

    /// Applies a bytes-to-bytes transformation on the state value contents,
    /// leaving the state value metadata untouched.
    pub fn map_bytes<F: FnOnce(Bytes) -> anyhow::Result<Bytes>>(
        self,
        f: F,
    ) -> anyhow::Result<StateValue> {
        Ok(Self::new_with_metadata(
            f(self.inner.data)?,
            self.inner.metadata,
        ))
    }

    pub fn into_metadata(self) -> StateValueMetadata {
        self.inner.metadata
    }

    pub fn into_parts(self) -> (StateValueMetadata, Bytes) {
        let InMemoryStateValue { data, metadata } = self.inner;
        (metadata, data)
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
        *self
            .hash
            .get_or_init(|| CryptoHash::hash(&self.inner.to_persistable_form()))
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
