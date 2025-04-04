// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    compiler::{as_module, compile_units},
    tests::execute_function_with_single_storage_for_test,
};
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    identifier::Identifier,
    language_storage::ModuleId,
    value::{serialize_values, MoveValue},
    vm_status::StatusType,
};
use move_vm_test_utils::InMemoryStorage;

const TEST_ADDR: AccountAddress = AccountAddress::new([42; AccountAddress::LENGTH]);

#[test]
fn call_non_existent_module() {
    let storage = InMemoryStorage::new();
    let module_id = ModuleId::new(TEST_ADDR, Identifier::new("M").unwrap());

    let args = serialize_values(&vec![MoveValue::Signer(TEST_ADDR)]);
    let err = execute_function_with_single_storage_for_test(
        &storage,
        &module_id,
        ident_str!("foo"),
        &[],
        args,
    )
    .unwrap_err();

    assert_eq!(err.status_type(), StatusType::Verification);
}

#[test]
fn call_non_existent_function() {
    let code = r#"
        module {{ADDR}}::M {}
    "#;
    let code = code.replace("{{ADDR}}", &format!("0x{}", TEST_ADDR.to_hex()));

    let mut units = compile_units(&code).unwrap();
    let m = as_module(units.pop().unwrap());
    let mut blob = vec![];
    m.serialize(&mut blob).unwrap();

    let mut storage = InMemoryStorage::new();
    let module_id = ModuleId::new(TEST_ADDR, Identifier::new("M").unwrap());
    storage.add_module_bytes(module_id.address(), module_id.name(), blob.into());

    let args = serialize_values(&vec![MoveValue::Signer(TEST_ADDR)]);
    let err = execute_function_with_single_storage_for_test(
        &storage,
        &module_id,
        ident_str!("foo"),
        &[],
        args,
    )
    .unwrap_err();
    assert_eq!(err.status_type(), StatusType::Verification);
}
