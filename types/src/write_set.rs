// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! For each transaction the VM executes, the VM will output a `WriteSet` that contains each access
//! path it updates. For each access path, the VM can either give its new value or delete it.

use crate::state_store::state_key::StateKey;
use anyhow::Result;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use serde::{Deserialize, Serialize};

#[derive(Clone, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum WriteOp {
    Creation(#[serde(with = "serde_bytes")] Vec<u8>),
    Modification(#[serde(with = "serde_bytes")] Vec<u8>),
    Deletion,
}

impl WriteOp {
    #[inline]
    pub fn is_deletion(&self) -> bool {
        match self {
            WriteOp::Deletion => true,
            WriteOp::Modification(_) | WriteOp::Creation(_) => false,
        }
    }

    pub fn is_creation(&self) -> bool {
        match self {
            WriteOp::Creation(_) => true,
            WriteOp::Modification(_) | WriteOp::Deletion => false,
        }
    }

    pub fn is_modification(&self) -> bool {
        match self {
            WriteOp::Modification(_) => true,
            WriteOp::Creation(_) | WriteOp::Deletion => false,
        }
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
        }
    }
}

/// `WriteSet` contains all access paths that one transaction modifies. Each of them is a `WriteOp`
/// where `Value(val)` means that serialized representation should be updated to `val`, and
/// `Deletion` means that we are going to delete this access path.
#[derive(
    BCSCryptoHash, Clone, CryptoHasher, Debug, Default, Eq, Hash, PartialEq, Serialize, Deserialize,
)]
pub struct WriteSet(WriteSetMut);

impl WriteSet {
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[inline]
    pub fn iter(&self) -> ::std::slice::Iter<'_, (StateKey, WriteOp)> {
        self.into_iter()
    }

    #[inline]
    pub fn into_mut(self) -> WriteSetMut {
        self.0
    }
}

/// A mutable version of `WriteSet`.
///
/// This is separate because it goes through validation before becoming an immutable `WriteSet`.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct WriteSetMut {
    write_set: Vec<(StateKey, WriteOp)>,
}

impl WriteSetMut {
    pub fn new(write_set: Vec<(StateKey, WriteOp)>) -> Self {
        Self { write_set }
    }

    pub fn push(&mut self, item: (StateKey, WriteOp)) {
        self.write_set.push(item);
    }

    pub fn append(&mut self, other: &mut Self) {
        self.write_set.append(&mut other.write_set);
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.write_set.is_empty()
    }

    pub fn freeze(self) -> Result<WriteSet> {
        // TODO: add structural validation
        Ok(WriteSet(self))
    }
}

impl ::std::iter::FromIterator<(StateKey, WriteOp)> for WriteSetMut {
    fn from_iter<I: IntoIterator<Item = (StateKey, WriteOp)>>(iter: I) -> Self {
        let mut ws = WriteSetMut::default();
        for write in iter {
            ws.push((write.0, write.1));
        }
        ws
    }
}

impl<'a> IntoIterator for &'a WriteSet {
    type Item = &'a (StateKey, WriteOp);
    type IntoIter = ::std::slice::Iter<'a, (StateKey, WriteOp)>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.write_set.iter()
    }
}

impl ::std::iter::IntoIterator for WriteSet {
    type Item = (StateKey, WriteOp);
    type IntoIter = ::std::vec::IntoIter<(StateKey, WriteOp)>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.write_set.into_iter()
    }
}
