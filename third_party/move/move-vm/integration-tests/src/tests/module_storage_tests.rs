// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use bytes::Bytes;
use claims::{assert_err, assert_none, assert_ok, assert_some};
use move_binary_format::{
    file_format::{empty_module_with_dependencies_and_friends, empty_script_with_dependencies},
    file_format_common::VERSION_DEFAULT,
    CompiledModule,
};
use move_core_types::{
    account_address::AccountAddress, ident_str, identifier::Identifier, language_storage::ModuleId,
    vm_status::StatusCode,
};
use move_vm_runtime::{AsUnsyncCodeStorage, AsUnsyncModuleStorage, CodeStorage, ModuleStorage};
use move_vm_test_utils::InMemoryStorage;
use move_vm_types::sha3_256;

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
