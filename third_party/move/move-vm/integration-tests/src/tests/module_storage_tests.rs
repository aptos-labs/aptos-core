// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::compiler::{as_module, compile_units};
use bytes::Bytes;
use claims::{assert_err, assert_none, assert_ok, assert_some};
use move_binary_format::{
    file_format::{empty_module_with_dependencies_and_friends, empty_script_with_dependencies},
    file_format_common::VERSION_DEFAULT,
    CompiledModule,
};
use move_core_types::{
    ability::{Ability, AbilitySet},
    account_address::AccountAddress,
    ident_str,
    identifier::Identifier,
    language_storage::{ModuleId, TypeTag},
    vm_status::StatusCode,
};
use move_vm_runtime::{
    AsFunctionValueExtension, AsUnsyncCodeStorage, AsUnsyncModuleStorage, CodeStorage,
    ModuleStorage, WithRuntimeEnvironment,
};
use move_vm_test_utils::InMemoryStorage;
use move_vm_types::{
    loaded_data::{
        runtime_types::{AbilityInfo, StructIdentifier, TypeBuilder},
        struct_name_indexing::StructNameIndex,
    },
    sha3_256,
    value_serde::FunctionValueExtension,
};
use std::str::FromStr;

#[cfg(test)]
fn module_bytes(module_code: &str) -> Bytes {
    let compiled_module = as_module(compile_units(module_code).unwrap().pop().unwrap());
    let mut bytes = vec![];
    compiled_module.serialize(&mut bytes).unwrap();
    bytes.into()
}

fn make_module<'a>(
    module_name: &'a str,
    dependencies: impl IntoIterator<Item = &'a str>,
    friends: impl IntoIterator<Item = &'a str>,
) -> (CompiledModule, Bytes) {
    let mut module = empty_module_with_dependencies_and_friends(module_name, dependencies, friends);
    module.version = VERSION_DEFAULT;

    let mut module_bytes = vec![];
    assert_ok!(module.serialize(&mut module_bytes));

    (module, module_bytes.into())
}

fn make_script<'a>(dependencies: impl IntoIterator<Item = &'a str>) -> Vec<u8> {
    let mut script = empty_script_with_dependencies(dependencies);
    script.version = VERSION_DEFAULT;

    let mut serialized_script = vec![];
    assert_ok!(script.serialize(&mut serialized_script));
    serialized_script
}

fn add_module_bytes<'a>(
    module_bytes_storage: &mut InMemoryStorage,
    module_name: &'a str,
    dependencies: impl IntoIterator<Item = &'a str>,
    friends: impl IntoIterator<Item = &'a str>,
) {
    let (module, bytes) = make_module(module_name, dependencies, friends);
    module_bytes_storage.add_module_bytes(module.self_addr(), module.self_name(), bytes);
}

#[test]
fn test_module_does_not_exist() {
    let module_storage = InMemoryStorage::new().into_unsync_module_storage();

    let result = module_storage.check_module_exists(&AccountAddress::ZERO, ident_str!("a"));
    assert!(!assert_ok!(result));

    let result = module_storage.fetch_module_size_in_bytes(&AccountAddress::ZERO, ident_str!("a"));
    assert_none!(assert_ok!(result));

    let result = module_storage.fetch_module_metadata(&AccountAddress::ZERO, ident_str!("a"));
    assert_none!(assert_ok!(result));

    let result = module_storage.fetch_deserialized_module(&AccountAddress::ZERO, ident_str!("a"));
    assert_none!(assert_ok!(result));

    let result = module_storage.fetch_verified_module(&AccountAddress::ZERO, ident_str!("a"));
    assert_none!(assert_ok!(result));
}

#[test]
fn test_module_exists() {
    let mut module_bytes_storage = InMemoryStorage::new();
    add_module_bytes(&mut module_bytes_storage, "a", vec![], vec![]);
    let id = ModuleId::new(AccountAddress::ZERO, Identifier::new("a").unwrap());

    let module_storage = module_bytes_storage.into_unsync_module_storage();

    assert!(assert_ok!(
        module_storage.check_module_exists(id.address(), id.name())
    ));
    module_storage.assert_cached_state(vec![&id], vec![]);
}

