// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;
use std::fmt::{Debug, Formatter, Result};
use move_core_types::language_storage::StructTag;
use move_core_types::vm_status::StatusCode;
use move_vm_types::natives::function::{PartialVMError, PartialVMResult};
use move_vm_types::resolver::Resource;
use crate::write_set::WriteOp;


/// Represents any write produced by a transaction.
#[derive(Copy, Clone, Debug)]
pub enum AptosWrite {
    AggregatorValue(u128),
    Standard(Resource),
    Group(BTreeMap<StructTag, Resource>)
}

impl AptosWrite {
    /// Serializes the write.
    pub fn as_bytes(&self) -> Option<Vec<u8>> {
        match self {
            AptosWrite::AggregatorValue(value) => bcs::to_bytes(value).ok(),
            AptosWrite::Standard(resource) => resource.serialize(),
            AptosWrite::Group(map) => bcs::to_bytes(map),
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

/// Represents a write op at the VM level.
#[derive(Copy, Clone, Debug)]
pub enum Op<T: Debug + AsBytes> {
    Creation(T),
    Modification(T),
    Deletion,
}

impl<T: AsBytes> Op<T> {
    /// Converts this op into a write op which can be used by the storage.
    pub fn into_write_op(self) -> PartialVMResult<WriteOp> {
        match self {
            Op::Creation(value) => value.as_bytes().map(WriteOp::Creation).ok_or(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)),
            Op::Modification(value) => value.as_bytes().map(WriteOp::Modification).ok_or(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)),
            Op::Deletion => Ok(WriteOp::Deletion)
        }
    }
}

impl<T: Debug> Debug for Op<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Op::Modification(value) => write!(f, "Modification({:?})", value),
            Op::Creation(value) => write!(f, "Creation({:?})", value),
            Op::Deletion => write!(f, "Deletion"),
        }
    }
}
