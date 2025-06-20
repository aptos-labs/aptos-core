// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Transactional tests for comparison operations, Lt/Le/Ge/Gt, over non-integer types,
//! introduced in Move language version 2.2 and onwards.

use crate::{assert_success, tests::common, MoveHarness};
use aptos_framework::{BuildOptions, BuiltPackage};
use aptos_language_e2e_tests::account::TransactionBuilder;
use aptos_types::{account_address::AccountAddress, transaction::Script};

#[test]
fn function_generic_cmp() {
    let mut h = MoveHarness::new();

    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x99").unwrap());
    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("cmp_generic.data/pack"),
        BuildOptions::move_2().set_latest_language()
    ));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x99::primitive_cmp::test_bool").unwrap(),
        vec![],
        vec![],
    ));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x99::primitive_cmp::test_address").unwrap(),
        vec![],
        vec![],
    ));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x99::primitive_cmp::test_vector").unwrap(),
        vec![],
        vec![],
    ));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x99::struct_cmp::test_simple_struct").unwrap(),
        vec![],
        vec![],
    ));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x99::struct_cmp::test_complex_struct").unwrap(),
        vec![],
        vec![],
    ));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x99::struct_cmp::test_nested_complex_struct").unwrap(),
        vec![],
        vec![],
    ));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x99::struct_cmp::test_special_complex_struct").unwrap(),
        vec![],
        vec![],
    ));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x99::generic_cmp::test_generic_arg").unwrap(),
        vec![],
        vec![],
    ));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x99::generic_cmp::test_generic_struct").unwrap(),
        vec![],
        vec![],
    ));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x99::generic_cmp::test_generic_complex_struct").unwrap(),
        vec![],
        vec![],
    ));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x99::function_value_cmp::test_module_name_cmp").unwrap(),
        vec![],
        vec![],
    ));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x99::function_value_cmp::test_function_name_cmp").unwrap(),
        vec![],
        vec![],
    ));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x99::function_value_cmp::test_typed_arg_cmp").unwrap(),
        vec![],
        vec![],
    ));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x99::function_value_cmp::test_captured_var_cmp").unwrap(),
        vec![],
        vec![],
    ));
}

/// Special case of comparing two signers
#[test]
fn function_signer_cmp() {
    let mut h = MoveHarness::new();

    let alice = h.new_account_at(AccountAddress::from_hex_literal("0xa11ce").unwrap());
    let bob = h.new_account_at(AccountAddress::from_hex_literal("0xb0b01").unwrap());

    let build_options = BuildOptions {
        with_srcs: false,
        with_abis: false,
        with_source_maps: false,
        with_error_map: false,
        ..BuildOptions::move_2().set_latest_language()
    };

    let package = BuiltPackage::build(
        common::test_dir_path("cmp_generic.data/script"),
        build_options,
    )
    .expect("building package must succeed");

    let code = package.extract_script_code()[0].clone();
    let script = Script::new(code, vec![], vec![]);

    let transaction = TransactionBuilder::new(alice.clone())
        .secondary_signers(vec![bob.clone()])
        .script(script)
        .sequence_number(h.sequence_number(alice.address()))
        .max_gas_amount(1_000_000)
        .gas_unit_price(1)
        .sign_multi_agent();

    let output = h.executor.execute_transaction(transaction);
    assert_success!(output.status().to_owned());
}
