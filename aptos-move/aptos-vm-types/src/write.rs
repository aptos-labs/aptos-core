// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::effects::Op;
use aptos_types::write_set::WriteOp as StorageWriteOp;
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::vm_status::StatusCode;
use move_vm_types::resolver::{Module, ModuleRef, Resource, ResourceRef};

/// Represents any write operation produced by the VM.
#[derive(Clone, Debug)]
pub enum WriteOp {
    AggregatorWrite(Option<u128>),
    ResourceWrite(Op<Resource>),
    ModuleWrite(Op<Module>),
}

/// Represents any write operation reference produced by the VM.
#[derive(Clone, Debug)]
pub enum WriteOpRef {
    AggregatorWrite(Option<u128>),
    ResourceWrite(Op<ResourceRef>),
    ModuleWrite(Op<ModuleRef>),
}

impl From<WriteOp> for WriteOpRef {
    fn from(value: WriteOp) -> Self {
        match value {
            WriteOp::AggregatorWrite(w) => WriteOpRef::AggregatorWrite(w),
            WriteOp::ResourceWrite(w) => WriteOpRef::ResourceWrite(w.map(ResourceRef::new)),
            WriteOp::ModuleWrite(w) => WriteOpRef::ModuleWrite(w.map(ModuleRef::new)),
        }
    }
}

pub trait TransactionWriteRef {
    fn as_aggregator_value(&self) -> Option<u128>;
    fn into_module_ref(self) -> Option<ModuleRef>;
    fn into_resource_ref(self) -> Option<ResourceRef>;
}

impl TransactionWriteRef for WriteOpRef {
    fn as_aggregator_value(&self) -> Option<u128> {
        match self {
            Self::AggregatorWrite(w) => w.clone(),
            _ => unreachable!(),
        }
    }

    fn into_module_ref(self) -> Option<ModuleRef> {
        match self {
            Self::ModuleWrite(op) => match op {
                Op::Creation(m) | Op::Modification(m) => Some(m),
                Op::Deletion => None,
            },
            _ => unreachable!(),
        }
    }

    fn into_resource_ref(self) -> Option<ResourceRef> {
        match self {
            Self::ResourceWrite(op) => match op {
                Op::Creation(m) | Op::Modification(m) => Some(m),
                Op::Deletion => None,
            },
            _ => unreachable!(),
        }
    }
}

impl WriteOp {
    pub fn is_deletion(&self) -> bool {
        match self {
            Self::AggregatorWrite(None)
            | Self::ResourceWrite(Op::Deletion)
            | Self::ModuleWrite(Op::Deletion) => true,
            _ => false,
        }
    }

    /// Reinterprets this write as a module write. The called must ensure that
    /// the type of the write beforehand, e.g. by checking the state key.
    pub fn into_module_write(self) -> Op<Module> {
        match self {
            Self::ModuleWrite(w) => w,
            _ => unreachable!(),
        }
    }

    pub fn into_aggregator_write(self) -> Option<u128> {
        match self {
            Self::AggregatorWrite(w) => w,
            _ => unreachable!(),
        }
    }

    /// Reinterprets this write as a resource write. The called must ensure that
    /// the type of the write beforehand, e.g. by checking the state key.
    pub fn into_resource_write(self) -> Op<Resource> {
        match self {
            Self::ResourceWrite(w) => w,
            _ => unreachable!(),
        }
    }

    /// Converts this write into storage-friendly write op which contains the serialized
    /// version of the data.
    pub fn into_write_op(self) -> PartialVMResult<StorageWriteOp> {
        let err = PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
            .with_message("Failed to serialize module, resource or aggregator value".to_string());

        // TODO: Remove code duplication.
        Ok(match self {
            Self::AggregatorWrite(aw) => match aw {
                Some(v) => {
                    let bytes = bcs::to_bytes(&v).map_err(|_| err)?;
                    StorageWriteOp::Modification(bytes)
                },
                None => StorageWriteOp::Deletion,
            },
            Self::ResourceWrite(rw) => match rw {
                Op::Creation(r) => StorageWriteOp::Creation(r.into_bytes().ok_or(err)?),
                Op::Modification(r) => StorageWriteOp::Modification(r.into_bytes().ok_or(err)?),
                Op::Deletion => StorageWriteOp::Deletion,
            },
            Self::ModuleWrite(mw) => match mw {
                Op::Creation(r) => StorageWriteOp::Creation(r.into_bytes().ok_or(err)?),
                Op::Modification(r) => StorageWriteOp::Modification(r.into_bytes().ok_or(err)?),
                Op::Deletion => StorageWriteOp::Deletion,
            },
        })
    }
}

/// Squashes two writes together. If the result of the squash is a no-op, returns `false`.
pub fn squash_writes(write: &mut WriteOp, other_write: WriteOp) -> anyhow::Result<bool> {
    match (write, other_write) {
        (WriteOp::ModuleWrite(w1), WriteOp::ModuleWrite(w2)) => w1.squash(w2),
        (WriteOp::ResourceWrite(w1), WriteOp::ResourceWrite(w2)) => w1.squash(w2),
        _ => unreachable!("Squashing modules with resources is not possible"),
    }
}
