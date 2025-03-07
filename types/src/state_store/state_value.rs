// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    on_chain_config::CurrentTimeMicroseconds, proof::SparseMerkleRangeProof,
    state_store::state_key::StateKey, transaction::Version,
};
use aptos_crypto::{hash::SPARSE_MERKLE_PLACEHOLDER_HASH, HashValue};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher, Deref};
use bytes::Bytes;
#[cfg(any(test, feature = "fuzzing"))]
use proptest::{arbitrary::Arbitrary, prelude::*};
use ref_cast::RefCast;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::ops::Deref;

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExistingStateValue {
    data: Bytes,
    metadata: StateValueMetadata,
    access_time_secs: u32,
}

/// Shared memory layout between StateValue and DbStateValue. Avoids unnecessary memory movement
/// when converting between the two.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Store {
    Existing(ExistingStateValue),
    /// Non-existent, cached in the hot tier.
    HotNonExistent {
        access_time_secs: u32,
    },
}

impl Store {
    fn new_existing_without_access_ts(data: Bytes, metadata: StateValueMetadata) -> Self {
        Self::Existing(ExistingStateValue {
            data,
            metadata,
            access_time_secs: 0,
        })
    }

    fn new_hot_non_existent(access_time_secs: u32) -> Self {
        Self::HotNonExistent { access_time_secs }
    }

    fn set_access_time_secs(&mut self, secs: u32) {
        match self {
            Self::HotNonExistent { access_time_secs } => *access_time_secs = secs,
            Self::Existing(existing) => existing.access_time_secs = secs,
        }
    }

    pub fn is_hot_non_existent(&self) -> bool {
        matches!(self, Self::HotNonExistent { .. })
    }

    pub fn size(&self) -> usize {
        match self {
            Self::HotNonExistent { .. } => 0,
            Self::Existing(existing) => existing.data.len(),
        }
    }
}

/// StateValue for usage by the VM / `StateView`.
///
/// 1. It's guaranteed by constructors that it's not `HotNonExistent`.
/// 2. Access time is just placeholder, must be set when converting to DbStateValue.
#[derive(BCSCryptoHash, Clone, CryptoHasher, Debug, Eq, PartialEq, RefCast)]
#[repr(transparent)]
pub struct StateValue(Store);

impl StateValue {
    fn to_persistable_form(&self) -> PersistedStateValue {
        let ExistingStateValue {
            data,
            metadata,
            access_time_secs: _,
        } = self.inner().clone();
        let metadata = metadata.into_persistable();
        match metadata {
            None => PersistedStateValue::V0(data),
            Some(metadata) => PersistedStateValue::WithMetadata { data, metadata },
        }
    }
}

impl Deref for StateValue {
    type Target = ExistingStateValue;

    fn deref(&self) -> &Self::Target {
        self.inner()
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
    pub fn new_legacy(bytes: Bytes) -> Self {
        Self::new_with_metadata(bytes, StateValueMetadata::none())
    }

    pub fn new_with_metadata(data: Bytes, metadata: StateValueMetadata) -> Self {
        Self(Store::new_existing_without_access_ts(data, metadata))
    }

    /// Becomes a DbStateValue. It's expected to be an `Existing` flavor.
    pub fn into_db_state_value(mut self, access_time_secs: u32) -> DbStateValue {
        self.0.set_access_time_secs(access_time_secs);
        DbStateValue(self.0)
    }

    fn inner(&self) -> &ExistingStateValue {
        match &self.0 {
            Store::Existing(value) => value,
            Store::HotNonExistent { .. } => {
                unreachable!("Guaranteed to be the Existing flavor.")
            },
        }
    }

    fn inner_mut(&mut self) -> &mut ExistingStateValue {
        match &mut self.0 {
            Store::Existing(ref mut value) => value,
            Store::HotNonExistent { .. } => {
                unreachable!("Guaranteed to be the Existing flavor.")
            },
        }
    }

    fn into_inner(self) -> ExistingStateValue {
        match self.0 {
            Store::Existing(value) => value,
            Store::HotNonExistent { .. } => {
                unreachable!("Guaranteed to be the Existing flavor.")
            },
        }
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
        self,
        f: F,
    ) -> anyhow::Result<StateValue> {
        let inner = self.into_inner();
        Ok(Self::new_with_metadata(f(inner.data)?, inner.metadata))
    }

    pub fn into_bytes(self) -> Bytes {
        self.into_inner().data
    }

    pub fn set_bytes(&mut self, data: Bytes) {
        self.inner_mut().data = data;
    }

    pub fn metadata(&self) -> &StateValueMetadata {
        &self.inner().metadata
    }

    pub fn metadata_mut(&mut self) -> &mut StateValueMetadata {
        &mut self.inner_mut().metadata
    }

    pub fn into_metadata(self) -> StateValueMetadata {
        self.into_inner().metadata
    }

    pub fn unpack(self) -> (StateValueMetadata, Bytes) {
        let ExistingStateValue {
            data,
            metadata,
            access_time_secs: _,
        } = self.into_inner();
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

/// StateValue for usage by the DB and execution pipeline
///
/// 1. `HotNonExistent` flavor is possible
/// 2. Access time is properly set on construction.
#[derive(Clone, Debug, Deref, RefCast)]
#[repr(transparent)]
pub struct DbStateValue(Store);

impl DbStateValue {
    pub fn new_hot_non_existent(access_time_secs: u32) -> Self {
        Self(Store::new_hot_non_existent(access_time_secs))
    }

    pub fn from_state_value_opt(value_opt: Option<StateValue>, access_time_secs: u32) -> Self {
        value_opt.map_or_else(
            || Self::new_hot_non_existent(access_time_secs),
            |v| v.into_db_state_value(access_time_secs),
        )
    }

    /// Returns None if self is HotNonExistent, otherwise return self as
    /// Some(StateValue)
    pub fn to_state_value_opt(&self) -> Option<&StateValue> {
        match &self.0 {
            Store::Existing(_) => Some(StateValue::ref_cast(&self.0)),
            Store::HotNonExistent { .. } => None,
        }
    }

    pub fn into_state_value_opt(self) -> Option<StateValue> {
        match &self.0 {
            Store::Existing(..) => Some(StateValue(self.0)),
            Store::HotNonExistent { .. } => None,
        }
    }

    pub fn is_hot_non_existent(&self) -> bool {
        self.0.is_hot_non_existent()
    }

    pub fn expect_state_value(&self) -> &StateValue {
        assert!(!self.is_hot_non_existent());
        StateValue::ref_cast(&self.0)
    }

    pub fn access_time_secs(&self) -> u32 {
        match &self.0 {
            Store::Existing(existing) => existing.access_time_secs,
            Store::HotNonExistent { access_time_secs } => *access_time_secs,
        }
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
