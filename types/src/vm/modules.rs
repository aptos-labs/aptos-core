// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    delayed_fields::PanicError,
    state_store::state_value::{StateValue, StateValueMetadata},
    write_set::TransactionWrite,
};
use bytes::Bytes;
use move_binary_format::{
    errors::{Location, PartialVMError, VMResult},
    CompiledModule,
};
use move_core_types::{metadata::Metadata, vm_status::StatusCode};
use move_vm_runtime::{Module, RuntimeEnvironment};
use std::sync::Arc;

/// Different kinds of representation a module can be in. The code cache
/// can implement different policies for promoting the representation from
/// one to the other.
#[derive(Debug)]
enum Representation {
    /// A simple deserialized representation with a non-verified module.
    Deserialized(Arc<CompiledModule>),
    /// A fully-verified module instance. Note that it is up to the code cache
    /// to ensure that the module still passes verification in case any configs
    /// are updated, or some feature flags are changed.
    Verified(Arc<Module>),
}

/// An entry for Aptos code cache, capable of resolving different requests including
/// bytes, metadata, etc. Note that it is the responsibility of the code cache to
/// ensure the data is consistent with the latest on-chain configs.
#[derive(Debug)]
pub struct ModuleStorageEntry {
    /// Serialized representation of the module.
    serialized_module: Bytes,
    /// The state value metadata associated with the module, when read from or
    /// written to storage.
    state_value_metadata: StateValueMetadata,
    /// Actual module representation. Can be deserialized, verified, etc.
    representation: Representation,
}

impl ModuleStorageEntry {
    /// Given a transaction write, constructs a new entry that can be used by the
    /// module storage. Returns an error if:
    ///   1. Module is being deleted. This is not allowed at the Move level, but
    ///      transaction write can be a deletion, so returning an error is a good
    ///      precaution.
    ///   2. If module entry cannot be constructed from a state value.
    pub fn from_transaction_write<V: TransactionWrite>(
        runtime_environment: &RuntimeEnvironment,
        write_op: V,
    ) -> Result<ModuleStorageEntry, PanicError> {
        let state_value = write_op.as_state_value().ok_or_else(|| {
            PanicError::CodeInvariantError("Modules cannot be deleted".to_string())
        })?;

        // Creation from the state value deserializes module bytes into compiled module
        // representation. Since we have successfully serialized the module when converting
        // into this transaction write, the deserialization should never fail.
        Self::from_state_value(runtime_environment, state_value).map_err(|e| {
            PanicError::CodeInvariantError(format!(
                "Failed to construct the module from state value: {:?}",
                e
            ))
        })
    }

    /// Given a state value, constructs a new module storage entry. Returns an error if the
    /// module fails to be deserialized.
    pub fn from_state_value(
        runtime_environment: &RuntimeEnvironment,
        state_value: StateValue,
    ) -> VMResult<Self> {
        let (state_value_metadata, serialized_module) = state_value.unpack();

        let deserializer_config = &runtime_environment.vm_config().deserializer_config;
        let compiled_module =
            CompiledModule::deserialize_with_config(&serialized_module, deserializer_config)
                .map_err(|err| {
                    let msg = format!("Deserialization error: {:?}", err);
                    PartialVMError::new(StatusCode::CODE_DESERIALIZATION_ERROR)
                        .with_message(msg)
                        .finish(Location::Undefined)
                })?;

        Ok(Self {
            serialized_module,
            state_value_metadata,
            representation: Representation::Deserialized(Arc::new(compiled_module)),
        })
    }

    /// Creates a new module storage entry which carries all additional metadata, but
    /// uses a verified module representation.
    pub fn make_verified(&self, module: Arc<Module>) -> Self {
        Self {
            serialized_module: self.serialized_module.clone(),
            state_value_metadata: self.state_value_metadata.clone(),
            representation: Representation::Verified(module),
        }
    }

    /// Returns the bytes of the given module.
    pub fn bytes(&self) -> &Bytes {
        &self.serialized_module
    }

    /// Returns the state value metadata of the given module.
    pub fn state_value_metadata(&self) -> &StateValueMetadata {
        &self.state_value_metadata
    }

    /// Returns module's metadata.
    pub fn metadata(&self) -> &[Metadata] {
        use Representation::*;
        match &self.representation {
            Deserialized(m) => &m.metadata,
            Verified(m) => &m.module().metadata,
        }
    }

    /// Returns the deserialized (i.e., [CompiledModule]) representation of the
    /// current storage entry.
    pub fn as_compiled_module(&self) -> Arc<CompiledModule> {
        use Representation::*;
        match &self.representation {
            Deserialized(m) => m.clone(),
            Verified(m) => m.as_compiled_module(),
        }
    }

    /// If the module representation is verified, returns it. Otherwise, returns [None].
    pub fn try_as_verified_module(&self) -> Option<Arc<Module>> {
        use Representation::*;
        match &self.representation {
            Deserialized(_) => None,
            Verified(m) => Some(m.clone()),
        }
    }

    /// Returns true if  the module representation is verified, and false otherwise.
    pub fn is_verified(&self) -> bool {
        use Representation::*;
        match &self.representation {
            Deserialized(_) => false,
            Verified(_) => true,
        }
    }
}
