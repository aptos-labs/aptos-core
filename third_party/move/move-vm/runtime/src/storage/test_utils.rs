// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    loader::{Module, Script},
    move_vm::MoveVM,
    script_hash, ModuleStorage, ScriptStorage,
};
use bytes::Bytes;
use move_binary_format::{
    deserializer::DeserializerConfig,
    errors::{PartialVMError, PartialVMResult},
    file_format::CompiledScript,
    CompiledModule,
};
use move_core_types::{
    account_address::AccountAddress,
    identifier::{IdentStr, Identifier},
    metadata::Metadata,
    vm_status::StatusCode,
};
use parking_lot::RwLock;
use std::{
    cell::RefCell,
    collections::{
        btree_map::Entry::{Occupied, Vacant},
        hash_map::Entry,
        BTreeMap, HashMap,
    },
    sync::Arc,
};

enum TestModuleStorageEntry {
    // Serialized module representation.
    Serialized(Bytes),
    // Compiled module representation along with its size.
    Deserialized(Arc<CompiledModule>, usize),
    // Verified module representation.
    Verified(Arc<Module>),
}

/// Module storage implementation which can be used when running unit tests via MoveVM. The
/// implementation is concurrent so that  multiple threads can operate on the same storage
/// (and in case of single-threaded execution, the performance penalty does not matter for
/// tests).
pub struct TestModuleStorage {
    deserializer_config: DeserializerConfig,
    storage: RwLock<BTreeMap<AccountAddress, BTreeMap<Identifier, TestModuleStorageEntry>>>,
}

impl TestModuleStorage {
    /// Returns a new empty module storage which deserializes the code according
    /// to the provided config.
    pub fn empty(deserializer_config: &DeserializerConfig) -> Self {
        Self {
            deserializer_config: deserializer_config.clone(),
            storage: RwLock::new(BTreeMap::new()),
        }
    }

    /// Returns a new empty module storage which deserializes the code according
    /// to the VM deserialization config.
    pub fn empty_for_vm(vm: &MoveVM) -> Self {
        Self::empty(&vm.vm_config().deserializer_config)
    }

    /// Adds serialized module to this module storage. Should not to be used concurrently with
    /// [ModuleStorage] APIs.
    pub fn add_module_bytes(&self, address: &AccountAddress, module_name: &IdentStr, bytes: Bytes) {
        let mut module_storage = self.storage.write();

        let account_module_storage = match module_storage.entry(*address) {
            Occupied(entry) => entry.into_mut(),
            Vacant(entry) => entry.insert(BTreeMap::new()),
        };

        use TestModuleStorageEntry::*;
        account_module_storage.insert(module_name.to_owned(), Serialized(bytes));
    }
}

macro_rules! return_linker_error {
    ($address:ident, $module_name:ident) => {
        let msg = format!("Module {}::{} does not exist", $address, $module_name);
        return Err(PartialVMError::new(StatusCode::LINKER_ERROR).with_message(msg));
    };
}

