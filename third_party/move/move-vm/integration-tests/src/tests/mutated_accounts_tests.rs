// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::compiler::{as_module, compile_units};
use move_core_types::{
    account_address::AccountAddress,
    identifier::Identifier,
    value::{serialize_values, MoveValue},
};
use move_vm_runtime::{
    module_traversal::*, move_vm::MoveVM, AsUnsyncModuleStorage, RuntimeEnvironment,
};
use move_vm_test_utils::InMemoryStorage;
use move_vm_types::gas::UnmeteredGasMeter;

const TEST_ADDR: AccountAddress = AccountAddress::new([42; AccountAddress::LENGTH]);

#[test]
fn mutated_accounts() {
    let code = r#"
        module {{ADDR}}::M {
            struct Foo has key { a: bool }
            public fun get(addr: address): bool acquires Foo {
                borrow_global<Foo>(addr).a
            }
            public fun flip(addr: address) acquires Foo {
                let f_ref = borrow_global_mut<Foo>(addr);
                f_ref.a = !f_ref.a;
            }
            public fun publish(addr: &signer) {
                move_to(addr, Foo { a: true} )
            }
        }
    "#;

    let code = code.replace("{{ADDR}}", &format!("0x{}", TEST_ADDR.to_hex()));
    let mut units = compile_units(&code).unwrap();
    let m = as_module(units.pop().unwrap());
    let mut blob = vec![];
    m.serialize(&mut blob).unwrap();

    let mut storage = InMemoryStorage::new();
    storage.add_module_bytes(m.self_addr(), m.self_name(), blob.into());

    let runtime_environment = RuntimeEnvironment::new(vec![]);
    let vm = MoveVM::new(vec![]);
    let mut sess = vm.new_session(&storage);

    let publish = Identifier::new("publish").unwrap();
    let flip = Identifier::new("flip").unwrap();
    let get = Identifier::new("get").unwrap();

    let account1 = AccountAddress::random();
    let traversal_storage = TraversalStorage::new();

    let module_storage = storage.as_unsync_module_storage(&runtime_environment);
    sess.execute_function_bypass_visibility(
        &m.self_id(),
        &publish,
        vec![],
        serialize_values(&vec![MoveValue::Signer(account1)]),
        &mut UnmeteredGasMeter,
        &mut TraversalContext::new(&traversal_storage),
        &module_storage,
    )
    .unwrap();

    // The resource was published to "account1" and the sender's account
    // (TEST_ADDR) is assumed to be mutated as well (e.g., in a subsequent
    // transaction epilogue).
    assert_eq!(sess.num_mutated_resources(&TEST_ADDR), 2);

    sess.execute_function_bypass_visibility(
        &m.self_id(),
        &get,
        vec![],
        serialize_values(&vec![MoveValue::Address(account1)]),
        &mut UnmeteredGasMeter,
        &mut TraversalContext::new(&traversal_storage),
        &module_storage,
    )
    .unwrap();

    assert_eq!(sess.num_mutated_resources(&TEST_ADDR), 2);

    sess.execute_function_bypass_visibility(
        &m.self_id(),
        &flip,
        vec![],
        serialize_values(&vec![MoveValue::Address(account1)]),
        &mut UnmeteredGasMeter,
        &mut TraversalContext::new(&traversal_storage),
        &module_storage,
    )
    .unwrap();
    assert_eq!(sess.num_mutated_resources(&TEST_ADDR), 2);

    let changes = sess.finish(&module_storage).unwrap();
    storage.apply(changes).unwrap();

    let mut sess = vm.new_session(&storage);
    let module_storage = storage.as_unsync_module_storage(&runtime_environment);
    sess.execute_function_bypass_visibility(
        &m.self_id(),
        &get,
        vec![],
        serialize_values(&vec![MoveValue::Address(account1)]),
        &mut UnmeteredGasMeter,
        &mut TraversalContext::new(&traversal_storage),
        &module_storage,
    )
    .unwrap();

    // Only the sender's account (TEST_ADDR) should have been modified.
    assert_eq!(sess.num_mutated_resources(&TEST_ADDR), 1);
}
