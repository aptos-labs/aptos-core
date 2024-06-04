// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    state_store::state_value::{StateValue, StateValueMetadata},
    write_set::{WriteOp, WriteOpSize},
};
use anyhow::bail;
use bytes::Bytes;
use move_binary_format::{
    deserializer::DeserializerConfig,
    errors::{PartialVMError, PartialVMResult},
    CompiledModule,
};
use move_core_types::vm_status::StatusCode;
use sha3::{Digest, Sha3_256};
use std::sync::Arc;

/// In addition to a basic write op information, carries non-serialized module.
/// Note that this is not at all redundant: when a module is published, it is
/// deserialized, and so we can capture it here for later caching.
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct ModuleWriteOp {
    // Modules cannot be deleted, so they can be:
    //   - created, or
    //   - modified (upgraded).
    is_creation: bool,
    // Metadata in order to create a write op which is persisted later in storage.
    state_value_metadata: StateValueMetadata,
    // Serialized and non-serialized representations.
    module: Arc<CompiledModule>,
    serialized_module: Bytes,
}

impl ModuleWriteOp {
    pub fn new(
        is_creation: bool,
        state_value_metadata: StateValueMetadata,
        module: Arc<CompiledModule>,
        serialized_module: Bytes,
    ) -> Self {
        Self {
            is_creation,
            state_value_metadata,
            module,
            serialized_module,
        }
    }

    /// Tries to construct a module write from its storage representation. Fails
    /// if 1) storage write is a deletion (invariant violation) or 2) module
    /// deserialization fails.
    pub fn from_write_op(
        write_op: WriteOp,
        deserializer_config: &DeserializerConfig,
    ) -> PartialVMResult<Self> {
        use WriteOp::*;
        let (is_creation, state_value_metadata, serialized_module) = match write_op {
            Creation { data, metadata } => (true, metadata, data),
            Modification { data, metadata } => (false, metadata, data),
            Deletion { .. } => {
                return Err(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message("Module write cannot be a deletion".to_string()),
                )
            },
        };

        let module = Arc::new(CompiledModule::deserialize_with_config(
            &serialized_module,
            deserializer_config,
        )?);
        Ok(Self {
            is_creation,
            state_value_metadata,
            module,
            serialized_module,
        })
    }

    /// Squashes a pair of write ops, and returns an error if squash was not
    /// successful. Because modules cannot be deleted, squash is never a no-op.
    pub fn squash(op: &mut Self, other_op: Self) -> anyhow::Result<()> {
        // We cannot create something that has been already created or modified.
        if other_op.is_creation {
            bail!("Creation cannot happen twice")
        }

        // Creation followed by modification is creation, and modification followed
        // by modification is modification, so there is no need to update the flag.
        WriteOp::ensure_metadata_compatible(
            &op.state_value_metadata,
            &other_op.state_value_metadata,
        )?;
        op.state_value_metadata = other_op.state_value_metadata;
        op.module = other_op.module;
        op.serialized_module = other_op.serialized_module;
        Ok(())
    }

    pub fn write_op_size(&self) -> WriteOpSize {
        let write_len = self.serialized_module.len() as u64;
        if self.is_creation {
            WriteOpSize::Creation { write_len }
        } else {
            WriteOpSize::Modification { write_len }
        }
    }

    pub fn get_metadata_mut(&mut self) -> &mut StateValueMetadata {
        &mut self.state_value_metadata
    }

    pub fn into_write_op(self) -> WriteOp {
        let data = self.serialized_module;
        let metadata = self.state_value_metadata;
        if self.is_creation {
            WriteOp::Creation { data, metadata }
        } else {
            WriteOp::Modification { data, metadata }
        }
    }
}

/// A representation of an entry in code cache, which stores information about
/// the module such as its size in bytes or its hash.
/// TODO: This representation is not stable and will evolve, in particular
///   - modules will become verified,
///   - hashes most likely disappear,
///   - ...
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct OnChainUnverifiedModule {
    pub module: Arc<CompiledModule>,

    pub hash: [u8; 32],
    pub num_bytes: usize,

    pub state_value_metadata: StateValueMetadata,
}

impl OnChainUnverifiedModule {
    pub fn from_state_value(
        state_value: StateValue,
        deserializer_config: &DeserializerConfig,
    ) -> PartialVMResult<Self> {
        let (state_value_metadata, bytes) = state_value.unpack();

        let mut hash = Sha3_256::new();
        hash.update(&bytes);
        let hash: [u8; 32] = hash.finalize().into();

        let num_bytes = bytes.len();
        let module = Arc::new(CompiledModule::deserialize_with_config(
            &bytes,
            deserializer_config,
        )?);

        Ok(Self {
            module,
            hash,
            num_bytes,
            state_value_metadata,
        })
    }

    pub fn from_module_write(module_write_op: ModuleWriteOp) -> Self {
        let ModuleWriteOp {
            state_value_metadata,
            module,
            serialized_module,
            ..
        } = module_write_op;
        let mut hash = Sha3_256::new();
        hash.update(&serialized_module);
        let hash: [u8; 32] = hash.finalize().into();

        let num_bytes = serialized_module.len();
        Self {
            module,
            hash,
            num_bytes,
            state_value_metadata,
        }
    }
}
