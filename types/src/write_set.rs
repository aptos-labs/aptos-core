// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! For each transaction the VM executes, the VM will output a `WriteSet` that contains each access
//! path it updates. For each access path, the VM can either give its new value or delete it.

use crate::state_store::{
    state_key::StateKey,
    state_value::{StateValue, StateValueMetadata},
};
use anyhow::{bail, Result};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use serde::{Deserialize, Serialize};
use std::{
    collections::{btree_map, BTreeMap},
    ops::Deref,
};

#[derive(Clone, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum WriteOp {
    Creation(#[serde(with = "serde_bytes")] Vec<u8>),
    Modification(#[serde(with = "serde_bytes")] Vec<u8>),
    Deletion,
    CreationWithMetadata {
        #[serde(with = "serde_bytes")]
        data: Vec<u8>,
        metadata: StateValueMetadata,
    },
    ModificationWithMetadata {
        #[serde(with = "serde_bytes")]
        data: Vec<u8>,
        metadata: StateValueMetadata,
    },
    DeletionWithMetadata {
        metadata: StateValueMetadata,
    },
}

impl WriteOp {
    #[inline]
    pub fn is_deletion(&self) -> bool {
        match self {
            WriteOp::Deletion | WriteOp::DeletionWithMetadata { .. } => true,
            WriteOp::Modification(_)
            | WriteOp::ModificationWithMetadata { .. }
            | WriteOp::Creation(_)
            | WriteOp::CreationWithMetadata { .. } => false,
        }
    }

    pub fn is_creation(&self) -> bool {
        match self {
            WriteOp::Creation(_) | WriteOp::CreationWithMetadata { .. } => true,
            WriteOp::Modification(_)
            | WriteOp::ModificationWithMetadata { .. }
            | WriteOp::Deletion
            | WriteOp::DeletionWithMetadata { .. } => false,
        }
    }

    pub fn is_modification(&self) -> bool {
        match self {
            WriteOp::Modification(_) | WriteOp::ModificationWithMetadata { .. } => true,
            WriteOp::Creation(_)
            | WriteOp::CreationWithMetadata { .. }
            | WriteOp::Deletion
            | WriteOp::DeletionWithMetadata { .. } => false,
        }
    }

    /// Merges two write ops on the same state item.
    ///
    /// returns `false` if the result indicates no op has happened -- that's when the first op
    ///   creates the item and the second deletes it.
    pub fn squash(op: &mut Self, other: Self) -> Result<bool> {
        use WriteOp::*;

        // n.b. With write sets from multiple sessions being squashed together, it's possible
        //   to see two ops carrying different metadata (or one with it the other without)
        //   due to deleting in one session and recreating in another. The original metadata
        //   shouldn't change due to the squash.
        // And because the deposit or refund happens after all squashing is finished, it's
        // not a concern of fairness.

        match (&op, other) {
            (
                Modification(_)
                | ModificationWithMetadata { .. }
                | Creation(_)
                | CreationWithMetadata { .. },
                Creation(_) | CreationWithMetadata {..},
            ) // create existing
            | (
                Deletion | DeletionWithMetadata {..},
                Deletion | DeletionWithMetadata {..} | Modification(_) | ModificationWithMetadata { .. },
            ) // delete or modify already deleted
            => {
                bail!(
                    "The given change sets cannot be squashed",
                )
            },
            (Modification(_), Modification(data) | ModificationWithMetadata {data, ..}) => *op = Modification(data),
            (ModificationWithMetadata{metadata, ..}, Modification(data) | ModificationWithMetadata{data, ..}) => {
                *op = ModificationWithMetadata{data, metadata: metadata.clone()}
            },
            (Creation(_), Modification(data) | ModificationWithMetadata {data, ..} ) => {
                *op = Creation(data)
            },
            (CreationWithMetadata{metadata , ..}, Modification(data) | ModificationWithMetadata{data, ..}) => {
                *op = CreationWithMetadata{data, metadata: metadata.clone()}
            },
            (Modification(_) , Deletion | DeletionWithMetadata {..}) => {
                *op = Deletion
            },
            (ModificationWithMetadata{metadata, ..} , Deletion | DeletionWithMetadata {..}) => {
                *op = DeletionWithMetadata {metadata: metadata.clone()}
            },
            (Deletion, Creation(data) | CreationWithMetadata {data, ..}) => {
                *op = Modification(data)
            },
            (DeletionWithMetadata {metadata, ..}, Creation(data)| CreationWithMetadata {data, ..}) => {
                *op = ModificationWithMetadata{data, metadata: metadata.clone()}
            },
            (Creation(_) | CreationWithMetadata {..}, Deletion | DeletionWithMetadata {..}) => {
                return Ok(false)
            },
        }
        Ok(true)
    }

    pub fn into_bytes(self) -> Option<Vec<u8>> {
        use WriteOp::*;

        match self {
            Creation(data)
            | CreationWithMetadata { data, .. }
            | Modification(data)
            | ModificationWithMetadata { data, .. } => Some(data),
            Deletion | DeletionWithMetadata { .. } => None,
        }
    }

    pub fn bytes(&self) -> Option<&[u8]> {
        use WriteOp::*;

        match self {
            Creation(data)
            | CreationWithMetadata { data, .. }
            | Modification(data)
            | ModificationWithMetadata { data, .. } => Some(data),
            Deletion | DeletionWithMetadata { .. } => None,
        }
    }

    pub fn metadata(&self) -> Option<&StateValueMetadata> {
        use WriteOp::*;

        match self {
            Creation(_) | Modification(_) | Deletion => None,
            CreationWithMetadata { metadata, .. }
            | ModificationWithMetadata { metadata, .. }
            | DeletionWithMetadata { metadata, .. } => Some(metadata),
        }
    }
}

pub trait TransactionWrite {
    fn extract_raw_bytes(&self) -> Option<Vec<u8>>;

    fn as_state_value(&self) -> Option<StateValue>;
}

impl TransactionWrite for WriteOp {
    fn extract_raw_bytes(&self) -> Option<Vec<u8>> {
        self.clone().into_bytes()
    }

    fn as_state_value(&self) -> Option<StateValue> {
        self.bytes().map(|bytes| match self.metadata() {
            None => StateValue::new_legacy(bytes.to_vec()),
            Some(metadata) => StateValue::new_with_metadata(bytes.to_vec(), metadata.clone()),
        })
    }
}

impl std::fmt::Debug for WriteOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WriteOp::Modification(value) => write!(
                f,
                "Modification({})",
                value
                    .iter()
                    .map(|byte| format!("{:02x}", byte))
                    .collect::<String>()
            ),
            WriteOp::Creation(value) => write!(
                f,
                "Creation({})",
                value
                    .iter()
                    .map(|byte| format!("{:02x}", byte))
                    .collect::<String>()
            ),
            WriteOp::Deletion => write!(f, "Deletion"),
            WriteOp::CreationWithMetadata { data, metadata } => write!(
                f,
                "CreationWithMetadata({}, metadata:{:?})",
                data.iter()
                    .map(|byte| format!("{:02x}", byte))
                    .collect::<String>(),
                metadata,
            ),
            WriteOp::ModificationWithMetadata { data, metadata } => write!(
                f,
                "ModificationWithMetadata({}, metadata:{:?})",
                data.iter()
                    .map(|byte| format!("{:02x}", byte))
                    .collect::<String>(),
                metadata,
            ),
            WriteOp::DeletionWithMetadata { metadata } => {
                write!(f, "DeletionWithMetadata(metadata:{:?})", metadata,)
            },
        }
    }
}

