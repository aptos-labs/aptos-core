// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! For each transaction the VM executes, the VM will output a `WriteSet` that contains each access
//! path it updates. For each access path, the VM can either give its new value or delete it.

use crate::state_store::{
    state_key::StateKey,
    state_slot::StateSlot,
    state_value::{PersistedStateValueMetadata, StateValue, StateValueMetadata},
};
use anyhow::{bail, ensure, Result};
use velor_crypto_derive::{BCSCryptoHash, CryptoHasher};
use ark_std::iterable::Iterable;
use bytes::Bytes;
use itertools::{EitherOrBoth, Itertools};
use once_cell::sync::Lazy;
use ref_cast::RefCast;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    collections::{btree_map, BTreeMap},
    fmt::{Debug, Formatter},
};
use strum_macros::AsRefStr;

// Note: in case this changes in the future, it doesn't have to be a constant, and can be read from
// genesis directly if necessary.
pub static TOTAL_SUPPLY_STATE_KEY: Lazy<StateKey> = Lazy::new(|| {
    StateKey::table_item(
        &"1b854694ae746cdbd8d44186ca4929b2b337df21d1c74633be19b2710552fdca"
            .parse()
            .unwrap(),
        &[
            6, 25, 220, 41, 160, 170, 200, 250, 20, 103, 20, 5, 142, 141, 214, 210, 208, 243, 189,
            245, 246, 51, 25, 7, 191, 145, 243, 172, 216, 30, 105, 53,
        ],
    )
});

#[derive(Eq, Clone, Debug, PartialEq)]
pub enum WriteOpKind {
    Creation,
    Modification,
    Deletion,
}

#[derive(Serialize, Deserialize)]
#[serde(rename = "WriteOp")]
enum PersistedWriteOp {
    Creation(Bytes),
    Modification(Bytes),
    Deletion,
    CreationWithMetadata {
        data: Bytes,
        metadata: PersistedStateValueMetadata,
    },
    ModificationWithMetadata {
        data: Bytes,
        metadata: PersistedStateValueMetadata,
    },
    DeletionWithMetadata {
        metadata: PersistedStateValueMetadata,
    },
}

impl PersistedWriteOp {
    fn into_in_mem_form(self) -> WriteOp {
        use PersistedWriteOp::*;

        match self {
            Creation(data) => WriteOp::legacy_creation(data),
            Modification(data) => WriteOp::legacy_modification(data),
            Deletion => WriteOp::legacy_deletion(),
            CreationWithMetadata { data, metadata } => {
                WriteOp::creation(data, metadata.into_in_mem_form())
            },
            ModificationWithMetadata { data, metadata } => {
                WriteOp::modification(data, metadata.into_in_mem_form())
            },
            DeletionWithMetadata { metadata } => WriteOp::deletion(metadata.into_in_mem_form()),
        }
    }
}

/// Shared in memory representation between the (value) WriteOp and the (hotness) HotStateOp
#[derive(Clone, Debug, Eq, PartialEq, AsRefStr)]
pub enum BaseStateOp {
    Creation(StateValue),
    Modification(StateValue),
    Deletion(StateValueMetadata),
    MakeHot { prev_slot: StateSlot },
    Eviction { prev_slot: StateSlot },
}

impl BaseStateOp {
    pub fn as_state_value_opt(&self) -> Option<&StateValue> {
        use BaseStateOp::*;

        match self {
            Creation(val) | Modification(val) => Some(val),
            Deletion(_) => None,
            MakeHot { prev_slot } => prev_slot.as_state_value_opt(),
            Eviction { prev_slot } => prev_slot.as_state_value_opt(),
        }
    }

    pub fn as_write_op_opt(&self) -> Option<&WriteOp> {
        use BaseStateOp::*;

        match self {
            Creation(_) | Modification(_) | Deletion(_) => Some(WriteOp::ref_cast(self)),
            MakeHot { .. } | Eviction { .. } => None,
        }
    }

    pub fn expect_as_write_op(&self) -> &WriteOp {
        self.as_write_op_opt().expect("Expected write op")
    }

    pub fn is_value_write_op(&self) -> bool {
        use BaseStateOp::*;

        match self {
            Creation(_) | Modification(_) | Deletion(_) => true,
            MakeHot { .. } | Eviction { .. } => false,
        }
    }
}

/// Represents a change to a state value.
#[derive(Clone, Eq, PartialEq, RefCast)]
#[repr(transparent)]
pub struct WriteOp(BaseStateOp);

