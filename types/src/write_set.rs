// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! For each transaction the VM executes, the VM will output a `WriteSet` that contains each access
//! path it updates. For each access path, the VM can either give its new value or delete it. For
//! aggregator, delta updates are used (note: this is a temporary solution and ideally we should
//! modify `ChangeSet` and `TransactionOutput` to keep deltas internal to executor).

use crate::state_store::state_key::StateKey;
use anyhow::Result;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use serde::{Deserialize, Serialize};

/// Specifies partial function such as +X or -X to use with `WriteOp::Delta`.
#[derive(Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub enum DeltaOperation {
    Addition(u128),
    Subtraction(u128),
}

impl std::fmt::Debug for DeltaOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeltaOperation::Addition(value) => write!(f, "+{}", value),
            DeltaOperation::Subtraction(value) => write!(f, "-{}", value),
        }
    }
}

#[derive(Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct DeltaLimit(pub u128);

impl std::fmt::Debug for DeltaLimit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "result <= {}", self.0)
    }
}

#[derive(Clone, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum WriteOp {
    Deletion,
    Value(#[serde(with = "serde_bytes")] Vec<u8>),
    #[serde(skip)]
    Delta(DeltaOperation, DeltaLimit),
}

impl WriteOp {
    #[inline]
    pub fn is_deletion(&self) -> bool {
        match self {
            WriteOp::Deletion => true,
            WriteOp::Delta(..) => false,
            WriteOp::Value(_) => false,
        }
    }
}

impl std::fmt::Debug for WriteOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WriteOp::Value(value) => write!(
                f,
                "Value({})",
                value
                    .iter()
                    .map(|byte| format!("{:02x}", byte))
                    .collect::<String>()
            ),
            WriteOp::Delta(op, limit) => {
                write!(f, "Delta({:?} ensures {:?})", op, limit)
            }
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