#[test]
fn test_deserialized_caching() {
    let mut module_bytes_storage = InMemoryStorage::new();

    let a_id = ModuleId::new(AccountAddress::ZERO, Identifier::new("a").unwrap());
    let c_id = ModuleId::new(AccountAddress::ZERO, Identifier::new("c").unwrap());

    add_module_bytes(&mut module_bytes_storage, "a", vec!["b", "c"], vec![]);
    add_module_bytes(&mut module_bytes_storage, "b", vec![], vec![]);
    add_module_bytes(&mut module_bytes_storage, "c", vec!["d", "e"], vec![]);
    add_module_bytes(&mut module_bytes_storage, "d", vec![], vec![]);
    add_module_bytes(&mut module_bytes_storage, "e", vec![], vec![]);

    let module_storage = module_bytes_storage.into_unsync_module_storage();

    let result = module_storage.fetch_module_metadata(a_id.address(), a_id.name());
    let expected = make_module("a", vec!["b", "c"], vec![]).0.metadata;
    assert_eq!(assert_some!(assert_ok!(result)), expected);
    module_storage.assert_cached_state(vec![&a_id], vec![]);

    let result = module_storage.fetch_deserialized_module(c_id.address(), c_id.name());
    let expected = make_module("c", vec!["d", "e"], vec![]).0;
    assert_eq!(assert_some!(assert_ok!(result)).as_ref(), &expected);
    module_storage.assert_cached_state(vec![&a_id, &c_id], vec![]);
}

#[test]
fn test_dependency_tree_traversal() {
    let mut module_bytes_storage = InMemoryStorage::new();

    let a_id = ModuleId::new(AccountAddress::ZERO, Identifier::new("a").unwrap());
    let b_id = ModuleId::new(AccountAddress::ZERO, Identifier::new("b").unwrap());
    let c_id = ModuleId::new(AccountAddress::ZERO, Identifier::new("c").unwrap());
    let d_id = ModuleId::new(AccountAddress::ZERO, Identifier::new("d").unwrap());
    let e_id = ModuleId::new(AccountAddress::ZERO, Identifier::new("e").unwrap());

    add_module_bytes(&mut module_bytes_storage, "a", vec!["b", "c"], vec![]);
    add_module_bytes(&mut module_bytes_storage, "b", vec![], vec![]);
    add_module_bytes(&mut module_bytes_storage, "c", vec!["d", "e"], vec![]);
    add_module_bytes(&mut module_bytes_storage, "d", vec![], vec![]);
    add_module_bytes(&mut module_bytes_storage, "e", vec![], vec![]);

    let module_storage = module_bytes_storage.into_unsync_module_storage();

    assert_ok!(module_storage.fetch_verified_module(c_id.address(), c_id.name()));
    module_storage.assert_cached_state(vec![], vec![&c_id, &d_id, &e_id]);

    assert_ok!(module_storage.fetch_verified_module(a_id.address(), a_id.name()));
    module_storage.assert_cached_state(vec![], vec![&a_id, &b_id, &c_id, &d_id, &e_id]);

    assert_ok!(module_storage.fetch_verified_module(a_id.address(), a_id.name()));
}

#[test]
fn test_dependency_dag_traversal() {
    let mut module_bytes_storage = InMemoryStorage::new();

    let a_id = ModuleId::new(AccountAddress::ZERO, Identifier::new("a").unwrap());
    let b_id = ModuleId::new(AccountAddress::ZERO, Identifier::new("b").unwrap());
    let c_id = ModuleId::new(AccountAddress::ZERO, Identifier::new("c").unwrap());
    let d_id = ModuleId::new(AccountAddress::ZERO, Identifier::new("d").unwrap());
    let e_id = ModuleId::new(AccountAddress::ZERO, Identifier::new("e").unwrap());
    let f_id = ModuleId::new(AccountAddress::ZERO, Identifier::new("f").unwrap());
    let g_id = ModuleId::new(AccountAddress::ZERO, Identifier::new("g").unwrap());

    add_module_bytes(&mut module_bytes_storage, "a", vec!["b", "c"], vec![]);
    add_module_bytes(&mut module_bytes_storage, "b", vec!["d"], vec![]);
    add_module_bytes(&mut module_bytes_storage, "c", vec!["d"], vec![]);
    add_module_bytes(&mut module_bytes_storage, "d", vec!["e", "f"], vec![]);
    add_module_bytes(&mut module_bytes_storage, "e", vec!["g"], vec![]);
    add_module_bytes(&mut module_bytes_storage, "f", vec!["g"], vec![]);
    add_module_bytes(&mut module_bytes_storage, "g", vec![], vec![]);

    let module_storage = module_bytes_storage.into_unsync_module_storage();

    assert_ok!(module_storage.fetch_deserialized_module(a_id.address(), a_id.name()));
    assert_ok!(module_storage.fetch_deserialized_module(c_id.address(), c_id.name()));
    module_storage.assert_cached_state(vec![&a_id, &c_id], vec![]);

    assert_ok!(module_storage.fetch_verified_module(d_id.address(), d_id.name()));
    module_storage.assert_cached_state(vec![&a_id, &c_id], vec![&d_id, &e_id, &f_id, &g_id]);

    assert_ok!(module_storage.fetch_verified_module(a_id.address(), a_id.name()));
    module_storage.assert_cached_state(vec![], vec![
        &a_id, &b_id, &c_id, &d_id, &e_id, &f_id, &g_id,
    ]);
}

