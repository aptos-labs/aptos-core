// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{compiler::compile_modules_in_file, tests::execute_function_for_test};
use move_binary_format::{
    file_format::{
        empty_module, AddressIdentifierIndex, Bytecode, CodeUnit, FunctionDefinition,
        FunctionHandle, FunctionHandleIndex, IdentifierIndex, ModuleHandle, ModuleHandleIndex,
        SignatureIndex, StructHandle, StructTypeParameter, TableIndex, Visibility,
    },
    CompiledModule,
};
use move_core_types::{
    ability::AbilitySet, account_address::AccountAddress, ident_str, identifier::Identifier,
    language_storage::ModuleId,
};
use move_vm_runtime::{AsUnsyncModuleStorage, ModuleStorage, StagingModuleStorage};
use move_vm_test_utils::InMemoryStorage;
use std::path::PathBuf;

const WORKING_ACCOUNT: AccountAddress = AccountAddress::TWO;

struct Adapter {
    store: InMemoryStorage,
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

        Self { store, functions }
    }

    fn publish_modules_using_loader_v2<'a, M: ModuleStorage>(
        &'a self,
        module_storage: &'a M,
        modules: Vec<CompiledModule>,
    ) -> StagingModuleStorage<M> {
        let module_bundle = modules
            .into_iter()
            .map(|module| {
                let mut binary = vec![];
                module
                    .serialize(&mut binary)
                    .unwrap_or_else(|_| panic!("failure in module serialization: {:#?}", module));
                binary.into()
            })
            .collect();
        StagingModuleStorage::create(&WORKING_ACCOUNT, module_storage, module_bundle)
            .expect("failure publishing modules")
    }

    fn call_functions(&self, module_storage: &impl ModuleStorage) {
        for (module_id, name) in &self.functions {
            execute_function_for_test(&self.store, module_storage, module_id, name, &[], vec![])
                .unwrap_or_else(|_| panic!("Failure executing {:?}::{:?}", module_id, name));
        }
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
    let adapter = Adapter::new(data_store);
    let modules = get_modules();

    // calls all functions sequentially
    let module_storage = InMemoryStorage::new().into_unsync_module_storage();
    let module_storage = adapter.publish_modules_using_loader_v2(&module_storage, modules);
    adapter.call_functions(&module_storage);
}

#[test]
fn load_phantom_module() {
    let data_store = InMemoryStorage::new();
    let adapter = Adapter::new(data_store);

    let mut module = empty_module();
    module.address_identifiers[0] = WORKING_ACCOUNT;
    module.identifiers[0] = Identifier::new("I").unwrap();
    module.identifiers.push(Identifier::new("H").unwrap());
    module.module_handles.push(ModuleHandle {
        address: AddressIdentifierIndex(0),
        name: IdentifierIndex((module.identifiers.len() - 1) as TableIndex),
    });
    module.identifiers.push(Identifier::new("S").unwrap());
    module.struct_handles.push(StructHandle {
        module: ModuleHandleIndex((module.module_handles.len() - 1) as TableIndex),
        name: IdentifierIndex((module.identifiers.len() - 1) as TableIndex),
        abilities: AbilitySet::EMPTY,
        type_parameters: vec![StructTypeParameter {
            constraints: AbilitySet::EMPTY,
            is_phantom: false,
        }],
    });

    module.identifiers.push(Identifier::new("foo").unwrap());
    module.function_handles.push(FunctionHandle {
        module: ModuleHandleIndex(0),
        name: IdentifierIndex((module.identifiers.len() - 1) as TableIndex),
        parameters: SignatureIndex(0),
        return_: SignatureIndex(0),
        type_parameters: vec![],
        access_specifiers: None,
        attributes: vec![],
    });
    module.function_defs.push(FunctionDefinition {
        function: FunctionHandleIndex(0),
        visibility: Visibility::Private,
        is_entry: false,
        acquires_global_resources: vec![],
        code: Some(CodeUnit {
            locals: SignatureIndex(0),
            code: vec![Bytecode::Ret],
        }),
    });

    let mut modules = get_modules();
    let module_id = module.self_id();
    modules.push(module);

    let module_storage = InMemoryStorage::new().into_unsync_module_storage();
    let new_module_storage = adapter.publish_modules_using_loader_v2(&module_storage, modules);

    let _ = new_module_storage
        .load_function(&module_id, ident_str!("foo"), &[])
        .unwrap();
}