impl WriteOp {
    fn to_persistable(&self) -> PersistedWriteOp {
        use PersistedWriteOp::*;

        let metadata = self.metadata().clone().into_persistable();
        match metadata {
            None => match &self.0 {
                BaseStateOp::Creation(v) => Creation(v.bytes().clone()),
                BaseStateOp::Modification(v) => Modification(v.bytes().clone()),
                BaseStateOp::Deletion { .. } => Deletion,
                BaseStateOp::MakeHot { .. } | BaseStateOp::Eviction { .. } => {
                    unreachable!("malformed write op")
                },
            },
            Some(metadata) => match &self.0 {
                BaseStateOp::Creation(v) => CreationWithMetadata {
                    data: v.bytes().clone(),
                    metadata,
                },
                BaseStateOp::Modification(v) => ModificationWithMetadata {
                    data: v.bytes().clone(),
                    metadata,
                },
                BaseStateOp::Deletion { .. } => DeletionWithMetadata { metadata },
                BaseStateOp::MakeHot { .. } | BaseStateOp::Eviction { .. } => {
                    unreachable!("malformed write op")
                },
            },
        }
    }

    /// Merges two write ops on the same state item.
    ///
    /// returns `false` if the result indicates no op has happened -- that's when the first op
    ///   creates the item and the second deletes it.
    pub fn squash(op: &mut Self, other: Self) -> Result<bool> {
        use BaseStateOp::*;

        match (&mut op.0, other.0) {
            (MakeHot { .. }, ..) | (.., MakeHot { .. })
            | (Eviction { .. }, ..) | (.., Eviction { .. }) => unreachable!("malformed write op"),
            (Modification { .. } | Creation { .. }, Creation { .. }) // create existing
            | (Deletion { .. }, Modification { .. } | Deletion { .. }) // delete or modify already deleted
            => {
                bail!("The given change sets cannot be squashed")
            },
            (Creation(c) , Modification(m)) => {
                Self::ensure_metadata_compatible(c.metadata(), m.metadata())?;

                *op = Self(Creation(m));
            },
            (Modification(c) , Modification(m)) => {
                Self::ensure_metadata_compatible(c.metadata(), m.metadata())?;

                *op = Self(Modification(m));
            },
            (Modification(m), Deletion(d_meta)) => {
                Self::ensure_metadata_compatible(m.metadata(), &d_meta)?;

                *op = Self(Deletion(d_meta))
            },
            (Deletion(d_meta), Creation(c)) => {
                // n.b. With write sets from multiple sessions being squashed together, it's possible
                //   to see two ops carrying different metadata (or one with it the other without)
                //   due to deleting in one session and recreating in another. The original metadata
                //   shouldn't change due to the squash.
                // And because the deposit or refund happens after all squashing is finished, it's
                // not a concern of fairness.
                *op = Self(Modification(StateValue::new_with_metadata(c.into_bytes(), d_meta.clone())))
            },
            (Creation(c), Deletion(d_meta)) => {
                Self::ensure_metadata_compatible(c.metadata(), &d_meta)?;

                return Ok(false)
            },
        }
        Ok(true)
    }

    fn ensure_metadata_compatible(
        old: &StateValueMetadata,
        new: &StateValueMetadata,
    ) -> Result<()> {
        // Write ops shouldn't be squashed after the second one is charged for fees, which might
        // result in metadata change (bytes_deposit increase, for example).
        ensure!(
            old == new,
            "Squashing incompatible metadata: old:{old:?}, new:{new:?}",
        );
        Ok(())
    }

    pub fn as_state_value_opt(&self) -> Option<&StateValue> {
        self.0.as_state_value_opt()
    }

    pub fn bytes(&self) -> Option<&Bytes> {
        self.as_state_value_opt().map(StateValue::bytes)
    }

    /// Size not counting metadata.
    pub fn bytes_size(&self) -> usize {
        self.bytes().map_or(0, Bytes::len)
    }

    pub fn metadata(&self) -> &StateValueMetadata {
        use BaseStateOp::*;

        match &self.0 {
            Creation(v) | Modification(v) => v.metadata(),
            Deletion(meta) => meta,
            MakeHot { .. } | Eviction { .. } => unreachable!("malformed write op"),
        }
    }

    pub fn metadata_mut(&mut self) -> &mut StateValueMetadata {
        use BaseStateOp::*;

        match &mut self.0 {
            Creation(v) | Modification(v) => v.metadata_mut(),
            Deletion(meta) => meta,
            MakeHot { .. } | Eviction { .. } => unreachable!("malformed write op"),
        }
    }

