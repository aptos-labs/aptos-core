// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    on_chain_config::CurrentTimeMicroseconds, proof::SparseMerkleRangeProof,
    state_store::state_key::StateKey, transaction::Version,
};
use aptos_crypto::{hash::SPARSE_MERKLE_PLACEHOLDER_HASH, HashValue};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use bytes::Bytes;
#[cfg(any(test, feature = "fuzzing"))]
use proptest::{arbitrary::Arbitrary, collection::vec, prelude::*};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Deserialize, Serialize)]
#[serde(rename = "StateValueMetadata")]
pub enum PersistedStateValueMetadata {
    V0 {
        deposit: u64,
        creation_time_usecs: u64,
    },
    V1 {
        slot_deposit: u64,
        bytes_deposit: u64,
        creation_time_usecs: u64,
    },
}

impl PersistedStateValueMetadata {
    pub fn into_in_mem_form(self) -> StateValueMetadata {
        match self {
            PersistedStateValueMetadata::V0 {
                deposit,
                creation_time_usecs,
            } => StateValueMetadata::new_impl(deposit, 0, creation_time_usecs),
            PersistedStateValueMetadata::V1 {
                slot_deposit,
                bytes_deposit,
                creation_time_usecs,
            } => StateValueMetadata::new_impl(slot_deposit, bytes_deposit, creation_time_usecs),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
struct StateValueMetadataInner {
    slot_deposit: u64,
    bytes_deposit: u64,
    creation_time_usecs: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct StateValueMetadata {
    inner: Option<StateValueMetadataInner>,
}

impl StateValueMetadata {
    pub fn into_persistable(self) -> Option<PersistedStateValueMetadata> {
        self.inner.map(|inner| {
            let StateValueMetadataInner {
                slot_deposit,
                bytes_deposit,
                creation_time_usecs,
            } = inner;
            if bytes_deposit == 0 {
                PersistedStateValueMetadata::V0 {
                    deposit: slot_deposit,
                    creation_time_usecs,
                }
            } else {
                PersistedStateValueMetadata::V1 {
                    slot_deposit,
                    bytes_deposit,
                    creation_time_usecs,
                }
            }
        })
    }

    pub fn new(
        slot_deposit: u64,
        bytes_deposit: u64,
        creation_time_usecs: &CurrentTimeMicroseconds,
    ) -> Self {
        Self::new_impl(
            slot_deposit,
            bytes_deposit,
            creation_time_usecs.microseconds,
        )
    }

    pub fn legacy(slot_deposit: u64, creation_time_usecs: &CurrentTimeMicroseconds) -> Self {
        Self::new(slot_deposit, 0, creation_time_usecs)
    }

    pub fn placeholder(creation_time_usecs: &CurrentTimeMicroseconds) -> Self {
        Self::legacy(0, creation_time_usecs)
    }

    pub fn none() -> Self {
        Self { inner: None }
    }

    fn new_impl(slot_deposit: u64, bytes_deposit: u64, creation_time_usecs: u64) -> Self {
        Self {
            inner: Some(StateValueMetadataInner {
                slot_deposit,
                bytes_deposit,
                creation_time_usecs,
            }),
        }
    }

    pub fn is_none(&self) -> bool {
        self.inner.is_none()
    }

    fn inner(&self) -> Option<&StateValueMetadataInner> {
        self.inner.as_ref()
    }

    pub fn creation_time_usecs(&self) -> u64 {
        self.inner().map_or(0, |v1| v1.creation_time_usecs)
    }

    pub fn slot_deposit(&self) -> u64 {
        self.inner().map_or(0, |v1| v1.slot_deposit)
    }

    pub fn bytes_deposit(&self) -> u64 {
        self.inner().map_or(0, |v1| v1.bytes_deposit)
    }

    pub fn total_deposit(&self) -> u64 {
        self.slot_deposit() + self.bytes_deposit()
    }

    pub fn maybe_upgrade(&mut self) -> &mut Self {
        *self = Self::new_impl(
            self.slot_deposit(),
            self.bytes_deposit(),
            self.creation_time_usecs(),
        );
        self
    }

    fn expect_upgraded(&mut self) -> &mut StateValueMetadataInner {
        self.inner.as_mut().expect("State metadata is None.")
    }

    pub fn set_slot_deposit(&mut self, amount: u64) {
        self.expect_upgraded().slot_deposit = amount;
    }

    pub fn set_bytes_deposit(&mut self, amount: u64) {
        self.expect_upgraded().bytes_deposit = amount;
    }
}

#[derive(BCSCryptoHash, CryptoHasher, Deserialize, Serialize)]
#[serde(rename = "StateValue")]
enum PersistedStateValue {
    V0(Bytes),
    WithMetadata {
        data: Bytes,
        metadata: PersistedStateValueMetadata,
    },
}

impl PersistedStateValue {
    fn into_in_mem_form(self) -> StateValue {
        match self {
            PersistedStateValue::V0(data) => StateValue::new_legacy(data),
            PersistedStateValue::WithMetadata { data, metadata } => {
                StateValue::new_with_metadata(data, metadata.into_in_mem_form())
            },
        }
    }
}

#[derive(BCSCryptoHash, Clone, CryptoHasher, Debug, Eq, PartialEq)]
pub struct StateValue {
    data: Bytes,
    metadata: StateValueMetadata,
}

pub const ARB_STATE_VALUE_MAX_SIZE: usize = 100;

#[cfg(any(test, feature = "fuzzing"))]
impl Arbitrary for StateValue {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        vec(any::<u8>(), 0..=ARB_STATE_VALUE_MAX_SIZE)
            .prop_map(|bytes| StateValue::new_legacy(bytes.into()))
            .boxed()
    }
}

impl<'de> Deserialize<'de> for StateValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(PersistedStateValue::deserialize(deserializer)?.into_in_mem_form())
    }
}

impl Serialize for StateValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_persistable_form().serialize(serializer)
    }
}

