// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::module_and_script_storage::{
    module_storage::AptosModuleStorage, script_storage::AptosScriptStorage,
    state_view_adapter::ModuleStorageEntry::Deserialized,
};
use aptos_types::{
    on_chain_config::{Features, OnChainConfig},
    state_store::{state_key::StateKey, StateView},
    vm::configs::aptos_prod_deserializer_config,
};
use bytes::Bytes;
#[cfg(test)]
use claims::{assert_matches, assert_ok};
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
use move_vm_runtime::{script_hash, Module, ModuleStorage, Script, ScriptStorage};
use std::{
    cell::RefCell,
    collections::{btree_map, hash_map, BTreeMap, HashMap},
    sync::Arc,
};

macro_rules! module_storage_error {
    ($addr:ident, $name:ident, $err:ident) => {
        PartialVMError::new(StatusCode::STORAGE_ERROR).with_message(format!(
            "Unexpected storage error for module {}::{}: {:?}",
            $addr, $name, $err
        ))
    };
}

macro_rules! module_linker_error {
    ($addr:ident, $name:ident) => {
        PartialVMError::new(StatusCode::LINKER_ERROR)
            .with_message(format!("Module {}::{} does not exist", $addr, $name))
    };
}

#[derive(Debug)]
enum ModuleStorageEntry {
    Deserialized(Arc<CompiledModule>),
    Verified(Arc<Module>),
}

#[derive(Debug)]
enum ScriptStorageEntry {
    Deserialized(Arc<CompiledScript>),
    Verified(Arc<Script>),
}

/// A simple not thread-safe implementation of code storage on top of a state view.
/// It is never built directly by clients - only via [AsAptosCodeStorage] trait.
pub struct AptosCodeStorageAdapter<'s, S> {
    // Config used to build compiled modules and scripts.
    deserializer_config: DeserializerConfig,

    // Module and script code storages.
    module_storage: RefCell<BTreeMap<AccountAddress, BTreeMap<Identifier, ModuleStorageEntry>>>,
    script_storage: RefCell<HashMap<[u8; 32], ScriptStorageEntry>>,

    // Baseline state view reference from which we can fetch raw module bytes
    // and the associated metadata.
    state_view: &'s S,
}

impl<'s, S: StateView> AptosCodeStorageAdapter<'s, S> {
    fn new(state_view: &'s S) -> Self {
        let features = Features::fetch_config(state_view).unwrap_or_default();
        let deserializer_config = aptos_prod_deserializer_config(&features);
        Self {
            deserializer_config,
            module_storage: RefCell::new(BTreeMap::new()),
            script_storage: RefCell::new(HashMap::new()),
            state_view,
        }
    }

    /// Returns module bytes, and an error if it does not exist.
    fn get_existing_module_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<Bytes> {
        let state_key = StateKey::module(address, module_name);
        self.state_view
            .get_state_value_bytes(&state_key)
            .map_err(|e| module_storage_error!(address, module_name, e))?
            .ok_or_else(|| module_linker_error!(address, module_name))
    }

