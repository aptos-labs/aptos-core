// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    IntoUnsyncModuleStorage, Module, ModuleBytesStorage, ModuleStorage, RuntimeEnvironment,
    UnsyncModuleStorage,
};
use bytes::Bytes;
use move_binary_format::{
    access::ModuleAccess,
    compatibility::Compatibility,
    errors::{verification_error, PartialVMError, PartialVMResult},
    normalized, CompiledModule, IndexKind,
};
use move_core_types::{
    account_address::AccountAddress,
    identifier::{IdentStr, Identifier},
    metadata::Metadata,
    vm_status::StatusCode,
};
use std::{
    collections::{btree_map, BTreeMap},
    sync::Arc,
};

/// An implementation of [ModuleBytesStorage] that stores temporary changes. If used by
/// [ModuleStorage], the most recent version of a module will be fetched.
struct TemporaryModuleBytesStorage<'m, M> {
    // Modules to be published, staged temporarily.
    temporary_storage: BTreeMap<AccountAddress, BTreeMap<Identifier, (Bytes, CompiledModule)>>,
    // Underlying ground-truth module storage.
    module_storage: &'m M,
}

impl<'m, M: ModuleStorage> TemporaryModuleBytesStorage<'m, M> {
    /// Returns a new storage instance, performing compatibility checks during staging
    /// of published modules.
    fn new(
        sender: &AccountAddress,
        env: &RuntimeEnvironment,
        compatibility: Compatibility,
        module_storage: &'m M,
        module_bundle: Vec<Bytes>,
    ) -> PartialVMResult<Self> {
        use btree_map::Entry::*;

        let mut temporary_storage = BTreeMap::new();
        for module_bytes in module_bundle {
            let deserializer_config = &env.vm_config().deserializer_config;
            let compiled_module =
                CompiledModule::deserialize_with_config(&module_bytes, deserializer_config)?;
            let addr = compiled_module.self_addr();
            let name = compiled_module.self_name();

            // Make sure all modules' addresses match the sender. The self address is
            // where the module will actually be published. If we did not check this,
            // the sender could publish a module under anyone's account.
            if addr != sender {
                return Err(verification_error(
                    StatusCode::MODULE_ADDRESS_DOES_NOT_MATCH_SENDER,
                    IndexKind::AddressIdentifier,
                    compiled_module.self_handle_idx().0,
                ));
            }

            // All modules can be republished, as long as the new module is compatible
            // with the old module.
            let module_exists = module_storage.check_module_exists(addr, name)?;
            if module_exists && compatibility.need_check_compat() {
                let old_module_ref = module_storage.fetch_verified_module(addr, name)?;
                let old_module = old_module_ref.module();
                if env.vm_config().use_compatibility_checker_v2 {
                    compatibility.check(old_module, &compiled_module)?
                } else {
                    #[allow(deprecated)]
                    let old_m = normalized::Module::new(old_module)?;
                    #[allow(deprecated)]
                    let new_m = normalized::Module::new(&compiled_module)?;
                    compatibility.legacy_check(&old_m, &new_m)?
                }
            }

            let account_module_storage = match temporary_storage.entry(*compiled_module.self_addr())
            {
                Occupied(entry) => entry.into_mut(),
                Vacant(entry) => entry.insert(BTreeMap::new()),
            };

            // Publishing the same module in the same bundle is not allowed.
            account_module_storage
                .insert(
                    compiled_module.self_name().to_owned(),
                    (module_bytes, compiled_module),
                )
                .ok_or_else(|| PartialVMError::new(StatusCode::DUPLICATE_MODULE_NAME))?;
        }

        Ok(Self {
            temporary_storage,
            module_storage,
        })
    }

    /// Returns addresses and names of all modules that were temporarily staged.
    fn staged_modules_iter(&self) -> impl Iterator<Item = (&AccountAddress, &IdentStr)> {
        self.temporary_storage
            .iter()
            .flat_map(|(addr, account_storage)| {
                account_storage
                    .iter()
                    .map(move |(name, _)| (addr, name.as_ident_str()))
            })
    }
}

impl<'m, M: ModuleStorage> ModuleBytesStorage for TemporaryModuleBytesStorage<'m, M> {
    fn fetch_module_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<Option<Bytes>> {
        if let Some(account_storage) = self.temporary_storage.get(address) {
            if let Some((bytes, _)) = account_storage.get(module_name) {
                return Ok(Some(bytes.clone()));
            }
        }
        self.module_storage.fetch_module_bytes(address, module_name)
    }
}

/// A [ModuleStorage] implementation which can stage published modules temporarily, without
/// leaking them into the underlying module storage. When modules are staged, multiple
/// checks are performed to ensure that:
///   1) Published modules are published to correct address of the sender.
///   2) Published modules satisfy compatibility constraints.
///   3) Published modules are verifiable and can link to existing modules without breaking
///      invariants such as cyclic dependencies.
pub struct TemporaryModuleStorage<'a, M> {
    storage: UnsyncModuleStorage<'a, TemporaryModuleBytesStorage<'a, M>>,
}

impl<'a, M: ModuleStorage> TemporaryModuleStorage<'a, M> {
    /// Returns new temporary module storage running full compatability checks.
    pub fn new(
        sender: &AccountAddress,
        env: &'a RuntimeEnvironment,
        existing_module_storage: &'a M,
        module_bundle: Vec<Bytes>,
    ) -> PartialVMResult<Self> {
        Self::new_with_compat_config(
            sender,
            env,
            Compatibility::full_check(),
            existing_module_storage,
            module_bundle,
        )
    }

    /// Returns new temporary module storage.
    pub fn new_with_compat_config(
        sender: &AccountAddress,
        env: &'a RuntimeEnvironment,
        compatibility: Compatibility,
        existing_module_storage: &'a M,
        module_bundle: Vec<Bytes>,
    ) -> PartialVMResult<Self> {
        // Verify compatibility here.
        let temporary_module_bytes_storage = TemporaryModuleBytesStorage::new(
            sender,
            env,
            compatibility,
            existing_module_storage,
            module_bundle,
        )?;
        let temporary_module_storage =
            temporary_module_bytes_storage.into_unsync_module_storage(env);

        // Verify the bundle, performing linking checks (e.g., no cyclic dependencies).
        for (addr, name) in temporary_module_storage
            .byte_storage()
            .staged_modules_iter()
        {
            temporary_module_storage.fetch_verified_module(addr, name)?;
        }

        Ok(Self {
            storage: temporary_module_storage,
        })
    }

    pub fn release_verified_module_bundle(self) -> impl Iterator<Item = (Bytes, CompiledModule)> {
        self.storage
            .release_byte_storage()
            .temporary_storage
            .into_iter()
            .flat_map(|(_, account_storage)| account_storage.into_values())
    }
}

impl<'a, M: ModuleStorage> ModuleStorage for TemporaryModuleStorage<'a, M> {
    fn check_module_exists(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<bool> {
        self.storage.check_module_exists(address, module_name)
    }

    fn fetch_module_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<Option<Bytes>> {
        self.storage.fetch_module_bytes(address, module_name)
    }

    fn fetch_module_size_in_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<usize> {
        self.storage
            .fetch_module_size_in_bytes(address, module_name)
    }

    fn fetch_module_metadata(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<Vec<Metadata>> {
        self.storage.fetch_module_metadata(address, module_name)
    }

    fn fetch_deserialized_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<Arc<CompiledModule>> {
        self.storage.fetch_deserialized_module(address, module_name)
    }

    fn fetch_verified_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<Arc<Module>> {
        self.storage.fetch_verified_module(address, module_name)
    }
}