    pub fn into_metadata(self) -> StateValueMetadata {
        use BaseStateOp::*;

        match self.0 {
            Creation(v) | Modification(v) => v.into_metadata(),
            Deletion(meta) => meta,
            MakeHot { .. } | Eviction { .. } => unreachable!("malformed write op"),
        }
    }

    pub fn creation(data: Bytes, metadata: StateValueMetadata) -> Self {
        Self(BaseStateOp::Creation(StateValue::new_with_metadata(
            data, metadata,
        )))
    }

    pub fn modification(data: Bytes, metadata: StateValueMetadata) -> Self {
        Self(BaseStateOp::Modification(StateValue::new_with_metadata(
            data, metadata,
        )))
    }

    pub fn modification_to_value(state_value: StateValue) -> Self {
        Self(BaseStateOp::Modification(state_value))
    }

    pub fn deletion(metadata: StateValueMetadata) -> Self {
        Self(BaseStateOp::Deletion(metadata))
    }

    pub fn legacy_creation(data: Bytes) -> Self {
        Self(BaseStateOp::Creation(StateValue::new_legacy(data)))
    }

    pub fn legacy_modification(data: Bytes) -> Self {
        Self(BaseStateOp::Modification(StateValue::new_legacy(data)))
    }

    pub fn legacy_deletion() -> Self {
        Self(BaseStateOp::Deletion(StateValueMetadata::none()))
    }

    pub fn project_write_op_size<GetSize>(&self, get_size: GetSize) -> WriteOpSize
    where
        GetSize: FnOnce() -> Option<u64>,
    {
        use BaseStateOp::*;

        match &self.0 {
            Creation { .. } => WriteOpSize::Creation {
                write_len: get_size().expect("Creation must have size"),
            },
            Modification { .. } => WriteOpSize::Modification {
                write_len: get_size().expect("Modification must have size"),
            },
            Deletion { .. } => WriteOpSize::Deletion,
            MakeHot { .. } | Eviction { .. } => unreachable!("malformed write op"),
        }
    }

    pub fn as_base_op(&self) -> &BaseStateOp {
        &self.0
    }

    pub fn into_base_op(self) -> BaseStateOp {
        self.0
    }

    pub fn is_delete(&self) -> bool {
        use BaseStateOp::*;

        match &self.0 {
            Creation(_) | Modification(_) => false,
            Deletion(_) => true,
            MakeHot { .. } | Eviction { .. } => unreachable!("malformed write op"),
        }
    }
}

impl<'de> Deserialize<'de> for WriteOp {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        PersistedWriteOp::deserialize(deserializer).map(|persisted| persisted.into_in_mem_form())
    }
}

impl Serialize for WriteOp {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_persistable().serialize(serializer)
    }
}

pub enum WriteOpSize {
    Creation { write_len: u64 },
    Modification { write_len: u64 },
    Deletion,
}

impl WriteOpSize {
    pub fn write_len(&self) -> Option<u64> {
        match self {
            WriteOpSize::Creation { write_len } | WriteOpSize::Modification { write_len } => {
                Some(*write_len)
            },
            WriteOpSize::Deletion => None,
        }
    }
}

pub trait TransactionWrite: Debug {
    fn bytes(&self) -> Option<&Bytes>;

    // Returns state value that would be observed by a read following the 'self' write.
    fn as_state_value(&self) -> Option<StateValue>;

    // Returns metadata that would be observed by a read following the 'self' write.
    // Provided as a separate method to avoid the clone in as_state_value method
    // (although default implementation below does just that).
    fn as_state_value_metadata(&self) -> Option<StateValueMetadata> {
        self.as_state_value().map(StateValue::into_metadata)
    }

    // Often, the contents of W:TransactionWrite are converted to Option<StateValue>, e.g.
    // to emulate reading from storage after W has been applied. However, in some contexts,
    // it is also helpful to convert a StateValue to a potential instance of W that would
    // have the desired effect. This allows e.g. storing sentinel elements of type W in
    // data-structures (notably in MVHashMap). The kind of W will be Modification and not
    // Creation, but o.w. if there are several instances of W that correspond to the
    // provided maybe_state_value, an arbitrary one may be provided.
    fn from_state_value(maybe_state_value: Option<StateValue>) -> Self;

    fn extract_raw_bytes(&self) -> Option<Bytes> {
        self.bytes().cloned()
    }

