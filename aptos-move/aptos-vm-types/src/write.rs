// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::effects::Op;
use aptos_types::write_set::WriteOp as FinalWriteOp;
use move_binary_format::errors::PartialVMResult;
use move_vm_types::resolver::{Module, ModuleRef, Resource, ResourceRef};

#[derive(Clone, Debug)]
pub enum AptosResource {
    AggregatorValue(u128),
    Standard(Resource),
}

#[derive(Clone, Debug)]
pub enum AptosResourceRef {
    AggregatorValue(u128),
    Standard(ResourceRef),
}

#[derive(Clone, Debug)]
pub struct AptosModule(Module);

#[derive(Clone, Debug)]
pub struct AptosModuleRef(ModuleRef);

pub trait TransactionWrite {}
impl TransactionWrite for AptosResource {}
impl TransactionWrite for AptosModule {}

// Change!
#[derive(Clone, Debug)]
pub struct WriteOp;

impl WriteOp {
    pub fn into_write_op(self) -> PartialVMResult<FinalWriteOp> {
        Ok(FinalWriteOp::Creation(vec![]))
    }

    pub fn squash(&mut self, o: WriteOp) -> PartialVMResult<bool> {
        Ok(false)
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
