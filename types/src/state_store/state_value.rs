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
use proptest::{arbitrary::Arbitrary, collection::vec, prelude::*};
use ref_cast::RefCast;
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

impl StateValueMetadataInner {
    fn new(slot_deposit: u64, bytes_deposit: u64, creation_time_usecs: u64) -> Self {
        Self {
            slot_deposit,
            bytes_deposit,
            creation_time_usecs,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct StateValueMetadata {
    inner: Option<StateValueMetadataInner>,
    last_data_change_usecs: Option<u64>,
    // FIXME(aldenhu): change to usecs
    hot_since_secs: Option<u32>,
}

impl StateValueMetadata {
    pub fn into_persistable(self) -> Option<PersistedStateValueMetadata> {
        let Self {
            inner,
            // TODO(HotState): Revisit when persisting the hot state.
            last_data_change_usecs: _,
            // TODO(HotState): Revisit when persisting the hot state.
            hot_since_secs: _,

        } = self;

        inner.map(|inner| {
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

    // FIXME(aldenhu): rename to cold
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

    // FIXME(aldenhu): rename to cold_legacy
    pub fn legacy(slot_deposit: u64, creation_time_usecs: &CurrentTimeMicroseconds) -> Self {
        Self::new(slot_deposit, 0, creation_time_usecs)
    }

    pub fn placeholder(creation_time_usecs: &CurrentTimeMicroseconds) -> Self {
        Self::legacy(0, creation_time_usecs)
    }

    // FIXME(aldenhu): rename to cold_none
    pub fn none() -> Self {
        Self {
            inner: None,
            last_data_change_usecs: None,
            hot_since_secs: None,
        }
    }

    pub fn hot_none(hot_since_secs: u32) -> Self {
        Self {
            inner: None,
            last_data_change_usecs: None,
            hot_since_secs: Some(hot_since_secs),
        }
    }

    // FIXME(aldenhu): rename to new_cold_impl
    // FIXME(aldenhu): check call sites -- is it right to set hot_since=None?
    fn new_impl(slot_deposit: u64, bytes_deposit: u64, creation_time_usecs: u64) -> Self {
        Self {
            inner: Some(StateValueMetadataInner {
                slot_deposit,
                bytes_deposit,
                creation_time_usecs,
            }),
            last_data_change_usecs: Some(creation_time_usecs),
            hot_since_secs: None,
        }
    }

    pub fn is_none(&self) -> bool {
        self.inner.is_none()
    }

    pub fn is_hot(&self) -> bool {
        self.hot_since_secs.is_some()
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
        self.inner = Some(StateValueMetadataInner::new(
            self.slot_deposit(),
            self.bytes_deposit(),
            self.creation_time_usecs(),
        ));
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

/// TODO(HotState): Note: this is the persistent format in the cold state.
///                 Revisit: Maybe use StateSlot for hot state.
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

/* FIXME(aldenhu): remove
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExistingStateValue {
    data: Bytes,
    metadata: StateValueMetadata,
    access_time_secs: u32,
}
 */

/// Shared memory layout between StateValue and StateSlot. Avoids unnecessary memory movement
/// when converting between the two.
///
/// StateSlot can be empty, StateValue can not.
#[derive(Clone, Debug)]
pub struct Store {
    data: Option<Bytes>,
    metadata: StateValueMetadata,
}

impl Store {
    fn occupied(data: Bytes, metadata: StateValueMetadata) -> Self {
        Self {
            data: Some(data),
            metadata,
        }
    }

    fn new_hot_non_existent(hot_since_secs: u32) -> Self {
        Self {
            data: None,
            metadata: StateValueMetadata::hot_none(hot_since_secs),
        }
    }

    // FIXME(aldenhu): rename to set_hot_since_secs
    fn set_access_time_secs(&mut self, secs: u32) {
        self.metadata.hot_since_secs = Some(secs);
    }

    /* FIXME(aldenhu): remove
    pub fn is_hot_non_existent(&self) -> bool {
        self.data.is_none() && self.metadata.hot_since_secs.is_some()
    }
     */

    pub fn size(&self) -> usize {
        self.data.as_ref().map_or(0, Bytes::len)
    }
}

/// StateValue for usage by the VM / `StateView`.
///
/// 1. It's guaranteed by constructors that it's not `HotNonExistent`.
/// 2. Access time is just placeholder, must be set when converting to DbStateValue.
#[derive(BCSCryptoHash, Clone, CryptoHasher, Debug, RefCast)]
#[repr(transparent)]
pub struct StateValue(Store);

impl PartialEq for StateValue {
    fn eq(&self, other: &Self) -> bool {
        // Ignoring hot_state_since for equality check for now.
        // TODO(HotState): change after the hot state is deterministic
        self.0.data == other.0.data && self.0.metadata == other.0.metadata
    }
}

impl Eq for StateValue {}

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
        let Store { data, metadata } = self.0.clone();
        let data = data.expect("persisting empty slot to cold storage.");

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
        Self(Store::occupied(data, metadata))
    }

    /// Becomes a DbStateValue. It's expected to be an `Existing` flavor.
    pub fn into_db_state_value(mut self, access_time_secs: u32) -> StateSlot {
        self.0.set_access_time_secs(access_time_secs);
        StateSlot(self.0)
    }

    pub fn size(&self) -> usize {
        self.0.size()
    }

    pub fn bytes(&self) -> &Bytes {
        self.0
            .data
            .as_ref()
            .expect("StateValue expected from empty slot.")
    }

    pub fn into_bytes(self) -> Bytes {
        self.0.data.expect("StateValue expected from empty slot.")
    }

    /// Applies a bytes-to-bytes transformation on the state value contents,
    /// leaving the state value metadata untouched.
    pub fn map_bytes<F: FnOnce(Bytes) -> anyhow::Result<Bytes>>(
        mut self,
        f: F,
    ) -> anyhow::Result<StateValue> {
        self.0.data = self.0.data.map(f).transpose()?;
        Ok(self)
    }

    pub fn set_bytes(&mut self, data: Bytes) {
        self.0.data = Some(data);
    }

    pub fn metadata(&self) -> &StateValueMetadata {
        &self.0.metadata
    }

    pub fn metadata_mut(&mut self) -> &mut StateValueMetadata {
        &mut self.0.metadata
    }

    pub fn into_metadata(self) -> StateValueMetadata {
        self.0.metadata
    }

    pub fn unpack(self) -> (StateValueMetadata, Bytes) {
        let Store { data, metadata } = self.0;
        (
            metadata,
            data.expect("StateValue expected from empty slot."),
        )
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

/// FIXME(aldenhu): update doc
#[derive(Clone, Debug, Deref, RefCast)]
#[repr(transparent)]
pub struct StateSlot(Store);

impl StateSlot {
    pub fn new_hot_non_existent(access_time_secs: u32) -> Self {
        Self(Store::new_hot_non_existent(access_time_secs))
    }

    /// Returns None if self is empty, otherwise return self as
    /// Some(StateValue)
    pub fn to_state_value_opt(&self) -> Option<&StateValue> {
        self.data.is_some().then(|| StateValue::ref_cast(&self.0))
    }

    pub fn into_state_value_opt(self) -> Option<StateValue> {
        self.data.is_some().then(|| StateValue(self.0))
    }

    /* FIXME(aldenhu): remove
    pub fn is_hot_non_existent(&self) -> bool {
        self.0.is_hot_non_existent()
    }
     */

    pub fn expect_occupied(&self) -> &StateValue {
        assert!(self.0.data.is_some());
        StateValue::ref_cast(&self.0)
    }

    // FIXME(aldenhu): rename to hot_since_secs_opt
    pub fn access_time_secs(&self) -> Option<u32> {
        self.0.metadata.hot_since_secs
    }

    pub fn with_access_time_secs(mut self, access_time_secs: u32) -> Self {
        self.0.set_access_time_secs(access_time_secs);
        self
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
