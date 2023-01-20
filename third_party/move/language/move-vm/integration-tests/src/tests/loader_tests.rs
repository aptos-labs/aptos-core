// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::compiler::compile_modules_in_file;
use move_binary_format::{
    file_format::{
        empty_module, AddressIdentifierIndex, IdentifierIndex, ModuleHandle, TableIndex,
    },
    CompiledModule,
};
use move_bytecode_verifier::VerifierConfig;
use move_core_types::{
    account_address::AccountAddress,
    identifier::{IdentStr, Identifier},
    language_storage::ModuleId,
};
use move_vm_runtime::{config::VMConfig, move_vm::MoveVM};
use move_vm_test_utils::InMemoryStorage;
use move_vm_types::gas::UnmeteredGasMeter;

use std::{path::PathBuf, sync::Arc, thread};

const WORKING_ACCOUNT: AccountAddress = AccountAddress::TWO;

struct Adapter {
    store: InMemoryStorage,
    vm: Arc<MoveVM>,
    functions: Vec<(ModuleId, Identifier)>,
}

impl Adapter {
    fn new(store: InMemoryStorage) -> Self {
        let functions = vec![
            (
                ModuleId::new(WORKING_ACCOUNT, Identifier::new("A").unwrap()),
                Identifier::new("entry_a").unwrap(),
            ),
            (
                ModuleId::new(WORKING_ACCOUNT, Identifier::new("D").unwrap()),
                Identifier::new("entry_d").unwrap(),
            ),
            (
                ModuleId::new(WORKING_ACCOUNT, Identifier::new("E").unwrap()),
                Identifier::new("entry_e").unwrap(),
            ),
            (
                ModuleId::new(WORKING_ACCOUNT, Identifier::new("F").unwrap()),
                Identifier::new("entry_f").unwrap(),
            ),
            (
                ModuleId::new(WORKING_ACCOUNT, Identifier::new("C").unwrap()),
                Identifier::new("just_c").unwrap(),
            ),
        ];
        let config = VMConfig {
            verifier: VerifierConfig {
                max_dependency_depth: Some(100),
                ..Default::default()
            },
            ..Default::default()
        };
        Self {
            store,
            vm: Arc::new(MoveVM::new_with_config(vec![], config).unwrap()),
            functions,
        }
    }

    fn fresh(self) -> Self {
        let config = VMConfig {
            verifier: VerifierConfig {
                max_dependency_depth: Some(100),
                ..Default::default()
            },
            ..Default::default()
        };
        Self {
            store: self.store,
            vm: Arc::new(MoveVM::new_with_config(vec![], config).unwrap()),
            functions: self.functions,
        }
    }

    fn publish_modules(&mut self, modules: Vec<CompiledModule>) {
        let mut session = self.vm.new_session(&self.store);

        for module in modules {
            let mut binary = vec![];
            module
                .serialize(&mut binary)
                .unwrap_or_else(|_| panic!("failure in module serialization: {:#?}", module));
            session
                .publish_module(binary, WORKING_ACCOUNT, &mut UnmeteredGasMeter)
                .unwrap_or_else(|_| panic!("failure publishing module: {:#?}", module));
        }
        let (changeset, _) = session.finish().expect("failure getting write set");
        self.store
            .apply(changeset)
            .expect("failure applying write set");
    }

    fn publish_modules_with_error(&mut self, modules: Vec<CompiledModule>) {
        let mut session = self.vm.new_session(&self.store);

        for module in modules {
            let mut binary = vec![];
            module
                .serialize(&mut binary)
                .unwrap_or_else(|_| panic!("failure in module serialization: {:#?}", module));
            session
                .publish_module(binary, WORKING_ACCOUNT, &mut UnmeteredGasMeter)
                .expect_err("publishing must fail");
        }
    }

    fn call_functions(&self) {
        for (module_id, name) in &self.functions {
            self.call_function(module_id, name);
        }
    }