impl StateValue {
    fn to_persistable_form(&self) -> PersistedStateValue {
        let Self { data, metadata } = self.clone();
        let metadata = metadata.into_persistable();
        match metadata {
            None => PersistedStateValue::V0(data),
            Some(metadata) => PersistedStateValue::WithMetadata { data, metadata },
        }
    }

    pub fn new_legacy(bytes: Bytes) -> Self {
        Self::new_with_metadata(bytes, StateValueMetadata::none())
    }

    pub fn new_with_metadata(data: Bytes, metadata: StateValueMetadata) -> Self {
        Self { data, metadata }
    }

    pub fn size(&self) -> usize {
        self.bytes().len()
    }

    pub fn bytes(&self) -> &Bytes {
        &self.data
    }

    /// Applies a bytes-to-bytes transformation on the state value contents,
    /// leaving the state value metadata untouched.
    pub fn map_bytes<F: FnOnce(Bytes) -> anyhow::Result<Bytes>>(
        mut self,
        f: F,
    ) -> anyhow::Result<StateValue> {
        self.data = f(self.data)?;
        Ok(self)
    }

    pub fn into_bytes(self) -> Bytes {
        self.data
    }

    pub fn set_bytes(&mut self, data: Bytes) {
        self.data = data;
    }

    pub fn metadata(&self) -> &StateValueMetadata {
        &self.metadata
    }

    pub fn metadata_mut(&mut self) -> &mut StateValueMetadata {
        &mut self.metadata
    }

    pub fn into_metadata(self) -> StateValueMetadata {
        self.metadata
    }

    pub fn unpack(self) -> (StateValueMetadata, Bytes) {
        let Self { data, metadata } = self;

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

/// Indicates a state value becomes stale since `stale_since_version`.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
pub struct StaleStateValueByKeyHashIndex {
    /// The version since when the node is overwritten and becomes stale.
    pub stale_since_version: Version,
    /// The version identifying the value associated with this record.
    pub version: Version,
    /// The hash of `StateKey` identifying the value associated with this record.
    pub state_key_hash: HashValue,
}