    fn as_u128(&self) -> anyhow::Result<Option<u128>> {
        match self.bytes() {
            Some(bytes) => Ok(Some(bcs::from_bytes(bytes)?)),
            None => Ok(None),
        }
    }

    fn write_op_kind(&self) -> WriteOpKind;

    fn is_deletion(&self) -> bool {
        self.write_op_kind() == WriteOpKind::Deletion
    }

    fn is_creation(&self) -> bool {
        self.write_op_kind() == WriteOpKind::Creation
    }

    fn is_modification(&self) -> bool {
        self.write_op_kind() == WriteOpKind::Modification
    }

    fn set_bytes(&mut self, bytes: Bytes);

    fn write_op_size(&self) -> WriteOpSize {
        use WriteOpKind::*;
        match self.write_op_kind() {
            Creation => WriteOpSize::Creation {
                write_len: self.bytes().unwrap().len() as u64,
            },
            Modification => WriteOpSize::Modification {
                write_len: self.bytes().unwrap().len() as u64,
            },
            Deletion => WriteOpSize::Deletion,
        }
    }
}

impl TransactionWrite for WriteOp {
    fn bytes(&self) -> Option<&Bytes> {
        self.bytes()
    }

    fn as_state_value(&self) -> Option<StateValue> {
        self.as_state_value_opt().cloned()
    }

    // Note that even if WriteOp is DeletionWithMetadata, the method returns None, as a later
    // read would not read the metadata of the deletion op.
    fn as_state_value_metadata(&self) -> Option<StateValueMetadata> {
        self.as_state_value_opt().map(StateValue::metadata).cloned()
    }

    fn from_state_value(maybe_state_value: Option<StateValue>) -> Self {
        match maybe_state_value {
            None => Self::legacy_deletion(),
            Some(state_value) => Self(BaseStateOp::Modification(state_value)),
        }
    }

    fn write_op_kind(&self) -> WriteOpKind {
        use WriteOpKind::*;
        match &self.0 {
            BaseStateOp::Creation { .. } => Creation,
            BaseStateOp::Modification { .. } => Modification,
            BaseStateOp::Deletion { .. } => Deletion,
            BaseStateOp::MakeHot { .. } | BaseStateOp::Eviction { .. } => {
                unreachable!("malformed write op")
            },
        }
    }

    fn set_bytes(&mut self, bytes: Bytes) {
        use BaseStateOp::*;

        match &mut self.0 {
            Creation(v) | Modification(v) => v.set_bytes(bytes),
            Deletion { .. } => (),
            MakeHot { .. } | Eviction { .. } => unreachable!("malformed write op"),
        }
    }
}

#[allow(clippy::format_collect)]
impl Debug for WriteOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use BaseStateOp::*;

        match &self.0 {
            Creation(v) => write!(
                f,
                "Creation({}, metadata:{:?})",
                v.bytes()
                    .iter()
                    .map(|byte| format!("{:02x}", byte))
                    .collect::<String>(),
                v.metadata(),
            ),
            Modification(v) => write!(
                f,
                "Modification({}, metadata:{:?})",
                v.bytes()
                    .iter()
                    .map(|byte| format!("{:02x}", byte))
                    .collect::<String>(),
                v.metadata(),
            ),
            Deletion(metadata) => {
                write!(f, "Deletion(metadata:{:?})", metadata,)
            },
            MakeHot { .. } | Eviction { .. } => unreachable!("malformed write op"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename = "WriteSet")]
pub enum ValueWriteSet {
    V0(WriteSetV0),
}

impl Default for ValueWriteSet {
    fn default() -> Self {
        Self::V0(WriteSetV0::default())
    }
}

// TODO(HotState): revisit when the hot state is deterministic.
/// Represents a hotness only change, not persisted for now.
#[derive(Clone, Eq, PartialEq)]
pub struct HotStateOp(BaseStateOp);

impl HotStateOp {
    pub fn make_hot(prev_slot: StateSlot) -> Self {
        Self(BaseStateOp::MakeHot { prev_slot })
    }

    pub fn as_base_op(&self) -> &BaseStateOp {
        &self.0
    }

    pub fn into_base_op(self) -> BaseStateOp {
        self.0
    }
}

