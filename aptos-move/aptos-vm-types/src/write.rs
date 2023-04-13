// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::effects::Op;
use anyhow::bail;
use aptos_types::write_set::WriteOp as StorageWriteOp;
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::vm_status::StatusCode;
use move_vm_types::resolver::{Module, ModuleRef, Resource, ResourceRef};

#[derive(Clone, Debug)]
pub enum AptosResource {
    AggregatorValue(u128),
    Standard(Resource),
}

impl AptosResource {
    pub fn as_aggregator_value(&self) -> anyhow::Result<u128> {
        Ok(match self {
            Self::AggregatorValue(v) => *v,
            Self::Standard(r) => match r {
                Resource::Serialized(bytes) => bcs::from_bytes(bytes)?,
                // Aggregator is an extension, so it is produced either as an integer,
                // or as a blob from storage.
                _ => bail!("Aggregator value cannot be stored as a Move value"),
            },
        })
    }

    pub fn into_bytes(self) -> Option<Vec<u8>> {
        match self {
            AptosResource::AggregatorValue(v) => bcs::to_bytes(&v).ok(),
            AptosResource::Standard(r) => r.into_bytes(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum AptosResourceRef {
    AggregatorValue(u128),
    Standard(ResourceRef),
}

#[derive(Clone, Debug)]
pub struct AptosModule(Module);

impl AptosModule {
    pub fn into_bytes(self) -> Option<Vec<u8>> {
        self.0.into_bytes()
    }

    pub fn new(m: Module) -> Self {
        Self(m)
    }
}

#[derive(Clone, Debug)]
pub struct AptosModuleRef(ModuleRef);

pub trait TransactionWrite {}
impl TransactionWrite for AptosResource {}
impl TransactionWrite for AptosModule {}

/// Represents any write operation produced by the VM.
#[derive(Clone, Debug)]
pub enum WriteOp {
    ResourceWrite(Op<AptosResource>),
    ModuleWrite(Op<AptosModule>),
}

impl WriteOp {
    pub fn is_deletion(&self) -> bool {
        match self {
            Self::ResourceWrite(Op::Deletion) | Self::ModuleWrite(Op::Deletion) => true,
            _ => false,
        }
    }

    /// Reinterprets this write as a module write. The called must ensure that
    /// the type of the write beforehand, e.g. by checking the state key.
    pub fn into_module_write(self) -> Op<AptosModule> {
        match self {
            Self::ResourceWrite(_) => unreachable!(),
            Self::ModuleWrite(w) => w,
        }
    }

    /// Reinterprets this write as a resource write. The called must ensure that
    /// the type of the write beforehand, e.g. by checking the state key.
    pub fn into_resource_write(self) -> Op<AptosResource> {
        match self {
            Self::ResourceWrite(w) => w,
            Self::ModuleWrite(_) => unreachable!(),
        }
    }

    /// Converts this write into storage-friendly write op which contains the serialized
    /// version of the data.
    pub fn into_write_op(self) -> PartialVMResult<StorageWriteOp> {
        let err = PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR).with_message("Failed to serialize module or resource".to_string());

        // TODO: Remove code duplication.
        Ok(match self {
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

pub trait TransactionRef {}
impl TransactionWrite for AptosResourceRef {}
impl TransactionWrite for AptosModuleRef {}

impl From<&AptosResource> for AptosResourceRef {
    fn from(ar: &AptosResource) -> Self {
        match ar {
            AptosResource::AggregatorValue(v) => Self::AggregatorValue(*v),
            AptosResource::Standard(r) => Self::Standard(ResourceRef::new(r.clone())),
        }
    }
}

impl From<&AptosModule> for AptosModuleRef {
    fn from(am: &AptosModule) -> Self {
        Self(ModuleRef::new(am.0.clone()))
    }
}
