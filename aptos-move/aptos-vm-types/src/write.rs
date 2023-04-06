// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::{
    state_store::state_value::StateValue,
    write_set::{TransactionWrite, WriteOp},
};
use move_core_types::{language_storage::StructTag, vm_status::StatusCode};
use move_vm_types::{
    natives::function::{PartialVMError, PartialVMResult},
    resolver::Resource,
};
use std::{
    collections::BTreeMap,
    fmt::{Debug, Formatter, Result},
};

/// Represents any write produced by a transaction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AptosWrite {
    AggregatorValue(u128),
    Module(Vec<u8>),
    Standard(Resource),
    Group(BTreeMap<StructTag, Resource>),
}

impl AptosWrite {
    pub fn as_bytes(&self) -> Option<Vec<u8>> {
        match self {
            AptosWrite::AggregatorValue(value) => bcs::to_bytes(value).ok(),
            AptosWrite::Module(blob) => Some(blob.clone()),
            AptosWrite::Standard(resource) => resource.serialize(),
            AptosWrite::Group(wgroup) => {
                // TODO: Fix this!
                // let serialized_group: BTreeMap<StructTag, Option<Vec<u8>>> = group.clone().into_iter().map(|(tag, r)| (tag, &r.serialize())).collect();
                // bcs::to_bytes(&serialized_group).ok()
                None
            },
        }
    }

    pub fn as_aggregator_value(&self) -> PartialVMResult<u128> {
        match self {
            AptosWrite::AggregatorValue(value) => Ok(*value),
            _ => Err(PartialVMError::new(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
            )),
        }
    }
}

/// This trait helps to treat writes of values or resources as writes of bytes and helps
/// with writing generic code for the executor.
pub trait AsBytes {
    fn as_bytes(&self) -> Option<Vec<u8>>;
}

impl AsBytes for AptosWrite {
    fn as_bytes(&self) -> Option<Vec<u8>> {
        self.as_bytes()
    }
}

impl AsBytes for Op<AptosWrite> {
    fn as_bytes(&self) -> Option<Vec<u8>> {
        match self {
            Op::Creation(write) => write.as_bytes(),
            Op::Modification(write) => write.as_bytes(),
            Op::Deletion => None,
        }
    }
}

// TODO: use as bytes instead!
impl<T: Debug + AsBytes> TransactionWrite for Op<T> {
    fn extract_raw_bytes(&self) -> Option<Vec<u8>> {
        match self {
            Op::Creation(write) => write.as_bytes(),
            Op::Modification(write) => write.as_bytes(),
            Op::Deletion => None,
        }
    }

    fn as_state_value(&self) -> Option<StateValue> {
        match self {
            Op::Creation(write) => write.as_bytes().map(|bytes| StateValue::new_legacy(bytes)),
            Op::Modification(write) => write.as_bytes().map(|bytes| StateValue::new_legacy(bytes)),
            Op::Deletion => None,
        }
    }
}

/// Represents a write op at the VM level.
#[derive(Clone, PartialEq, Eq)]
pub enum Op<T: Debug + AsBytes> {
    Creation(T),
    Modification(T),
    Deletion,
}

impl<T: Debug + AsBytes> Op<T> {
    /// Converts this op into a write op which can be used by the storage.
    pub fn into_write_op(self) -> PartialVMResult<WriteOp> {
        match self {
            Op::Creation(value) => value
                .as_bytes()
                .map(WriteOp::Creation)
                .ok_or(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)),
            Op::Modification(value) => value
                .as_bytes()
                .map(WriteOp::Modification)
                .ok_or(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)),
            Op::Deletion => Ok(WriteOp::Deletion),
        }
    }

    pub fn squash(&mut self, other_op: Self) -> anyhow::Result<bool> {
        match (&self, other_op) {
            (Op::Modification(_) | Op::Creation(_), Op::Creation(_)) // create existing
            | (Op::Deletion, Op::Deletion | Op::Modification(_)) // delete or modify already deleted
            => {
                anyhow::bail!(
                    "Ops cannot be squashed",
                )
            },
            (Op::Modification(_), Op::Modification(data)) => *self= Op::Modification(data),
            (Op::Creation(_), Op::Modification(data)) => {
                *self = Op::Creation(data)
            },
            (Op::Modification(_) , Op::Deletion) => {
                *self = Op::Deletion
            },
            (Op::Deletion, Op::Creation(data)) => {
                *self = Op::Modification(data)
            },
            (Op::Creation(_), Op::Deletion) => {
                return Ok(false)
            },
        }
        Ok(true)
    }
}

impl<T: Debug + AsBytes> Debug for Op<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Op::Modification(value) => write!(f, "Modification({:?})", value),
            Op::Creation(value) => write!(f, "Creation({:?})", value),
            Op::Deletion => write!(f, "Deletion"),
        }
    }
}