#[derive(
    BCSCryptoHash, Clone, CryptoHasher, Debug, Eq, Hash, PartialEq, Serialize, Deserialize,
)]
pub enum WriteSet {
    V0(WriteSetV0),
}

impl Default for WriteSet {
    fn default() -> Self {
        Self::V0(WriteSetV0::default())
    }
}

impl WriteSet {
    pub fn into_mut(self) -> WriteSetMut {
        match self {
            Self::V0(write_set) => write_set.0,
        }
    }
}

impl Deref for WriteSet {
    type Target = WriteSetV0;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::V0(write_set) => write_set,
        }
    }
}

/// `WriteSet` contains all access paths that one transaction modifies. Each of them is a `WriteOp`
/// where `Value(val)` means that serialized representation should be updated to `val`, and
/// `Deletion` means that we are going to delete this access path.
#[derive(
    BCSCryptoHash, Clone, CryptoHasher, Debug, Default, Eq, Hash, PartialEq, Serialize, Deserialize,
)]
pub struct WriteSetV0(WriteSetMut);

impl WriteSetV0 {
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[inline]
    pub fn iter(&self) -> btree_map::Iter<'_, StateKey, WriteOp> {
        self.0.write_set.iter()
    }

    pub fn get(&self, key: &StateKey) -> Option<&WriteOp> {
        self.0.get(key)
    }
}

/// A mutable version of `WriteSet`.
///
/// This is separate because it goes through validation before becoming an immutable `WriteSet`.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq, Serialize, Deserialize)]
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
        Ok(WriteSet::V0(WriteSetV0(self)))
    }

    pub fn get(&self, key: &StateKey) -> Option<&WriteOp> {
        self.write_set.get(key)
    }

    pub fn as_inner_mut(&mut self) -> &mut BTreeMap<StateKey, WriteOp> {
        &mut self.write_set
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

impl ::std::iter::FromIterator<(StateKey, WriteOp)> for WriteSetMut {
    fn from_iter<I: IntoIterator<Item = (StateKey, WriteOp)>>(iter: I) -> Self {
        let mut ws = WriteSetMut::default();
        for write in iter {
            ws.insert((write.0, write.1));
        }
        ws
    }
}

impl<'a> IntoIterator for &'a WriteSet {
    type IntoIter = btree_map::Iter<'a, StateKey, WriteOp>;
    type Item = (&'a StateKey, &'a WriteOp);

    fn into_iter(self) -> Self::IntoIter {
        match self {
            WriteSet::V0(write_set) => write_set.0.write_set.iter(),
        }
    }
}

impl ::std::iter::IntoIterator for WriteSet {
    type IntoIter = btree_map::IntoIter<StateKey, WriteOp>;
    type Item = (StateKey, WriteOp);

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Self::V0(write_set) => write_set.0.write_set.into_iter(),
        }
    }
}
