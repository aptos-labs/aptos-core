// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::compiler::{as_module, compile_units};
use move_core_types::{
    account_address::AccountAddress,
    identifier::Identifier,
    language_storage::ModuleId,
    value::{serialize_values, MoveValue},
    vm_status::StatusType,
};
use move_vm_runtime::{module_traversal::*, move_vm::MoveVM, IntoUnsyncModuleStorage};
use move_vm_test_utils::{BlankStorage, InMemoryStorage};
use move_vm_types::gas::UnmeteredGasMeter;

const TEST_ADDR: AccountAddress = AccountAddress::new([42; AccountAddress::LENGTH]);

#[test]
fn call_non_existent_module() {
    let vm = MoveVM::new(vec![]);
    let storage = BlankStorage;
    let module_storage = storage
        .clone()
        .into_unsync_module_storage(vm.runtime_environment());

    let mut sess = vm.new_session(&storage);
    let module_id = ModuleId::new(TEST_ADDR, Identifier::new("M").unwrap());
    let fun_name = Identifier::new("foo").unwrap();
    let traversal_storage = TraversalStorage::new();

    let err = sess
        .execute_function_bypass_visibility(
            &module_id,
            &fun_name,
            vec![],
            serialize_values(&vec![MoveValue::Signer(TEST_ADDR)]),
            &mut UnmeteredGasMeter,
            &mut TraversalContext::new(&traversal_storage),
            &module_storage,
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
    storage.add_module_bytes(m.self_addr(), m.self_name(), blob.into());

    let vm = MoveVM::new(vec![]);
    let mut sess = vm.new_session(&storage);

    let fun_name = Identifier::new("foo").unwrap();

    let traversal_storage = TraversalStorage::new();

    let module_storage = storage
        .clone()
        .into_unsync_module_storage(vm.runtime_environment());
    let err = sess
        .execute_function_bypass_visibility(
            &m.self_id(),
            &fun_name,
            vec![],
            serialize_values(&vec![MoveValue::Signer(TEST_ADDR)]),
            &mut UnmeteredGasMeter,
            &mut TraversalContext::new(&traversal_storage),
            &module_storage,
        )
        .unwrap_err();
    assert_eq!(err.status_type(), StatusType::Verification);
}
