// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    AsUnsyncModuleStorage, Module, ModuleStorage, RuntimeEnvironment, UnsyncModuleStorage,
    WithRuntimeEnvironment,
};
use bytes::Bytes;
use move_binary_format::{
    access::ModuleAccess,
    compatibility::Compatibility,
    errors::{verification_error, Location, PartialVMError, VMResult},
    normalized, CompiledModule, IndexKind,
};
use move_core_types::{
    account_address::AccountAddress,
    identifier::{IdentStr, Identifier},
    language_storage::ModuleId,
    metadata::Metadata,
    vm_status::StatusCode,
};
use move_vm_types::{code_storage::ModuleBytesStorage, module_linker_error};
use std::{
    collections::{btree_map, BTreeMap},
    sync::Arc,
};

/// Represents a verified module pundle that can be extracted from [StagingModuleStorage].
pub struct VerifiedModuleBundle<K: Ord, V: Clone> {
    bundle: BTreeMap<K, V>,
}

impl<K: Ord, V: Clone> IntoIterator for VerifiedModuleBundle<K, V> {
    type IntoIter = btree_map::IntoIter<K, V>;
    type Item = (K, V);

    fn into_iter(self) -> Self::IntoIter {
        self.bundle.into_iter()
    }
}

/// An implementation of [ModuleBytesStorage] that stores some additional staged changes. If used
/// by [ModuleStorage], the most recent version of a module will be fetched.
struct StagingModuleBytesStorage<'a, M> {
    // Modules to be published, staged temporarily.
    staged_module_bytes: BTreeMap<AccountAddress, BTreeMap<Identifier, Bytes>>,
    // Underlying ground-truth module storage, used as a raw byte storage.
    module_storage: &'a M,
}

impl<'a, M: ModuleStorage> ModuleBytesStorage for StagingModuleBytesStorage<'a, M> {
    fn fetch_module_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Bytes>> {
        if let Some(account_storage) = self.staged_module_bytes.get(address) {
            if let Some(bytes) = account_storage.get(module_name) {
                return Ok(Some(bytes.clone()));
            }
        }
        self.module_storage.fetch_module_bytes(address, module_name)
    }
}

/// A [ModuleStorage] implementation which can stage published modules temporarily, without
/// leaking them into the underlying module storage. When modules are staged, multiple checks are
/// performed to ensure that:
///   1) Published modules are published to correct address of the sender.
///   2) Published modules satisfy compatibility constraints.
///   3) Published modules are verifiable and can link to existing modules without breaking
///      invariants such as cyclic dependencies.
#[ouroboros::self_referencing]
pub struct StagingModuleStorage<'a, M: 'a> {
    // TODO(loader_v2):
    //   Avoid clone and instead stage runtime environment so that higher order indices are
    //   resolved through some temporary data structure.
    runtime_environment: RuntimeEnvironment,
    #[borrows(runtime_environment)]
    #[covariant]
    storage: UnsyncModuleStorage<'this, StagingModuleBytesStorage<'a, M>>,
}

impl<'a, M: ModuleStorage> StagingModuleStorage<'a, M> {
    /// Returns new module storage with staged modules, running full compatability checks for them.
    pub fn create(
        sender: &AccountAddress,
        existing_module_storage: &'a M,
        module_bundle: Vec<Bytes>,
    ) -> VMResult<Self> {
        Self::create_with_compat_config(
            sender,
            Compatibility::full_check(),
            existing_module_storage,
            module_bundle,
        )
    }