impl ModuleStorage for TestModuleStorage {
    fn check_module_exists(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<bool> {
        Ok(self
            .storage
            .read()
            .get(address)
            .is_some_and(|account_storage| account_storage.contains_key(module_name)))
    }

    fn fetch_module_size_in_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<usize> {
        use TestModuleStorageEntry::*;

        if !self.check_module_exists(address, module_name)? {
            return_linker_error!(address, module_name);
        }

        let storage = self.storage.read();
        let entry = storage.get(address).unwrap().get(module_name).unwrap();
        Ok(match entry {
            Serialized(bytes) => bytes.len(),
            Deserialized(_, size) => *size,
            Verified(module) => module.size,
        })
    }

    fn fetch_module_metadata(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<Vec<Metadata>> {
        use TestModuleStorageEntry::*;

        if !self.check_module_exists(address, module_name)? {
            return_linker_error!(address, module_name);
        }

        let mut storage = self.storage.write();
        let entry = storage
            .get_mut(address)
            .unwrap()
            .get_mut(module_name)
            .unwrap();

        Ok(match entry {
            Serialized(bytes) => {
                let compiled_module = Arc::new(CompiledModule::deserialize_with_config(
                    bytes,
                    &self.deserializer_config,
                )?);
                *entry = Deserialized(compiled_module.clone(), bytes.len());
                compiled_module.metadata.clone()
            },
            Deserialized(compiled_module, _) => compiled_module.metadata.clone(),
            Verified(module) => module.module().metadata.clone(),
        })
    }

    fn fetch_deserialized_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<Arc<CompiledModule>> {
        use TestModuleStorageEntry::*;

        if !self.check_module_exists(address, module_name)? {
            return_linker_error!(address, module_name);
        }

        let mut storage = self.storage.write();
        let entry = storage
            .get_mut(address)
            .unwrap()
            .get_mut(module_name)
            .unwrap();

        Ok(match entry {
            Serialized(bytes) => {
                let compiled_module = Arc::new(CompiledModule::deserialize_with_config(
                    bytes,
                    &self.deserializer_config,
                )?);
                *entry = Deserialized(compiled_module.clone(), bytes.len());
                compiled_module
            },
            Deserialized(compiled_module, _) => compiled_module.clone(),
            Verified(module) => module.as_compiled_module(),
        })
    }

    fn fetch_or_create_verified_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
        f: &dyn Fn(Arc<CompiledModule>) -> PartialVMResult<Module>,
    ) -> PartialVMResult<Arc<Module>> {
        use TestModuleStorageEntry::*;

        if !self.check_module_exists(address, module_name)? {
            return_linker_error!(address, module_name);
        }

        // Here, the module creation callback may also read/write to this module storage,
        // so we need to make sure we do not end up in a deadlock.
        let storage = self.storage.read();
        let entry = storage.get(address).unwrap().get(module_name).unwrap();
        let compiled_module = match entry {
            Serialized(bytes) => Arc::new(CompiledModule::deserialize_with_config(
                bytes,
                &self.deserializer_config,
            )?),
            Deserialized(compiled_module, _) => compiled_module.clone(),
            Verified(module) => return Ok(module.clone()),
        };

        // Lock is released here.
        drop(storage);

        // Build a module. The callback can potentially acquire locks, so we should not be under
        // the lock ourselves when executing this code.
        let module = Arc::new(f(compiled_module)?);

        // Now re-acquire the lock and set the entry to the right value, if needed.
        let mut storage = self.storage.write();
        let entry = storage
            .get_mut(address)
            .unwrap()
            .get_mut(module_name)
            .unwrap();
        if matches!(entry, Serialized(_) | Deserialized(_, _)) {
            *entry = Verified(module.clone());
        }
        Ok(module)
    }
}

enum TestScriptStorageEntry {
    Deserialized(Arc<CompiledScript>),
    Verified(Arc<Script>),
}

// TODO(loader_v2): Deduplicate this script storage with one defined in Aptos code adapter.
pub struct TestScriptStorage {
    deserializer_config: DeserializerConfig,
    storage: RefCell<HashMap<[u8; 32], TestScriptStorageEntry>>,
}

impl TestScriptStorage {
    pub fn empty(deserializer_config: &DeserializerConfig) -> Self {
        Self {
            deserializer_config: deserializer_config.clone(),
            storage: RefCell::new(HashMap::new()),
        }
    }

    fn deserialize_script(&self, serialized_script: &[u8]) -> PartialVMResult<Arc<CompiledScript>> {
        let compiled_script =
            CompiledScript::deserialize_with_config(serialized_script, &self.deserializer_config)
                .map_err(|err| {
                // Ensure we remap the error to be consistent with loader V1 implementation.
                let msg = format!("[VM] deserializer for script returned error: {:?}", err);
                PartialVMError::new(StatusCode::CODE_DESERIALIZATION_ERROR).with_message(msg)
            })?;
        Ok(Arc::new(compiled_script))
    }
}

impl ScriptStorage for TestScriptStorage {
    fn fetch_deserialized_script(
        &self,
        serialized_script: &[u8],
    ) -> PartialVMResult<Arc<CompiledScript>> {
        use TestScriptStorageEntry::*;

        let hash = script_hash(serialized_script);
        let mut storage = self.storage.borrow_mut();

        Ok(match storage.entry(hash) {
            Entry::Occupied(e) => match e.get() {
                Deserialized(compiled_script) => compiled_script.clone(),
                Verified(script) => script.script.clone(),
            },
            Entry::Vacant(e) => {
                let compiled_script = self.deserialize_script(serialized_script)?;
                e.insert(Deserialized(compiled_script.clone()));
                compiled_script
            },
        })
    }

    fn fetch_or_create_verified_script(
        &self,
        serialized_script: &[u8],
        f: &dyn Fn(Arc<CompiledScript>) -> PartialVMResult<Script>,
    ) -> PartialVMResult<Arc<Script>> {
        use TestScriptStorageEntry::*;

        let hash = script_hash(serialized_script);
        let mut storage = self.storage.borrow_mut();

        Ok(match storage.entry(hash) {
            Entry::Occupied(mut e) => match e.get() {
                Deserialized(compiled_script) => {
                    let script = Arc::new(f(compiled_script.clone())?);
                    e.insert(Verified(script.clone()));
                    script
                },
                Verified(script) => script.clone(),
            },
            Entry::Vacant(e) => {
                let compiled_script = self.deserialize_script(serialized_script)?;
                let script = Arc::new(f(compiled_script)?);
                e.insert(Verified(script.clone()));
                script
            },
        })
    }
}

#[cfg(test)]
mod test {
    // TODO(loader_v2): Implement tests for these module storage implementations. Make sure locking
    //                  is correct in multi-threaded environment. Move tests from Aptos code script
    //                  storage adapter here.
}
