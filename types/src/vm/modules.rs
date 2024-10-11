// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::PanicError,
    state_store::state_value::{StateValue, StateValueMetadata},
    write_set::TransactionWrite,
};
use bytes::Bytes;
use move_binary_format::{
    access::ModuleAccess,
    errors::{Location, VMResult},
    CompiledModule,
};
#[cfg(any(test, feature = "testing"))]
use move_binary_format::{
    file_format::empty_module_with_dependencies_and_friends, file_format_common::VERSION_MAX,
};
use move_core_types::metadata::Metadata;
use move_vm_runtime::{Module, RuntimeEnvironment};
use move_vm_types::panic_error;
use std::{fmt::Debug, sync::Arc};

/// Different kinds of representation a module can be in. The code cache can implement different
/// policies for promoting the representation from one to the other.
#[derive(Debug, Clone)]
enum Representation {
    /// A simple deserialized representation with a non-verified module.
    Deserialized(Arc<CompiledModule>),
    /// A fully-verified module instance. Note that it is up to the code cache to ensure that the
    /// module still passes verification in case any configs are updated, or some feature flags
    /// are changed.
    Verified(Arc<Module>),
}

/// An entry for Aptos code cache, capable of resolving different requests including bytes, size,
/// metadata, etc. Note that it is the responsibility of the code cache to ensure the data is
/// consistent with the latest on-chain configs.
#[derive(Debug, Clone)]
pub struct ModuleCacheEntry {
    /// Serialized representation of the module.
    serialized_module: Bytes,
    hash: [u8; 32],
    /// The state value metadata associated with the module, when read from or
    /// written to storage.
    state_value_metadata: StateValueMetadata,
    /// Actual module representation. Can be deserialized, verified, etc.
    representation: Representation,
}

impl ModuleCacheEntry {
    /// Given a transaction write, constructs a new entry that can be used by the module storage.
    /// Returns an error if:
    ///   1. Module is being deleted. This is not allowed at the Move level, but transaction write
    ///      can be a deletion, so returning an error is a good precaution.
    ///   2. If module entry cannot be constructed from a state value.
    pub fn from_transaction_write<V: TransactionWrite>(
        runtime_environment: &RuntimeEnvironment,
        write_op: V,
    ) -> Result<ModuleCacheEntry, PanicError> {
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

    /// Creates a deserialized module storage entry from the [StateValue].
    pub fn from_state_value(
        runtime_environment: &RuntimeEnvironment,
        state_value: StateValue,
    ) -> VMResult<Self> {
        let (state_value_metadata, serialized_module) = state_value.unpack();
        let (compiled_module, _, hash) =
            runtime_environment.deserialize_into_compiled_module(&serialized_module)?;

        Ok(Self {
            serialized_module,
            hash,
            state_value_metadata,
            representation: Representation::Deserialized(Arc::new(compiled_module)),
        })
    }

    /// Returns the bytes of the module stored in this entry.
    pub fn bytes(&self) -> &Bytes {
        &self.serialized_module
    }

    /// Returns the size in bytes of the module stored in this entry.
    pub fn size_in_bytes(&self) -> usize {
        self.bytes().len()
    }

    /// Returns the state value metadata for the given entry.
    pub fn state_value_metadata(&self) -> &StateValueMetadata {
        &self.state_value_metadata
    }

    /// Returns the hash of the module stored in this entry.
    pub fn hash(&self) -> &[u8; 32] {
        &self.hash
    }

    /// Returns the module metadata for the given entry.
    pub fn metadata(&self) -> &[Metadata] {
        use Representation::*;
        match &self.representation {
            Deserialized(m) => &m.metadata,
            Verified(m) => &m.module().metadata,
        }
    }

    /// Returns the deserialized (i.e., a [CompiledModule]) representation of the current entry.
    pub fn compiled_module(&self) -> &Arc<CompiledModule> {
        use Representation::*;
        match &self.representation {
            Deserialized(compiled_module) => compiled_module,
            Verified(module) => module.compiled_module(),
        }
    }

    /// Returns the verified (i.e., a [Module]) representation of the current entry. If the entry
    /// is not verified, returns a panic error.
    pub fn verified_module(&self) -> VMResult<&Arc<Module>> {
        use Representation::*;
        match &self.representation {
            Verified(module) => Ok(module),
            Deserialized(compiled_module) => {
                let msg = format!(
                    "Module entry for {}::{} is not verified",
                    compiled_module.address(),
                    compiled_module.name()
                );
                Err(panic_error!(msg).finish(Location::Undefined))
            },
        }
    }

    /// Returns true if the entry stores a verified module.
    pub fn is_verified(&self) -> bool {
        use Representation::*;
        match &self.representation {
            Deserialized(_) => false,
            Verified(_) => true,
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

    /// Creates a new module storage entry which carries all additional metadata, but uses a
    /// verified module representation.
    pub fn make_verified(&self, module: Arc<Module>) -> Self {
        Self {
            serialized_module: self.serialized_module.clone(),
            hash: self.hash,
            state_value_metadata: self.state_value_metadata.clone(),
            representation: Representation::Verified(module),
        }
    }

    /// Creates new deserialized entry based on the provided [RuntimeEnvironment]. Used for tests
    /// only.
    #[cfg(any(test, feature = "testing"))]
    pub fn deserialized_for_test<'a>(
        module_name: &'a str,
        dependencies: impl IntoIterator<Item = &'a str>,
        runtime_environment: &RuntimeEnvironment,
    ) -> Self {
        let module = empty_module_with_dependencies_and_friends(module_name, dependencies, vec![]);

        let mut module_bytes = vec![];
        module
            .serialize_for_version(Some(VERSION_MAX), &mut module_bytes)
            .unwrap();

        ModuleCacheEntry::from_state_value(
            runtime_environment,
            StateValue::new_legacy(module_bytes.into()),
        )
        .unwrap()
    }

    /// Creates new verified entry based on verified dependencies and the [RuntimeEnvironment].
    /// Used for tests only.
    #[cfg(any(test, feature = "testing"))]
    pub fn verified_for_test(
        module_name: &str,
        dependencies: &[Arc<Module>],
        runtime_environment: &RuntimeEnvironment,
    ) -> Self {
        let dep_names = dependencies
            .iter()
            .map(|d| d.compiled_module().self_name().as_str());
        let entry = Self::deserialized_for_test(module_name, dep_names, runtime_environment);
        let locally_verified_module = runtime_environment
            .build_locally_verified_module(
                entry.compiled_module().clone(),
                entry.size_in_bytes(),
                entry.hash(),
            )
            .unwrap();
        let module = runtime_environment
            .build_verified_module(locally_verified_module, dependencies)
            .map(Arc::new)
            .unwrap();
        entry.make_verified(module)
    }
}