    /// Returns new module storage with staged modules, checking compatibility based on the
    /// provided config.
    pub fn create_with_compat_config(
        sender: &AccountAddress,
        compatibility: Compatibility,
        existing_module_storage: &'a M,
        module_bundle: Vec<Bytes>,
    ) -> VMResult<Self> {
        // Create a new runtime environment, so that it is not shared with the existing one. This
        // is extremely important for correctness of module publishing: we need to make sure that
        // no speculative information is cached! By cloning the environment, we ensure that when
        // using this new module storage with changes, global caches are not accessed. Only when
        // the published module is committed, and its structs are accessed, their information will
        // be cached in the global runtime environment.
        let runtime_environment = existing_module_storage.runtime_environment().clone();
        let deserializer_config = &runtime_environment.vm_config().deserializer_config;

        // For every module in bundle, run compatibility checks and construct a new bytes storage
        // view such that added modules shadow any existing ones.
        let mut staged_module_bytes = BTreeMap::new();
        for module_bytes in module_bundle {
            let compiled_module =
                CompiledModule::deserialize_with_config(&module_bytes, deserializer_config)
                    .map_err(|err| {
                        err.append_message_with_separator(
                            '\n',
                            "[VM] module deserialization failed".to_string(),
                        )
                        .finish(Location::Undefined)
                    })?;
            let addr = compiled_module.self_addr();
            let name = compiled_module.self_name();

            // Make sure all modules' addresses match the sender. The self address is
            // where the module will actually be published. If we did not check this,
            // the sender could publish a module under anyone's account.
            if addr != sender {
                let msg = format!(
                    "Compiled modules address {} does not match the sender {}",
                    addr, sender
                );
                return Err(verification_error(
                    StatusCode::MODULE_ADDRESS_DOES_NOT_MATCH_SENDER,
                    IndexKind::AddressIdentifier,
                    compiled_module.self_handle_idx().0,
                )
                .with_message(msg)
                .finish(Location::Undefined));
            }

            // All modules can be republished, as long as the new module is compatible
            // with the old module.
            if compatibility.need_check_compat() {
                if let Some(old_module_ref) =
                    existing_module_storage.fetch_deserialized_module(addr, name)?
                {
                    let old_module = old_module_ref.as_ref();
                    if runtime_environment.vm_config().use_compatibility_checker_v2 {
                        compatibility
                            .check(old_module, &compiled_module)
                            .map_err(|e| e.finish(Location::Undefined))?;
                    } else {
                        #[allow(deprecated)]
                        let old_m = normalized::Module::new(old_module)
                            .map_err(|e| e.finish(Location::Undefined))?;
                        #[allow(deprecated)]
                        let new_m = normalized::Module::new(&compiled_module)
                            .map_err(|e| e.finish(Location::Undefined))?;
                        compatibility
                            .legacy_check(&old_m, &new_m)
                            .map_err(|e| e.finish(Location::Undefined))?;
                    }
                }
            }

            // Modules that pass compatibility checks are added to the staged storage.
            use btree_map::Entry::*;
            let account_module_storage =
                match staged_module_bytes.entry(*compiled_module.self_addr()) {
                    Occupied(entry) => entry.into_mut(),
                    Vacant(entry) => entry.insert(BTreeMap::new()),
                };
            let prev =
                account_module_storage.insert(compiled_module.self_name().to_owned(), module_bytes);

            // Publishing the same module in the same bundle is not allowed.
            if prev.is_some() {
                let msg = format!(
                    "Module {}::{} occurs more than once in published bundle",
                    compiled_module.self_addr(),
                    compiled_module.self_name()
                );
                return Err(PartialVMError::new(StatusCode::DUPLICATE_MODULE_NAME)
                    .with_message(msg)
                    .finish(Location::Undefined));
            }
        }

        // At this point, we have successfully created a new module storage that also contains the
        // newly published bundle.
        let staged_module_storage = StagingModuleStorageBuilder {
            runtime_environment,
            storage_builder: |runtime_environment| {
                let staged_module_bytes_storage = StagingModuleBytesStorage {
                    staged_module_bytes,
                    module_storage: existing_module_storage,
                };
                // Create module storage by "owning" the underlying bytes.
                staged_module_bytes_storage.into_unsync_module_storage(runtime_environment)
            },
        }
        .build();

        // Finally, verify the bundle, performing linking checks for all staged modules.
        for (addr, name) in staged_module_storage
            .borrow_storage()
            .byte_storage()
            .staged_module_bytes
            .iter()
            .flat_map(|(addr, account_storage)| {
                account_storage
                    .iter()
                    .map(move |(name, _)| (addr, name.as_ident_str()))
            })
        {
            // Verify the module and its dependencies, and that they do not form a cycle.
            let module = staged_module_storage
                .fetch_verified_module(addr, name)?
                .ok_or_else(|| {
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message(format!(
                            "Staged module {}::{} must always exist",
                            addr, name
                        ))
                        .finish(Location::Undefined)
                })?;

            // Also verify that all friends exist.
            for (friend_addr, friend_name) in module.module().immediate_friends_iter() {
                if !staged_module_storage.check_module_exists(friend_addr, friend_name)? {
                    return Err(module_linker_error!(friend_addr, friend_name));
                }
            }
        }

        // All checks passed! Now this storage can be used to run Move functions.
        Ok(staged_module_storage)
    }

    pub fn release_verified_module_bundle(self) -> VerifiedModuleBundle<ModuleId, Bytes> {
        let staged_module_bytes = &self.borrow_storage().byte_storage().staged_module_bytes;

        let mut bundle = BTreeMap::new();
        for (addr, account_storage) in staged_module_bytes {
            for (name, bytes) in account_storage {
                bundle.insert(ModuleId::new(*addr, name.clone()), bytes.clone());
            }
        }

        VerifiedModuleBundle { bundle }
    }
}

/// Note: [ambassador::Delegate] cannot be used for [StagingModuleStorage] because it is a self-
/// referencing struct.
impl<'a, M: ModuleStorage> WithRuntimeEnvironment for StagingModuleStorage<'a, M> {
    fn runtime_environment(&self) -> &RuntimeEnvironment {
        self.borrow_runtime_environment()
    }
}

/// Note: [ambassador::Delegate] cannot be used for [StagingModuleStorage] because it is a self-
/// referencing struct.
impl<'a, M: ModuleStorage> ModuleStorage for StagingModuleStorage<'a, M> {
    fn check_module_exists(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<bool> {
        self.borrow_storage()
            .check_module_exists(address, module_name)
    }

    fn fetch_module_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Bytes>> {
        self.borrow_storage()
            .fetch_module_bytes(address, module_name)
    }

    fn fetch_module_size_in_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<usize>> {
        self.borrow_storage()
            .fetch_module_size_in_bytes(address, module_name)
    }

    fn fetch_module_metadata(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Vec<Metadata>>> {
        self.borrow_storage()
            .fetch_module_metadata(address, module_name)
    }

    fn fetch_deserialized_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Arc<CompiledModule>>> {
        self.borrow_storage()
            .fetch_deserialized_module(address, module_name)
    }

    fn fetch_verified_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Arc<Module>>> {
        self.borrow_storage()
            .fetch_verified_module(address, module_name)
    }
}