impl Debug for HotStateOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use BaseStateOp::*;

        match &self.0 {
            MakeHot { prev_slot } => {
                write!(f, "MakeHot(prev_slot:{:?})", prev_slot)
            },
            Eviction { prev_slot } => {
                write!(f, "Eviction(prev_slot:{:?})", prev_slot)
            },
            Creation(_) | Modification(_) | Deletion(_) => {
                unreachable!("malformed hot state op")
            },
        }
    }
}

#[derive(BCSCryptoHash, Clone, CryptoHasher, Debug, Default, Eq, PartialEq)]
pub struct WriteSet {
    value: ValueWriteSet,
    /// TODO(HotState): this field is not serialized for now.
    hotness: BTreeMap<StateKey, HotStateOp>,
}

impl Serialize for WriteSet {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.value.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for WriteSet {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = ValueWriteSet::deserialize(deserializer)?;
        Ok(Self {
            value,
            hotness: BTreeMap::new(),
        })
    }
}

impl WriteSet {
    fn into_v0(self) -> WriteSetV0 {
        match self.value {
            ValueWriteSet::V0(ws) => ws,
        }
    }

    pub fn as_v0(&self) -> &WriteSetV0 {
        match &self.value {
            ValueWriteSet::V0(ws) => ws,
        }
    }

    fn as_v0_mut(&mut self) -> &mut WriteSetV0 {
        match &mut self.value {
            ValueWriteSet::V0(ws) => ws,
        }
    }

    pub fn into_mut(self) -> WriteSetMut {
        self.into_v0().0
    }

    pub fn new(write_ops: impl IntoIterator<Item = (StateKey, WriteOp)>) -> Result<Self> {
        WriteSetMut::new(write_ops).freeze()
    }

    pub fn new_for_test(kvs: impl IntoIterator<Item = (StateKey, Option<StateValue>)>) -> Self {
        Self::new(kvs.into_iter().map(|(k, v_opt)| {
            (
                k,
                v_opt.map_or_else(WriteOp::legacy_deletion, |v| {
                    WriteOp::legacy_modification(v.bytes().clone())
                }),
            )
        }))
        .expect("Must succeed")
    }

    pub fn state_update_refs(&self) -> impl Iterator<Item = (&StateKey, Option<&StateValue>)> + '_ {
        self.as_v0()
            .iter()
            .map(|(key, op)| (key, op.as_state_value_opt()))
    }

    pub fn state_updates_cloned(
        &self,
    ) -> impl Iterator<Item = (StateKey, Option<StateValue>)> + '_ {
        self.state_update_refs()
            .map(|(k, v)| (k.clone(), v.cloned()))
    }

    pub fn update_total_supply(&mut self, value: u128) {
        self.as_v0_mut().update_total_supply(value);
    }

    pub fn get_write_op(&self, state_key: &StateKey) -> Option<&WriteOp> {
        self.as_v0().get(state_key)
    }

    pub fn get_total_supply(&self) -> Option<u128> {
        self.as_v0().get_total_supply()
    }

    pub fn is_empty(&self) -> bool {
        self.as_v0().is_empty() && self.hotness.is_empty()
    }

    pub fn expect_into_write_op_iter(self) -> impl IntoIterator<Item = (StateKey, WriteOp)> {
        self.into_v0().0.write_set
    }

    pub fn expect_write_op_iter(&self) -> impl Iterator<Item = (&StateKey, &WriteOp)> {
        self.as_v0().0.write_set.iter()
    }

    pub fn write_op_iter(&self) -> impl Iterator<Item = (&StateKey, &WriteOp)> {
        self.as_v0().iter()
    }

    pub fn into_write_op_iter(self) -> impl Iterator<Item = (StateKey, WriteOp)> {
        self.into_v0().into_write_op_iter()
    }

    pub fn base_op_iter(&self) -> impl Iterator<Item = (&StateKey, &BaseStateOp)> {
        self.as_v0()
            .iter()
            .map(|(key, op)| (key, op.as_base_op()))
            .merge_join_by(
                self.hotness.iter().map(|(key, op)| (key, op.as_base_op())),
                |a, b| a.0.cmp(b.0),
            )
            .map(|entry| {
                // It seems like it's possible to have a key that is both in `value` and `hotness`
                // (possibly due to inaccurate read write summary). If this happens we discard the
                // hotness change, since the recently written keys will be made hot anyway.
                match entry {
                    EitherOrBoth::Left(e) | EitherOrBoth::Right(e) => e,
                    EitherOrBoth::Both(e, _) => e,
                }
            })
    }

    pub fn add_hotness(&mut self, hotness: BTreeMap<StateKey, HotStateOp>) {
        assert!(
            self.hotness.is_empty(),
            "hotness should only be initialized once."
        );
        self.hotness = hotness;
    }
}