    fn call_functions_async(&self, reps: usize) {
        let mut children = vec![];
        for _ in 0..reps {
            for (module_id, name) in self.functions.clone() {
                let vm = self.vm.clone();
                let data_store = self.store.clone();
                children.push(thread::spawn(move || {
                    let mut session = vm.new_session(&data_store);
                    session
                        .execute_function_bypass_visibility(
                            &module_id,
                            &name,
                            vec![],
                            Vec::<Vec<u8>>::new(),
                            &mut UnmeteredGasMeter,
                        )
                        .unwrap_or_else(|_| {
                            panic!("Failure executing {:?}::{:?}", module_id, name)
                        });
                }));
            }
        }
        for child in children {
            let _ = child.join();
        }
    }

    fn call_function(&self, module: &ModuleId, name: &IdentStr) {
        let mut session = self.vm.new_session(&self.store);
        session
            .execute_function_bypass_visibility(
                module,
                name,
                vec![],
                Vec::<Vec<u8>>::new(),
                &mut UnmeteredGasMeter,
            )
            .unwrap_or_else(|_| panic!("Failure executing {:?}::{:?}", module, name));
    }
}

fn get_modules() -> Vec<CompiledModule> {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("src/tests/loader_tests_modules.move");
    compile_modules_in_file(&path).unwrap()
}

#[test]
fn load() {
    let data_store = InMemoryStorage::new();
    let mut adapter = Adapter::new(data_store);
    let modules = get_modules();
    adapter.publish_modules(modules);
    // calls all functions sequentially
    adapter.call_functions();
}

#[test]
fn load_concurrent() {
    let data_store = InMemoryStorage::new();
    let mut adapter = Adapter::new(data_store);
    let modules = get_modules();
    adapter.publish_modules(modules);
    // makes 15 threads
    adapter.call_functions_async(3);
}

#[test]
fn load_concurrent_many() {
    let data_store = InMemoryStorage::new();
    let mut adapter = Adapter::new(data_store);
    let modules = get_modules();
    adapter.publish_modules(modules);
    // makes 150 threads
    adapter.call_functions_async(30);
}

#[test]
fn deep_dependency_list_err_0() {
    let data_store = InMemoryStorage::new();
    let mut adapter = Adapter::new(data_store);

    let mut modules = vec![];

    // create a chain of dependencies
    let max = 350u64;
    dependency_chain(1, max, &mut modules);
    adapter.publish_modules(modules);

    let mut adapter = adapter.fresh();
    let name = format!("A{}", max);
    let dep_name = format!("A{}", max - 1);
    let deps = vec![dep_name];
    let module = empty_module_with_dependencies(name, deps);
    adapter.publish_modules_with_error(vec![module]);
}

#[test]
fn deep_dependency_list_err_1() {
    let data_store = InMemoryStorage::new();
    let mut adapter = Adapter::new(data_store);

    let mut modules = vec![];

    // create a chain of dependencies
    let max = 101u64;
    dependency_chain(1, max, &mut modules);
    adapter.publish_modules(modules);

    let mut adapter = adapter.fresh();
    let name = format!("A{}", max);
    let dep_name = format!("A{}", max - 1);
    let deps = vec![dep_name];
    let module = empty_module_with_dependencies(name, deps);
    adapter.publish_modules_with_error(vec![module]);
}

#[test]
fn deep_dependency_list_ok_0() {
    let data_store = InMemoryStorage::new();
    let mut adapter = Adapter::new(data_store);

    let mut modules = vec![];

    // create a chain of dependencies
    let max = 100u64;
    dependency_chain(1, max, &mut modules);
    adapter.publish_modules(modules);

    let mut adapter = adapter.fresh();
    let name = format!("A{}", max);
    let dep_name = format!("A{}", max - 1);
    let deps = vec![dep_name];
    let module = empty_module_with_dependencies(name, deps);
    adapter.publish_modules(vec![module]);
}

#[test]
fn deep_dependency_list_ok_1() {
    let data_store = InMemoryStorage::new();
    let mut adapter = Adapter::new(data_store);

    let mut modules = vec![];

    // create a chain of dependencies
    let max = 30u64;
    dependency_chain(1, max, &mut modules);
    adapter.publish_modules(modules);

    let mut adapter = adapter.fresh();
    let name = format!("A{}", max);
    let dep_name = format!("A{}", max - 1);
    let deps = vec![dep_name];
    let module = empty_module_with_dependencies(name, deps);
    adapter.publish_modules(vec![module]);
}