    /// Returns rue if the module is cached in module storage.
    fn check_module_exists_in_module_storage(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> bool {
        let module_storage = self.module_storage.borrow();
        module_storage
            .get(address)
            .is_some_and(|account_module_storage| account_module_storage.contains_key(module_name))
    }

    /// If module is not yet cached in module storage, fetches it from the state view
    /// and caches as deserialized entry.
    fn initialize_module_storage_entry(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<()> {
        use btree_map::Entry::*;

        if !self.check_module_exists_in_module_storage(address, module_name) {
            let bytes = self.get_existing_module_bytes(address, module_name)?;
            let new_entry = Deserialized(Arc::new(CompiledModule::deserialize_with_config(
                &bytes,
                &self.deserializer_config,
            )?));

            let mut module_storage = self.module_storage.borrow_mut();
            let account_module_storage = match module_storage.entry(*address) {
                Occupied(entry) => entry.into_mut(),
                Vacant(entry) => entry.insert(BTreeMap::new()),
            };
            account_module_storage.insert(module_name.to_owned(), new_entry);
        }
        Ok(())
    }
}

impl<'s, S: StateView> ModuleStorage for AptosCodeStorageAdapter<'s, S> {
    fn check_module_exists(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<bool> {
        // Check module existence via state view.
        let state_key = StateKey::module(address, module_name);
        Ok(self
            .state_view
            .get_state_value(&state_key)
            .map_err(|e| module_storage_error!(address, module_name, e))?
            .is_some())
    }

    fn fetch_module_size_in_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<usize> {
        Ok(self.get_existing_module_bytes(address, module_name)?.len())
    }

    fn fetch_module_metadata(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<Vec<Metadata>> {
        Ok(self
            .fetch_deserialized_module(address, module_name)?
            .metadata
            .clone())
    }

    fn fetch_deserialized_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<Arc<CompiledModule>> {
        use ModuleStorageEntry::*;

        self.initialize_module_storage_entry(address, module_name)?;

        // At this point module storage contains the entry.
        let module_storage = self.module_storage.borrow();
        let entry = module_storage
            .get(address)
            .unwrap()
            .get(module_name)
            .unwrap();
        Ok(match entry {
            Deserialized(compiled_module) => compiled_module.clone(),
            Verified(module) => module.as_compiled_module(),
        })
    }

    fn fetch_or_create_verified_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
        f: &dyn Fn(Arc<CompiledModule>) -> PartialVMResult<Module>,
    ) -> PartialVMResult<Arc<Module>> {
        use ModuleStorageEntry::*;

        self.initialize_module_storage_entry(address, module_name)?;

        // At this point module storage contains the entry, but it still can be deserialized,
        // so we need to make sure to promote it to the verified.
        let mut module_storage = self.module_storage.borrow_mut();
        let entry = module_storage
            .get_mut(address)
            .unwrap()
            .get_mut(module_name)
            .unwrap();
        Ok(match entry {
            Deserialized(compiled_module) => {
                let module = Arc::new(f(compiled_module.clone())?);
                *entry = Verified(module.clone());
                module
            },
            Verified(module) => module.clone(),
        })
    }
}

impl<'s, S: StateView> AptosModuleStorage for AptosCodeStorageAdapter<'s, S> {}

impl<'s, S: StateView> ScriptStorage for AptosCodeStorageAdapter<'s, S> {
    fn fetch_deserialized_script(
        &self,
        serialized_script: &[u8],
    ) -> PartialVMResult<Arc<CompiledScript>> {
        use hash_map::Entry::*;
        use ScriptStorageEntry::*;

        let mut script_storage = self.script_storage.borrow_mut();
        let script_hash = script_hash(serialized_script);

        Ok(match script_storage.entry(script_hash) {
            Occupied(e) => match e.get() {
                Deserialized(compiled_script) => compiled_script.clone(),
                Verified(script) => script.as_compiled_script(),
            },
            Vacant(e) => {
                let compiled_script = Arc::new(CompiledScript::deserialize_with_config(
                    serialized_script,
                    &self.deserializer_config,
                )?);
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
        use hash_map::Entry::*;
        use ScriptStorageEntry::*;

        let mut script_storage = self.script_storage.borrow_mut();
        let script_hash = script_hash(serialized_script);

        Ok(match script_storage.entry(script_hash) {
            Occupied(mut e) => match e.get() {
                Deserialized(compiled_script) => {
                    let script = Arc::new(f(compiled_script.clone())?);
                    e.insert(Verified(script.clone()));
                    script
                },
                Verified(script) => script.clone(),
            },
            Vacant(e) => {
                let compiled_script = Arc::new(CompiledScript::deserialize_with_config(
                    serialized_script,
                    &self.deserializer_config,
                )?);
                let script = Arc::new(f(compiled_script)?);
                e.insert(Verified(script.clone()));
                script
            },
        })
    }
}

impl<'s, S: StateView> AptosScriptStorage for AptosCodeStorageAdapter<'s, S> {}

/// Allows to treat a state view as a code storage with scripts and modules. The
/// main use case is when transaction or a Move function has to be executed outside
/// the long-living environment or block executor, e.g., for single transaction
/// simulation, Aptos debugger, etc.
pub trait AsAptosCodeStorage<S> {
    fn as_aptos_code_storage(&self) -> AptosCodeStorageAdapter<S>;
}

impl<S: StateView> AsAptosCodeStorage<S> for S {
    fn as_aptos_code_storage(&self) -> AptosCodeStorageAdapter<S> {
        AptosCodeStorageAdapter::new(self)
    }
}

#[cfg(test)]
impl<'s, S: StateView> AptosCodeStorageAdapter<'s, S> {
    fn assert_deserialized_module_entry_at(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) {
        let module_storage = self.module_storage.borrow();
        let entry = module_storage
            .get(address)
            .unwrap_or_else(|| panic!("No module at address {}", address))
            .get(module_name)
            .unwrap_or_else(|| panic!("No module {}::{} in module storage", address, module_name));

        use ModuleStorageEntry::*;
        assert_matches!(entry, Deserialized(_));
    }

    fn assert_verified_module_entry_at(&self, address: &AccountAddress, module_name: &IdentStr) {
        let module_storage = self.module_storage.borrow();
        let entry = module_storage
            .get(address)
            .unwrap_or_else(|| panic!("No module at address {}", address))
            .get(module_name)
            .unwrap_or_else(|| panic!("No module {}::{} in module storage", address, module_name));

        use ModuleStorageEntry::*;
        assert_matches!(entry, Verified(_));
    }

    fn assert_deserialized_script_entry_exists_for(&self, serialized_script: &[u8]) {
        let expected_compiled_script = assert_ok!(CompiledScript::deserialize_with_config(
            serialized_script,
            &self.deserializer_config
        ));

        let script_storage = self.script_storage.borrow();
        let entry = script_storage
            .get(&script_hash(serialized_script))
            .expect("Script entry must exist");

        use ScriptStorageEntry::*;
        if let Deserialized(compiled_script) = entry {
            assert_eq!(&expected_compiled_script, compiled_script.as_ref());
        } else {
            panic!("Expected a deserialized script");
        }
    }

    fn assert_verified_script_entry_exists_for(&self, serialized_script: &[u8]) {
        let expected_compiled_script = assert_ok!(CompiledScript::deserialize_with_config(
            serialized_script,
            &self.deserializer_config
        ));

        let script_storage = self.script_storage.borrow();
        let entry = script_storage
            .get(&script_hash(serialized_script))
            .expect("Script entry must exist");

        use ScriptStorageEntry::*;
        if let Verified(script) = entry {
            assert_eq!(
                &expected_compiled_script,
                script.as_compiled_script().as_ref()
            );
        } else {
            panic!("Expected a verified script");
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use aptos_language_e2e_tests::data_store::FakeDataStore;
    use aptos_vm::data_cache::AsMoveResolver;
    use claims::{assert_err, assert_ok};
    use move_binary_format::file_format::{basic_test_module, basic_test_script};
    use move_core_types::{ident_str, language_storage::ModuleId};
    use move_vm_runtime::{
        config::VMConfig,
        module_traversal::{TraversalContext, TraversalStorage},
        move_vm::MoveVM,
        should_use_loader_v2,
    };
    use move_vm_types::gas::UnmeteredGasMeter;

    #[test]
    fn test_module_existence_in_adapter() {
        let state_view = FakeDataStore::default();
        let module_storage = state_view.as_aptos_code_storage();

        // Module does not exist.
        let result = module_storage.check_module_exists(&AccountAddress::ONE, ident_str!("foo"));
        assert!(!assert_ok!(result));

        // Any other access to non-existing module results in linker error.
        let err = assert_err!(
            module_storage.fetch_module_size_in_bytes(&AccountAddress::ONE, ident_str!("foo"))
        );
        assert_eq!(err.major_status(), StatusCode::LINKER_ERROR);

        let err = assert_err!(
            module_storage.fetch_module_metadata(&AccountAddress::ONE, ident_str!("foo"))
        );
        assert_eq!(err.major_status(), StatusCode::LINKER_ERROR);

        let err = assert_err!(
            module_storage.fetch_deserialized_module(&AccountAddress::ONE, ident_str!("foo"))
        );
        assert_eq!(err.major_status(), StatusCode::LINKER_ERROR);

        let err = assert_err!(module_storage.fetch_or_create_verified_module(
            &AccountAddress::ONE,
            ident_str!("foo"),
            &|_| unreachable!()
        ));
        assert_eq!(err.major_status(), StatusCode::LINKER_ERROR);
    }

    #[test]
    fn test_module_size_in_adapter() {
        let mut state_view = FakeDataStore::default();

        // Two dummy modules with different sizes.
        state_view.set_legacy(
            StateKey::module(&AccountAddress::ONE, ident_str!("foo")),
            vec![],
        );
        state_view.set_legacy(
            StateKey::module(&AccountAddress::TWO, ident_str!("bar")),
            vec![0, 1, 2],
        );
        let module_storage = state_view.as_aptos_code_storage();

        let foo_size = assert_ok!(
            module_storage.fetch_module_size_in_bytes(&AccountAddress::ONE, ident_str!("foo"))
        );
        assert_eq!(foo_size, 0);

        let bar_size = assert_ok!(
            module_storage.fetch_module_size_in_bytes(&AccountAddress::TWO, ident_str!("bar"))
        );
        assert_eq!(bar_size, 3);
    }

    #[test]
    fn test_deserialized_and_verified_module_in_adapter() {
        let test_module = basic_test_module();
        let addr = test_module.self_addr();
        let name = test_module.self_name();

        let mut test_module_bytes = vec![];
        let bytecode_version = Some(Features::default().get_max_binary_format_version());
        assert_ok!(test_module.serialize_for_version(bytecode_version, &mut test_module_bytes));

        let mut state_view = FakeDataStore::default();
        state_view.set_legacy(StateKey::module(addr, name), test_module_bytes);
        let module_storage = state_view.as_aptos_code_storage();

        // Module is not cached yet.
        assert!(!module_storage.check_module_exists_in_module_storage(addr, name));

        // Module is still not cached if we accessed its size or checked its existence.
        assert_ok!(module_storage.check_module_exists(addr, name));
        assert_ok!(module_storage.fetch_module_size_in_bytes(addr, name));
        assert!(!module_storage.check_module_exists_in_module_storage(addr, name));

        // After first access it is promoted to cache.
        let returned_module = assert_ok!(module_storage.fetch_deserialized_module(addr, name));
        assert_eq!(returned_module.as_ref(), &test_module);
        module_storage.assert_deserialized_module_entry_at(addr, name);

        // Check it is still the case on repeated access, e.g. to metadata.
        assert_ok!(module_storage.fetch_module_metadata(addr, name));
        module_storage.assert_deserialized_module_entry_at(addr, name);

        if should_use_loader_v2() {
            // Next, check we can promote entries into verified ones.
            let vm_config = VMConfig {
                deserializer_config: module_storage.deserializer_config.clone(),
                ..VMConfig::default()
            };
            let vm = MoveVM::new_with_config(vec![], vm_config);
            let resolver = state_view.as_move_resolver();

            let traversal_storage = TraversalStorage::new();
            let mut session = vm.new_session(&resolver);
            assert_ok!(session.execute_function_bypass_visibility(
                &ModuleId::new(*addr, name.to_owned()),
                ident_str!("foo"),
                vec![],
                Vec::<Vec<u8>>::new(),
                &mut UnmeteredGasMeter,
                &mut TraversalContext::new(&traversal_storage),
                &module_storage,
            ));
            module_storage.assert_verified_module_entry_at(addr, name);
        }
    }

    #[test]
    fn test_deserialized_and_verified_script_in_adapter() {
        let state_view = FakeDataStore::default();
        let module_and_script_storage = state_view.as_aptos_code_storage();

        let test_script = basic_test_script();
        let mut test_script_bytes = vec![];
        let bytecode_version = Some(Features::default().get_max_binary_format_version());
        assert_ok!(test_script.serialize_for_version(bytecode_version, &mut test_script_bytes));

        // Check that the script is correctly returned and is also cached.
        let returned_script =
            assert_ok!(module_and_script_storage.fetch_deserialized_script(&test_script_bytes));
        assert_eq!(returned_script.as_ref(), &test_script);
        module_and_script_storage.assert_deserialized_script_entry_exists_for(&test_script_bytes);

        if should_use_loader_v2() {
            // Next, check we can promote entries into verified ones.
            let vm_config = VMConfig {
                deserializer_config: module_and_script_storage.deserializer_config.clone(),
                ..VMConfig::default()
            };
            let vm = MoveVM::new_with_config(vec![], vm_config);
            let resolver = state_view.as_move_resolver();

            let traversal_storage = TraversalStorage::new();
            let mut session = vm.new_session(&resolver);
            assert_ok!(session.execute_script(
                test_script_bytes.clone(),
                vec![],
                Vec::<Vec<u8>>::new(),
                &mut UnmeteredGasMeter,
                &mut TraversalContext::new(&traversal_storage),
                &module_and_script_storage,
                &module_and_script_storage,
            ));
            module_and_script_storage.assert_verified_script_entry_exists_for(&test_script_bytes);
        }
    }
}