/// `WriteSet` contains all access paths that one transaction modifies. Each of them is a `WriteOp`
/// where `Value(val)` means that serialized representation should be updated to `val`, and
/// `Deletion` means that we are going to delete this access path.
#[derive(
    BCSCryptoHash, Clone, CryptoHasher, Debug, Default, Eq, PartialEq, Serialize, Deserialize,
)]
pub struct WriteSetV0(WriteSetMut);

impl WriteSetV0 {
    #[inline]
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[inline]
    pub fn iter(&self) -> btree_map::Iter<'_, StateKey, WriteOp> {
        self.0.write_set.iter()
    }

    #[inline]
    pub fn into_write_op_iter(self) -> btree_map::IntoIter<StateKey, WriteOp> {
        self.0.write_set.into_iter()
    }

    pub fn get(&self, key: &StateKey) -> Option<&WriteOp> {
        self.0.get(key)
    }

    fn get_total_supply(&self) -> Option<u128> {
        let value = self
            .0
            .get(&TOTAL_SUPPLY_STATE_KEY)
            .and_then(|op| op.bytes())
            .map(|bytes| bcs::from_bytes::<u128>(bytes));
        value.transpose().map_err(anyhow::Error::msg).unwrap()
    }

    // This is a temporary method to update the total supply in the write set.
    // TODO: get rid of this func() and use WriteSetMut instead; for that we need to change
    //       VM execution such that to 'TransactionOutput' is materialized after updating
    //       total_supply.
    fn update_total_supply(&mut self, value: u128) {
        assert!(self
            .0
            .write_set
            .insert(
                TOTAL_SUPPLY_STATE_KEY.clone(),
                WriteOp::legacy_modification(bcs::to_bytes(&value).unwrap().into())
            )
            .is_some());
    }
}

/// A mutable version of `WriteSet`.
///
/// This is separate because it goes through validation before becoming an immutable `WriteSet`.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct WriteSetMut {
    // TODO: Change to HashMap with a stable iterator for serialization.
    write_set: BTreeMap<StateKey, WriteOp>,
}

impl WriteSetMut {
    pub fn new(write_ops: impl IntoIterator<Item = (StateKey, WriteOp)>) -> Self {
        Self {
            write_set: write_ops.into_iter().collect(),
        }
    }

    pub fn try_new(
        write_ops: impl IntoIterator<Item = Result<(StateKey, WriteOp)>>,
    ) -> Result<Self> {
        Ok(Self {
            write_set: write_ops.into_iter().collect::<Result<_>>()?,
        })
    }

    pub fn insert(&mut self, item: (StateKey, WriteOp)) {
        self.write_set.insert(item.0, item.1);
    }

    pub fn extend(&mut self, write_ops: impl IntoIterator<Item = (StateKey, WriteOp)>) {
        self.write_set.extend(write_ops);
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.write_set.is_empty()
    }

    pub fn len(&self) -> usize {
        self.write_set.len()
    }

    pub fn freeze(self) -> Result<WriteSet> {
        // TODO: add structural validation
        Ok(WriteSet {
            value: ValueWriteSet::V0(WriteSetV0(self)),
            hotness: BTreeMap::new(),
        })
    }

    pub fn get(&self, key: &StateKey) -> Option<&WriteOp> {
        self.write_set.get(key)
    }

    pub fn as_inner_mut(&mut self) -> &mut BTreeMap<StateKey, WriteOp> {
        &mut self.write_set
    }

    pub fn into_inner(self) -> BTreeMap<StateKey, WriteOp> {
        self.write_set
    }

    pub fn squash(mut self, other: Self) -> Result<Self> {
        use btree_map::Entry::*;

        for (key, op) in other.write_set.into_iter() {
            match self.write_set.entry(key) {
                Occupied(mut entry) => {
                    if !WriteOp::squash(entry.get_mut(), op)? {
                        entry.remove();
                    }
                },
                Vacant(entry) => {
                    entry.insert(op);
                },
            }
        }

        Ok(self)
    }
}

impl FromIterator<(StateKey, WriteOp)> for WriteSetMut {
    fn from_iter<I: IntoIterator<Item = (StateKey, WriteOp)>>(iter: I) -> Self {
        let mut ws = WriteSetMut::default();
        for write in iter {
            ws.insert((write.0, write.1));
        }
        ws
    }
}