#[test]
fn test_cyclic_dependencies_traversal_fails() {
    let mut module_bytes_storage = InMemoryStorage::new();

    let c_id = ModuleId::new(AccountAddress::ZERO, Identifier::new("c").unwrap());

    add_module_bytes(&mut module_bytes_storage, "a", vec!["b"], vec![]);
    add_module_bytes(&mut module_bytes_storage, "b", vec!["c"], vec![]);
    add_module_bytes(&mut module_bytes_storage, "c", vec!["a"], vec![]);

    let module_storage = module_bytes_storage.into_unsync_module_storage();

    let result = module_storage.fetch_verified_module(c_id.address(), c_id.name());
    assert_eq!(
        assert_err!(result).major_status(),
        StatusCode::CYCLIC_MODULE_DEPENDENCY
    );
}

#[test]
fn test_cyclic_friends_are_allowed() {
    let mut module_bytes_storage = InMemoryStorage::new();

    let c_id = ModuleId::new(AccountAddress::ZERO, Identifier::new("c").unwrap());

    add_module_bytes(&mut module_bytes_storage, "a", vec![], vec!["b"]);
    add_module_bytes(&mut module_bytes_storage, "b", vec![], vec!["c"]);
    add_module_bytes(&mut module_bytes_storage, "c", vec![], vec!["a"]);

    let module_storage = module_bytes_storage.into_unsync_module_storage();

    let result = module_storage.fetch_verified_module(c_id.address(), c_id.name());
    assert_ok!(result);

    // Since `c` has no dependencies, only it gets deserialized and verified.
    module_storage.assert_cached_state(vec![], vec![&c_id]);
}

#[test]
fn test_transitive_friends_are_allowed_to_be_transitive_dependencies() {
    let mut module_bytes_storage = InMemoryStorage::new();

    let a_id = ModuleId::new(AccountAddress::ZERO, Identifier::new("a").unwrap());
    let b_id = ModuleId::new(AccountAddress::ZERO, Identifier::new("b").unwrap());
    let c_id = ModuleId::new(AccountAddress::ZERO, Identifier::new("c").unwrap());

    add_module_bytes(&mut module_bytes_storage, "a", vec!["b"], vec!["d"]);
    add_module_bytes(&mut module_bytes_storage, "b", vec!["c"], vec![]);
    add_module_bytes(&mut module_bytes_storage, "c", vec![], vec![]);
    add_module_bytes(&mut module_bytes_storage, "d", vec![], vec!["c"]);

    let module_storage = module_bytes_storage.into_unsync_module_storage();

    assert_ok!(module_storage.fetch_verified_module(a_id.address(), a_id.name()));
    module_storage.assert_cached_state(vec![], vec![&a_id, &b_id, &c_id]);
}

#[test]
fn test_deserialized_script_caching() {
    let mut module_bytes_storage = InMemoryStorage::new();
    add_module_bytes(&mut module_bytes_storage, "a", vec!["b", "c"], vec![]);
    add_module_bytes(&mut module_bytes_storage, "b", vec![], vec![]);
    add_module_bytes(&mut module_bytes_storage, "c", vec![], vec![]);

    let code_storage = module_bytes_storage.into_unsync_code_storage();

    let serialized_script = make_script(vec!["a"]);
    let hash_1 = sha3_256(&serialized_script);
    assert_ok!(code_storage.deserialize_and_cache_script(&serialized_script));

    let serialized_script = make_script(vec!["b"]);
    let hash_2 = sha3_256(&serialized_script);
    assert_ok!(code_storage.deserialize_and_cache_script(&serialized_script));

    code_storage.assert_cached_state(vec![&hash_1, &hash_2], vec![]);
}