#[test]
fn deep_dependency_tree_err_0() {
    let data_store = InMemoryStorage::new();
    let mut adapter = Adapter::new(data_store);

    let mut modules = vec![];

    // create a tree of dependencies
    let width = 5u64;
    let height = 101u64;
    dependency_tree(width, height, &mut modules);
    adapter.publish_modules(modules);

    // use one of the module in the tree
    let mut adapter = adapter.fresh();
    let name = "ASome".to_string();
    let dep_name = format!("A_{}_{}", height - 1, width - 1);
    let deps = vec![dep_name];
    let module = empty_module_with_dependencies(name, deps);
    adapter.publish_modules_with_error(vec![module]);
}

#[test]
fn deep_dependency_tree_err_1() {
    let data_store = InMemoryStorage::new();
    let mut adapter = Adapter::new(data_store);

    let mut modules = vec![];

    // create a tree of dependencies
    let width = 3u64;
    let height = 350u64;
    dependency_tree(width, height, &mut modules);
    adapter.publish_modules(modules);

    // use one of the module in the tree
    let mut adapter = adapter.fresh();
    let name = "ASome".to_string();
    let dep_name = format!("A_{}_{}", height - 1, width - 1);
    let deps = vec![dep_name];
    let module = empty_module_with_dependencies(name, deps);
    adapter.publish_modules_with_error(vec![module]);
}

#[test]
fn deep_dependency_tree_ok_0() {
    let data_store = InMemoryStorage::new();
    let mut adapter = Adapter::new(data_store);

    let mut modules = vec![];

    // create a tree of dependencies
    let width = 10u64;
    let height = 20u64;
    dependency_tree(width, height, &mut modules);
    adapter.publish_modules(modules);

    // use one of the module in the tree
    let mut adapter = adapter.fresh();
    let name = "ASome".to_string();
    let dep_name = format!("A_{}_{}", height - 1, width - 1);
    let deps = vec![dep_name];
    let module = empty_module_with_dependencies(name, deps);
    adapter.publish_modules(vec![module]);
}

#[test]
fn deep_dependency_tree_ok_1() {
    let data_store = InMemoryStorage::new();
    let mut adapter = Adapter::new(data_store);

    let mut modules = vec![];

    // create a tree of dependencies
    let width = 3u64;
    let height = 100u64;
    dependency_tree(width, height, &mut modules);
    adapter.publish_modules(modules);

    // use one of the module in the tree
    let mut adapter = adapter.fresh();
    let name = "ASome".to_string();
    let dep_name = format!("A_{}_{}", height - 1, width - 1);
    let deps = vec![dep_name];
    let module = empty_module_with_dependencies(name, deps);
    adapter.publish_modules(vec![module]);
}

#[test]
fn deep_friend_list_err_0() {
    let data_store = InMemoryStorage::new();
    let mut adapter = Adapter::new(data_store);

    let mut modules = vec![];

    // create a chain of dependencies
    let max = 1000u64;
    friend_chain(1, max, &mut modules);
    adapter.publish_modules(modules);

    let mut adapter = adapter.fresh();
    let name = format!("A{}", max);
    let dep_name = format!("A{}", max - 1);
    let deps = vec![dep_name];
    let module = empty_module_with_friends(name, deps);
    adapter.publish_modules_with_error(vec![module]);
}

#[test]
fn deep_friend_list_err_1() {
    let data_store = InMemoryStorage::new();
    let mut adapter = Adapter::new(data_store);

    let mut modules = vec![];

    // create a chain of dependencies
    let max = 101u64;
    friend_chain(1, max, &mut modules);
    adapter.publish_modules(modules);

    let mut adapter = adapter.fresh();
    let name = format!("A{}", max);
    let dep_name = format!("A{}", max - 1);
    let deps = vec![dep_name];
    let module = empty_module_with_friends(name, deps);
    adapter.publish_modules_with_error(vec![module]);
}