#[test]
fn load_with_extra_ability() {
    let data_store = InMemoryStorage::new();
    let adapter = Adapter::new(data_store);

    let mut module = empty_module();
    module.address_identifiers[0] = WORKING_ACCOUNT;
    module.identifiers[0] = Identifier::new("I").unwrap();
    module.identifiers.push(Identifier::new("H").unwrap());
    module.module_handles.push(ModuleHandle {
        address: AddressIdentifierIndex(0),
        name: IdentifierIndex((module.identifiers.len() - 1) as TableIndex),
    });
    module.identifiers.push(Identifier::new("F").unwrap());

    // Publish a module where a struct has COPY ability at definition site and EMPTY ability at use site.
    // This should be OK due to our module upgrade rule.
    module.struct_handles.push(StructHandle {
        module: ModuleHandleIndex((module.module_handles.len() - 1) as TableIndex),
        name: IdentifierIndex((module.identifiers.len() - 1) as TableIndex),
        abilities: AbilitySet::EMPTY,
        type_parameters: vec![StructTypeParameter {
            constraints: AbilitySet::EMPTY,
            is_phantom: false,
        }],
    });

    module.identifiers.push(Identifier::new("foo").unwrap());
    module.function_handles.push(FunctionHandle {
        module: ModuleHandleIndex(0),
        name: IdentifierIndex((module.identifiers.len() - 1) as TableIndex),
        parameters: SignatureIndex(0),
        return_: SignatureIndex(0),
        type_parameters: vec![],
        access_specifiers: None,
        attributes: vec![],
    });
    module.function_defs.push(FunctionDefinition {
        function: FunctionHandleIndex(0),
        visibility: Visibility::Private,
        is_entry: false,
        acquires_global_resources: vec![],
        code: Some(CodeUnit {
            locals: SignatureIndex(0),
            code: vec![Bytecode::Ret],
        }),
    });

    let mut modules = get_modules();
    let module_id = module.self_id();
    modules.push(module);

    let module_storage = InMemoryStorage::new().into_unsync_module_storage();
    let new_module_storage = adapter.publish_modules_using_loader_v2(&module_storage, modules);

    let _ = new_module_storage
        .load_function(&module_id, ident_str!("foo"), &[])
        .unwrap();
}

#[test]
fn deep_dependency_list_ok_0() {
    let data_store = InMemoryStorage::new();
    let adapter = Adapter::new(data_store);

    let mut modules = vec![];

    // create a chain of dependencies
    let max = 100u64;
    dependency_chain(1, max, &mut modules);
    let name = format!("A{}", max);
    let dep_name = format!("A{}", max - 1);
    let deps = vec![dep_name];
    let module = empty_module_with_dependencies(name, deps);

    let module_storage = InMemoryStorage::new().into_unsync_module_storage();
    let module_storage = adapter.publish_modules_using_loader_v2(&module_storage, modules);
    adapter.publish_modules_using_loader_v2(&module_storage, vec![module]);
}

#[test]
fn deep_dependency_list_ok_1() {
    let data_store = InMemoryStorage::new();
    let adapter = Adapter::new(data_store);

    let mut modules = vec![];

    // create a chain of dependencies
    let max = 30u64;
    dependency_chain(1, max, &mut modules);
    let name = format!("A{}", max);
    let dep_name = format!("A{}", max - 1);
    let deps = vec![dep_name];
    let module = empty_module_with_dependencies(name, deps);

    let module_storage = InMemoryStorage::new().into_unsync_module_storage();
    let module_storage = adapter.publish_modules_using_loader_v2(&module_storage, modules);
    adapter.publish_modules_using_loader_v2(&module_storage, vec![module]);
}

#[test]
fn deep_friend_list_ok_0() {
    let data_store = InMemoryStorage::new();
    let adapter = Adapter::new(data_store);

    let mut modules = vec![];

    // create a chain of friends
    let max = 100u64;
    friend_chain(1, max, &mut modules);
    let name = format!("A{}", max);
    let dep_name = format!("A{}", max - 1);
    let deps = vec![dep_name];
    let module = empty_module_with_friends(name, deps);

    let module_storage = InMemoryStorage::new().into_unsync_module_storage();
    let module_storage = adapter.publish_modules_using_loader_v2(&module_storage, modules);
    adapter.publish_modules_using_loader_v2(&module_storage, vec![module]);
}

#[test]
fn deep_friend_list_ok_1() {
    let data_store = InMemoryStorage::new();
    let adapter = Adapter::new(data_store);

    let mut modules = vec![];

    // create a chain of friends
    let max = 30u64;
    friend_chain(1, max, &mut modules);
    let name = format!("A{}", max);
    let dep_name = format!("A{}", max - 1);
    let deps = vec![dep_name];
    let module = empty_module_with_friends(name, deps);

    let module_storage = InMemoryStorage::new().into_unsync_module_storage();
    let module_storage = adapter.publish_modules_using_loader_v2(&module_storage, modules);
    adapter.publish_modules_using_loader_v2(&module_storage, vec![module]);
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
