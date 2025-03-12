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
<<<<<<< HEAD
use move_vm_runtime::{module_traversal::*, move_vm::MoveVM, AsUnsyncModuleStorage};
=======
>>>>>>> 35ea878580 (remove move vm session)
use move_vm_test_utils::InMemoryStorage;

const TEST_ADDR: AccountAddress = AccountAddress::new([42; AccountAddress::LENGTH]);

#[test]
fn call_non_existent_module() {
    let storage = InMemoryStorage::new();
<<<<<<< HEAD

<<<<<<< HEAD
    let mut sess = MoveVM::new_session(&storage);
=======
    let mut sess = MoveVm::new_session();
>>>>>>> 7bae6066b8 ([refactoring] Remove resolver from session, use impl in sesson_ext and respawned)
=======
>>>>>>> 35ea878580 (remove move vm session)
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

<<<<<<< HEAD
<<<<<<< HEAD
    let mut sess = MoveVM::new_session(&storage);
=======
    let mut sess = MoveVm::new_session();
>>>>>>> 7bae6066b8 ([refactoring] Remove resolver from session, use impl in sesson_ext and respawned)

    let fun_name = Identifier::new("foo").unwrap();

    let traversal_storage = TraversalStorage::new();
    let module_storage = storage.as_unsync_module_storage();

    let err = sess
        .execute_function_bypass_visibility(
            &module_id,
            &fun_name,
            vec![],
            serialize_values(&vec![MoveValue::Signer(TEST_ADDR)]),
            &mut UnmeteredGasMeter,
            &mut TraversalContext::new(&traversal_storage),
            &module_storage,
            &storage,
        )
        .unwrap_err();

=======
    let args = serialize_values(&vec![MoveValue::Signer(TEST_ADDR)]);
    let err = execute_function_with_single_storage_for_test(
        &storage,
        &module_id,
        ident_str!("foo"),
        &[],
        args,
    )
    .unwrap_err();
>>>>>>> 35ea878580 (remove move vm session)
    assert_eq!(err.status_type(), StatusType::Verification);
}
