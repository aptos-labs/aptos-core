// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::compiler::{as_module, compile_units};
use move_binary_format::errors::VMResult;
use move_core_types::{
    account_address::AccountAddress,
    identifier::Identifier,
    language_storage::TypeTag,
    value::{MoveTypeLayout, MoveValue},
};
use move_vm_runtime::{
    module_traversal::*, move_vm::MoveVM, session::SerializedReturnValues, AsUnsyncModuleStorage,
    RuntimeEnvironment,
};
use move_vm_test_utils::InMemoryStorage;
use move_vm_types::gas::UnmeteredGasMeter;

const TEST_ADDR: AccountAddress = AccountAddress::new([42; AccountAddress::LENGTH]);

fn run(
    structs: &[&str],
    fun_sig: &str,
    fun_body: &str,
    ty_args: Vec<TypeTag>,
    args: Vec<MoveValue>,
) -> VMResult<Vec<Vec<u8>>> {
    let structs = structs.to_vec().join("\n");

    let code = format!(
        r#"
        module 0x{}::M {{
            {}

            fun foo{} {{
                {}
            }}
        }}
    "#,
        TEST_ADDR.to_hex(),
        structs,
        fun_sig,
        fun_body
    );

    let mut units = compile_units(&code).unwrap();
    let m = as_module(units.pop().unwrap());
    let mut blob = vec![];
    m.serialize(&mut blob).unwrap();

    let mut storage = InMemoryStorage::new();
    storage.add_module_bytes(m.self_addr(), m.self_name(), blob.into());

    let runtime_environment = RuntimeEnvironment::new(vec![]);
    let vm = MoveVM::new_with_runtime_environment(&runtime_environment);
    let mut sess = vm.new_session(&storage);

    let fun_name = Identifier::new("foo").unwrap();

    let args: Vec<_> = args
        .into_iter()
        .map(|val| val.simple_serialize().unwrap())
        .collect();

    let module_storage = storage.as_unsync_module_storage(runtime_environment);
    let traversal_storage = TraversalStorage::new();

    let SerializedReturnValues {
        return_values,
        mutable_reference_outputs: _,
    } = sess.execute_function_bypass_visibility(
        &m.self_id(),
        &fun_name,
        ty_args,
        args,
        &mut UnmeteredGasMeter,
        &mut TraversalContext::new(&traversal_storage),
        &module_storage,
    )?;

    Ok(return_values
        .into_iter()
        .map(|(bytes, _layout)| bytes)
        .collect())
}

fn expect_success(
    structs: &[&str],
    fun_sig: &str,
    fun_body: &str,
    ty_args: Vec<TypeTag>,
    args: Vec<MoveValue>,
    expected_layouts: &[MoveTypeLayout],
) {
    let return_vals = run(structs, fun_sig, fun_body, ty_args, args).unwrap();
    assert!(return_vals.len() == expected_layouts.len());

    for (blob, layout) in return_vals.iter().zip(expected_layouts.iter()) {
        MoveValue::simple_deserialize(blob, layout).unwrap();
    }
}

#[test]
fn return_nothing() {
    expect_success(&[], "()", "", vec![], vec![], &[])
}

#[test]
fn return_u64() {
    expect_success(&[], "(): u64", "42", vec![], vec![], &[MoveTypeLayout::U64])
}

#[test]
fn return_u64_bool() {
    expect_success(&[], "(): (u64, bool)", "(42, true)", vec![], vec![], &[
        MoveTypeLayout::U64,
        MoveTypeLayout::Bool,
    ])
}

#[test]
fn return_signer_ref() {
    expect_success(
        &[],
        "(s: &signer): &signer",
        "s",
        vec![],
        vec![MoveValue::Signer(TEST_ADDR)],
        &[MoveTypeLayout::Signer],
    )
}