#[test]
fn test_verified_script_caching() {
    let mut module_bytes_storage = InMemoryStorage::new();

    let a_id = ModuleId::new(AccountAddress::ZERO, Identifier::new("a").unwrap());
    let b_id = ModuleId::new(AccountAddress::ZERO, Identifier::new("b").unwrap());
    let c_id = ModuleId::new(AccountAddress::ZERO, Identifier::new("c").unwrap());

    add_module_bytes(&mut module_bytes_storage, "a", vec!["b", "c"], vec![]);
    add_module_bytes(&mut module_bytes_storage, "b", vec![], vec![]);
    add_module_bytes(&mut module_bytes_storage, "c", vec![], vec![]);

    let code_storage = module_bytes_storage.into_unsync_code_storage();

    let serialized_script = make_script(vec!["a"]);
    let hash = sha3_256(&serialized_script);
    assert_ok!(code_storage.deserialize_and_cache_script(&serialized_script));

    // Nothing gets loaded into module cache.
    code_storage
        .module_storage()
        .assert_cached_state(vec![], vec![]);
    code_storage.assert_cached_state(vec![&hash], vec![]);

    assert_ok!(code_storage.verify_and_cache_script(&serialized_script));

    // Script is verified, so its dependencies are loaded into cache.
    code_storage
        .module_storage()
        .assert_cached_state(vec![], vec![&a_id, &b_id, &c_id]);
    code_storage.assert_cached_state(vec![], vec![&hash]);
}

#[test]
fn test_function_value_extension() {
    let mut module_bytes_storage = InMemoryStorage::new();

    let code = r#"
        module 0x1::test {
            struct Foo {}

            fun b() { }
            fun c(a: u64, _foo: &Foo): u64 { a }
            fun d<A: drop, C: drop>(_a: A, b: u64, _c: C): u64 { b }
        }
    "#;
    let bytes = module_bytes(code);
    let test_id = ModuleId::new(AccountAddress::ONE, Identifier::new("test").unwrap());
    module_bytes_storage.add_module_bytes(&AccountAddress::ONE, ident_str!("test"), bytes);

    let code = r#"
        module 0x1::other_test {
            struct Bar has drop {}
        }
    "#;
    let bytes = module_bytes(code);
    let other_test_id = ModuleId::new(AccountAddress::ONE, Identifier::new("other_test").unwrap());
    module_bytes_storage.add_module_bytes(&AccountAddress::ONE, ident_str!("other_test"), bytes);

    let module_storage = module_bytes_storage.into_unsync_module_storage();
    let function_value_extension = module_storage.as_function_value_extension();

    let result = function_value_extension.get_function_arg_tys(
        &ModuleId::new(AccountAddress::ONE, Identifier::new("test").unwrap()),
        ident_str!("a"),
        vec![],
    );
    assert!(result.is_err());

    let mut types = function_value_extension
        .get_function_arg_tys(&test_id, ident_str!("c"), vec![])
        .unwrap();
    assert_eq!(types.len(), 2);

    let ty_builder = TypeBuilder::with_limits(100, 100);
    let foo_ty = types.pop().unwrap();
    let name = module_storage
        .runtime_environment()
        .idx_to_struct_name_for_test(StructNameIndex::new(0))
        .unwrap();
    assert_eq!(name, StructIdentifier {
        module: test_id.clone(),
        name: Identifier::new("Foo").unwrap(),
    });
    assert_eq!(
        foo_ty,
        ty_builder
            .create_ref_ty(
                &ty_builder.create_struct_ty(
                    StructNameIndex::new(0),
                    AbilityInfo::struct_(AbilitySet::EMPTY)
                ),
                false
            )
            .unwrap()
    );
    let u64_ty = types.pop().unwrap();
    assert_eq!(u64_ty, ty_builder.create_u64_ty());

    // Generic function without type parameters  should fail.
    let result = function_value_extension.get_function_arg_tys(
        &ModuleId::new(AccountAddress::ONE, Identifier::new("test").unwrap()),
        ident_str!("d"),
        vec![],
    );
    assert!(result.is_err());

    let mut types = function_value_extension
        .get_function_arg_tys(&test_id, ident_str!("d"), vec![
            TypeTag::from_str("0x1::other_test::Bar").unwrap(),
            TypeTag::Vector(Box::new(TypeTag::U8)),
        ])
        .unwrap();
    assert_eq!(types.len(), 3);

    let vec_ty = types.pop().unwrap();
    assert_eq!(
        vec_ty,
        ty_builder
            .create_vec_ty(&ty_builder.create_u8_ty())
            .unwrap()
    );
    let u64_ty = types.pop().unwrap();
    assert_eq!(u64_ty, ty_builder.create_u64_ty());
    let bar_ty = types.pop().unwrap();
    let name = module_storage
        .runtime_environment()
        .idx_to_struct_name_for_test(StructNameIndex::new(1))
        .unwrap();
    assert_eq!(name, StructIdentifier {
        module: other_test_id,
        name: Identifier::new("Bar").unwrap(),
    });
    assert_eq!(
        bar_ty,
        ty_builder.create_struct_ty(
            StructNameIndex::new(1),
            AbilityInfo::struct_(AbilitySet::from_u8(Ability::Drop as u8).unwrap())
        )
    );
}
