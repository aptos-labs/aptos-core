// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    state_store::state_value::{StateValue, StateValueMetadata},
    write_set::{WriteOp, WriteOpSize},
};
use move_binary_format::{errors::PartialVMResult, CompiledModule};
use sha3::{Digest, Sha3_256};
use std::sync::Arc;

/// In addition to a basic write op, carries non-serialized module.
/// Note that this is not at all redundant: when a module is published,
/// it is deserialized, and so we can capture it here for later caching.
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct ModuleWriteOp {
    // Actual write which will be persisted to storage as part of transaction output.
    pub write_op: WriteOp,
    // Deserialized module which will be cached, if needed.
    pub module: Arc<CompiledModule>,
}

impl ModuleWriteOp {
    pub fn write_op_size(&self) -> WriteOpSize {
        use WriteOp::*;
        match self.write_op {
            Creation { .. } => WriteOpSize::Creation {
                write_len: self.write_op.size() as u64,
            },
            Modification { .. } => WriteOpSize::Modification {
                write_len: self.write_op.size() as u64,
            },

            // TODO: Consider properly gating this to not run into panic accidentally.
            Deletion { .. } => unreachable!("Modules cannot be deleted"),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct OnChainUnverifiedModule {
    pub module: Arc<CompiledModule>,

    pub hash: [u8; 32],
    pub num_bytes: usize,

    pub state_value_metadata: StateValueMetadata,
}

impl OnChainUnverifiedModule {
    pub fn from_state_value(state_value: StateValue) -> PartialVMResult<Self> {
        let (state_value_metadata, bytes) = state_value.unpack();

        let mut hash = Sha3_256::new();
        hash.update(&bytes);
        let hash: [u8; 32] = hash.finalize().into();

        let num_bytes = bytes.len();
        // TODO: This function should also take a deserializer config, which the caller
        //       must cache and pass here!
        let module = Arc::new(CompiledModule::deserialize(&bytes)?);

        Ok(Self {
            module,
            hash,
            num_bytes,
            state_value_metadata,
        })
    }

    pub fn from_module_write(module_write_op: ModuleWriteOp) -> Self {
        // TODO: Consider properly gating this to not run into panic accidentally.
        //       One way is to store something other than WriteOp!
        let bytes = module_write_op
            .write_op
            .bytes()
            .expect("Modules cannot be deleted");
        let num_bytes = module_write_op.write_op.size();

        let mut hash = Sha3_256::new();
        hash.update(bytes);
        let hash: [u8; 32] = hash.finalize().into();

        let module = module_write_op.module;
        let state_value_metadata = module_write_op.write_op.into_metadata();

        Self {
            module,
            hash,
            num_bytes,
            state_value_metadata,
        }
    }
}