#[test]
fn deep_friend_list_ok_0() {
    let data_store = InMemoryStorage::new();
    let mut adapter = Adapter::new(data_store);

    let mut modules = vec![];

    // create a chain of dependencies
    let max = 100u64;
    friend_chain(1, max, &mut modules);
    adapter.publish_modules(modules);

    let mut adapter = adapter.fresh();
    let name = format!("A{}", max);
    let dep_name = format!("A{}", max - 1);
    let deps = vec![dep_name];
    let module = empty_module_with_friends(name, deps);
    adapter.publish_modules(vec![module]);
}

#[test]
fn deep_friend_list_ok_1() {
    let data_store = InMemoryStorage::new();
    let mut adapter = Adapter::new(data_store);

    let mut modules = vec![];

    // create a chain of dependencies
    let max = 30u64;
    friend_chain(1, max, &mut modules);
    adapter.publish_modules(modules);

    let mut adapter = adapter.fresh();
    let name = format!("A{}", max);
    let dep_name = format!("A{}", max - 1);
    let deps = vec![dep_name];
    let module = empty_module_with_friends(name, deps);
    adapter.publish_modules(vec![module]);
}

fn leaf_module(name: &str) -> CompiledModule {
    let mut module = empty_module();
    module.identifiers[0] = Identifier::new(name).unwrap();
    module.address_identifiers[0] = WORKING_ACCOUNT;
    module
}

// Create a list of dependent modules
fn dependency_chain(start: u64, end: u64, modules: &mut Vec<CompiledModule>) {
    let module = leaf_module("A0");
    modules.push(module);

    for i in start..end {
        let name = format!("A{}", i);
        let dep_name = format!("A{}", i - 1);
        let deps = vec![dep_name];
        let module = empty_module_with_dependencies(name, deps);
        modules.push(module);
    }
}

// Create a tree (well a forest or DAG really) of dependent modules
fn dependency_tree(width: u64, height: u64, modules: &mut Vec<CompiledModule>) {
    let mut deps = vec![];
    for i in 0..width {
        let name = format!("A_{}_{}", 0, i);
        let module = leaf_module(name.as_str());
        deps.push(name);
        modules.push(module);
    }
    for i in 1..height {
        let mut new_deps = vec![];
        for j in 0..width {
            let name = format!("A_{}_{}", i, j);
            let module = empty_module_with_dependencies(name.clone(), deps.clone());
            new_deps.push(name);
            modules.push(module);
        }
        deps = new_deps;
    }
}

// Create a module that uses (depends on) the list of given modules
fn empty_module_with_dependencies(name: String, deps: Vec<String>) -> CompiledModule {
    let mut module = empty_module();
    module.address_identifiers[0] = WORKING_ACCOUNT;
    module.identifiers[0] = Identifier::new(name).unwrap();
    for dep in deps {
        module.identifiers.push(Identifier::new(dep).unwrap());
        module.module_handles.push(ModuleHandle {
            address: AddressIdentifierIndex(0),
            name: IdentifierIndex((module.identifiers.len() - 1) as TableIndex),
        });
    }
    module
}

// Create a list of friends modules
fn friend_chain(start: u64, end: u64, modules: &mut Vec<CompiledModule>) {
    let module = leaf_module("A0");
    modules.push(module);

    for i in start..end {
        let name = format!("A{}", i);
        let dep_name = format!("A{}", i - 1);
        let deps = vec![dep_name];
        let module = empty_module_with_friends(name, deps);
        modules.push(module);
    }
}

// Create a module that uses (friends on) the list of given modules
fn empty_module_with_friends(name: String, deps: Vec<String>) -> CompiledModule {
    let mut module = empty_module();
    module.address_identifiers[0] = WORKING_ACCOUNT;
    module.identifiers[0] = Identifier::new(name).unwrap();
    for dep in deps {
        module.identifiers.push(Identifier::new(dep).unwrap());
        module.friend_decls.push(ModuleHandle {
            address: AddressIdentifierIndex(0),
            name: IdentifierIndex((module.identifiers.len() - 1) as TableIndex),
        });
    }
    module
}
