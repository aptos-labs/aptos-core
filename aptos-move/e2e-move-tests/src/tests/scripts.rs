// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, build_package, tests::common, MoveHarness};
use aptos_language_e2e_tests::account::TransactionBuilder;
use aptos_types::{
    account_address::AccountAddress,
    transaction::{Script, TransactionArgument},
};
use move_core_types::language_storage::TypeTag;

#[test]
fn test_script_with_type_parameter() {
    let mut h = MoveHarness::new();

    let alice = h.new_account_at(AccountAddress::from_hex_literal("0xa11ce").unwrap());

    let package = build_package(
        common::test_dir_path("script_with_ty_param.data/pack"),
        aptos_framework::BuildOptions::default(),
    )
    .expect("building package must succeed");

    let code = package.extract_script_code().into_iter().next().unwrap();

    let txn = TransactionBuilder::new(alice.clone())
        .script(Script::new(
            code,
            std::iter::repeat_with(|| TypeTag::U64).take(33).collect(),
            vec![],
        ))
        .sequence_number(10)
        .max_gas_amount(1_000_000)
        .gas_unit_price(1)
        .sign();

    let status = h.run(txn);
    assert_success!(status);
}

#[test]
fn test_two_to_two_transfer() {
    let mut h = MoveHarness::new();

    let alice = h.new_account_at(AccountAddress::from_hex_literal("0xa11ce").unwrap());
    let bob = h.new_account_at(AccountAddress::from_hex_literal("0xb0b").unwrap());
    let carol = h.new_account_at(AccountAddress::from_hex_literal("0xca501").unwrap());
    let david = h.new_account_at(AccountAddress::from_hex_literal("0xda51d").unwrap());

    let amount_alice = 100;
    let amount_bob = 200;
    let amount_carol = 50;
    let amount_david = amount_alice + amount_bob - amount_carol;

    let build_options = aptos_framework::BuildOptions {
        with_srcs: false,
        with_abis: false,
        with_source_maps: false,
        with_error_map: false,
        ..aptos_framework::BuildOptions::default()
    };

    let package = build_package(
        common::test_dir_path("../../../move-examples/scripts/two_by_two_transfer"),
        build_options,
    )
    .expect("building package must succeed");

    let alice_start = read_coin(&h, alice.address());
    let bob_start = read_coin(&h, bob.address());
    let carol_start = read_coin(&h, carol.address());
    let david_start = read_coin(&h, david.address());

    let code = package.extract_script_code()[0].clone();
    let script = Script::new(code, vec![], vec![
        TransactionArgument::U64(amount_alice),
        TransactionArgument::U64(amount_bob),
        TransactionArgument::Address(*carol.address()),
        TransactionArgument::Address(*david.address()),
        TransactionArgument::U64(amount_carol),
    ]);

    let transaction = TransactionBuilder::new(alice.clone())
        .secondary_signers(vec![bob.clone()])
        .script(script)
        .sequence_number(h.sequence_number(alice.address()))
        .max_gas_amount(1_000_000)
        .gas_unit_price(1)
        .sign_multi_agent();

    let output = h.executor.execute_transaction(transaction);
    assert_success!(output.status().to_owned());
    h.executor.apply_write_set(output.write_set());

    let alice_end = read_coin(&h, alice.address());
    let bob_end = read_coin(&h, bob.address());
    let carol_end = read_coin(&h, carol.address());
    let david_end = read_coin(&h, david.address());

    assert_eq!(alice_start - amount_alice - output.gas_used(), alice_end);
    assert_eq!(bob_start - amount_bob, bob_end);
    assert_eq!(carol_start + amount_carol, carol_end);
    assert_eq!(david_start + amount_david, david_end);
}

fn read_coin(h: &MoveHarness, account: &AccountAddress) -> u64 {
    h.read_aptos_balance(account)
}
