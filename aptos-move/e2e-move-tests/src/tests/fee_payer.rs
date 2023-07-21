// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, tests::common, MoveHarness};
use aptos::move_tool::MemberId;
use aptos_language_e2e_tests::{account::TransactionBuilder, transaction_status_eq};
use aptos_types::{
    account_address::AccountAddress,
    account_config::CoinStoreResource,
    on_chain_config::FeatureFlag,
    transaction::{EntryFunction, Script, TransactionArgument, TransactionStatus},
};
use move_core_types::{move_resource::MoveStructType, vm_status::StatusCode};

fn read_coin(h: &MoveHarness, account: &AccountAddress) -> u64 {
    h.read_resource::<CoinStoreResource>(account, CoinStoreResource::struct_tag())
        .unwrap()
        .coin()
}

#[test]
fn test_two_to_two_transfer_fee_payer() {
    let mut h = MoveHarness::new_with_features(vec![FeatureFlag::GAS_PAYER_ENABLED], vec![]);

    let alice = h.new_account_at(AccountAddress::from_hex_literal("0xa11ce").unwrap());
    let bob = h.new_account_at(AccountAddress::from_hex_literal("0xb0b").unwrap());
    let carol = h.new_account_at(AccountAddress::from_hex_literal("0xca501").unwrap());
    let david = h.new_account_at(AccountAddress::from_hex_literal("0xda51d").unwrap());
    let payer = h.new_account_at(AccountAddress::from_hex_literal("0xea51d").unwrap());

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

    let package = aptos_framework::BuiltPackage::build(
        common::test_dir_path("../../../move-examples/scripts/two_by_two_transfer"),
        build_options,
    )
    .expect("building package must succeed");

    let alice_start = read_coin(&h, alice.address());
    let bob_start = read_coin(&h, bob.address());
    let carol_start = read_coin(&h, carol.address());
    let david_start = read_coin(&h, david.address());
    let payer_start = read_coin(&h, payer.address());

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
        .fee_payer(payer.clone())
        .script(script)
        .sequence_number(h.sequence_number(alice.address()))
        .max_gas_amount(1_000_000)
        .gas_unit_price(1)
        .sign_fee_payer();

    let output = h.executor.execute_transaction(transaction);
    assert_success!(output.status().to_owned());
    h.executor.apply_write_set(output.write_set());

    let alice_end = read_coin(&h, alice.address());
    let bob_end = read_coin(&h, bob.address());
    let carol_end = read_coin(&h, carol.address());
    let david_end = read_coin(&h, david.address());
    let payer_end = read_coin(&h, payer.address());

    // Make sure sender alice doesn't pay gas
    assert_eq!(alice_start - amount_alice, alice_end);
    assert_eq!(bob_start - amount_bob, bob_end);
    assert_eq!(carol_start + amount_carol, carol_end);
    assert_eq!(david_start + amount_david, david_end);
    // Make sure payer pays
    assert_eq!(payer_start - output.gas_used(), payer_end);
}

#[test]
fn test_two_to_two_transfer_fee_payer_is_sender() {
    let mut h = MoveHarness::new_with_features(vec![FeatureFlag::GAS_PAYER_ENABLED], vec![]);

    let alice = h.new_account_at(AccountAddress::from_hex_literal("0xa11ce").unwrap());
    let bob = h.new_account_at(AccountAddress::from_hex_literal("0xb0b").unwrap());
    let carol = h.new_account_at(AccountAddress::from_hex_literal("0xca501").unwrap());
    let david = h.new_account_at(AccountAddress::from_hex_literal("0xda51d").unwrap());
    let payer = h.new_account_at(AccountAddress::from_hex_literal("0xea51d").unwrap());

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

    let package = aptos_framework::BuiltPackage::build(
        common::test_dir_path("../../../move-examples/scripts/two_by_two_transfer"),
        build_options,
    )
    .expect("building package must succeed");

    let alice_start = read_coin(&h, alice.address());
    let bob_start = read_coin(&h, bob.address());
    let carol_start = read_coin(&h, carol.address());
    let david_start = read_coin(&h, david.address());
    let payer_start = read_coin(&h, payer.address());

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
        .fee_payer(alice.clone())
        .script(script)
        .sequence_number(h.sequence_number(alice.address()))
        .max_gas_amount(1_000_000)
        .gas_unit_price(1)
        .sign_fee_payer();

    let output = h.executor.execute_transaction(transaction);
    assert_success!(output.status().to_owned());
    h.executor.apply_write_set(output.write_set());

    let alice_end = read_coin(&h, alice.address());
    let bob_end = read_coin(&h, bob.address());
    let carol_end = read_coin(&h, carol.address());
    let david_end = read_coin(&h, david.address());
    let payer_end = read_coin(&h, payer.address());

    // Make sure alice pays gas
    assert_eq!(alice_start - amount_alice - output.gas_used(), alice_end);
    assert_eq!(bob_start - amount_bob, bob_end);
    assert_eq!(carol_start + amount_carol, carol_end);
    assert_eq!(david_start + amount_david, david_end);
    // Make sure payer doesn't pays
    assert_eq!(payer_start, payer_end);
}

#[test]
fn test_two_to_two_transfer_fee_payer_without_feature() {
    let mut h = MoveHarness::new_with_features(vec![], vec![FeatureFlag::GAS_PAYER_ENABLED]);

    let alice = h.new_account_at(AccountAddress::from_hex_literal("0xa11ce").unwrap());
    let bob = h.new_account_at(AccountAddress::from_hex_literal("0xb0b").unwrap());
    let carol = h.new_account_at(AccountAddress::from_hex_literal("0xca501").unwrap());
    let david = h.new_account_at(AccountAddress::from_hex_literal("0xda51d").unwrap());
    let payer = h.new_account_at(AccountAddress::from_hex_literal("0xea51d").unwrap());

    let amount_alice = 100;
    let amount_bob = 200;
    let amount_carol = 50;

    let build_options = aptos_framework::BuildOptions {
        with_srcs: false,
        with_abis: false,
        with_source_maps: false,
        with_error_map: false,
        ..aptos_framework::BuildOptions::default()
    };

    let package = aptos_framework::BuiltPackage::build(
        common::test_dir_path("../../../move-examples/scripts/two_by_two_transfer"),
        build_options,
    )
    .expect("building package must succeed");

    let code = package.extract_script_code()[0].clone();
    let script = Script::new(code, vec![], vec![
        TransactionArgument::U64(amount_alice),
        TransactionArgument::U64(amount_bob),
        TransactionArgument::Address(*carol.address()),
        TransactionArgument::Address(*david.address()),
        TransactionArgument::U64(amount_carol),
    ]);

    let transaction = TransactionBuilder::new(alice.clone())
        .secondary_signers(vec![bob])
        .fee_payer(payer)
        .script(script)
        .sequence_number(h.sequence_number(alice.address()))
        .max_gas_amount(1_000_000)
        .gas_unit_price(1)
        .sign_fee_payer();

    let output = h.executor.execute_transaction(transaction);
    assert!(transaction_status_eq(
        output.status(),
        &TransactionStatus::Discard(StatusCode::UNEXPECTED_ERROR_FROM_KNOWN_MOVE_FUNCTION)
    ));
}

#[test]
fn test_normal_tx_with_signer_with_fee_payer() {
    let mut h = MoveHarness::new_with_features(vec![FeatureFlag::GAS_PAYER_ENABLED], vec![]);

    let alice = h.new_account_at(AccountAddress::from_hex_literal("0xa11ce").unwrap());
    let bob = h.new_account_at(AccountAddress::from_hex_literal("0xb0b").unwrap());

    let alice_start = read_coin(&h, alice.address());
    let bob_start = read_coin(&h, bob.address());

    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    assert_success!(h.publish_package(&acc, &common::test_dir_path("string_args.data/pack")));

    let fun: MemberId = str::parse("0xcafe::test::hi").unwrap();
    let entry = EntryFunction::new(fun.module_id, fun.member_id, vec![], vec![bcs::to_bytes(
        &"Hi",
    )
    .unwrap()]);
    let transaction = TransactionBuilder::new(alice.clone())
        .fee_payer(bob.clone())
        .entry_function(entry)
        .sequence_number(h.sequence_number(alice.address()))
        .max_gas_amount(1_000_000)
        .gas_unit_price(1)
        .sign_fee_payer();

    let output = h.executor.execute_transaction(transaction);
    // The last signer became the gas payer and thus, the execution errors with a mismatch
    // between required signers as parameters and signers passed in.
    assert_success!(output.status().to_owned());
    h.executor.apply_write_set(output.write_set());

    let alice_after = read_coin(&h, alice.address());
    let bob_after = read_coin(&h, bob.address());

    assert_eq!(alice_start, alice_after);
    assert!(bob_start > bob_after);
}

#[test]
fn test_normal_tx_without_signer_with_fee_payer() {
    let mut h = MoveHarness::new_with_features(vec![FeatureFlag::GAS_PAYER_ENABLED], vec![]);

    let alice = h.new_account_at(AccountAddress::from_hex_literal("0xa11ce").unwrap());
    let bob = h.new_account_at(AccountAddress::from_hex_literal("0xb0b").unwrap());

    let alice_start = read_coin(&h, alice.address());
    let bob_start = read_coin(&h, bob.address());

    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    assert_success!(h.publish_package(&acc, &common::test_dir_path("string_args.data/pack")));

    let fun: MemberId = str::parse("0xcafe::test::nothing").unwrap();
    let entry = EntryFunction::new(fun.module_id, fun.member_id, vec![], vec![]);
    let transaction = TransactionBuilder::new(alice.clone())
        .fee_payer(bob.clone())
        .entry_function(entry)
        .sequence_number(h.sequence_number(alice.address()))
        .max_gas_amount(1_000_000)
        .gas_unit_price(1)
        .sign_fee_payer();

    let output = h.executor.execute_transaction(transaction);
    // The last signer became the gas payer and thus, the execution errors with a mismatch
    // between required signers as parameters and signers passed in.
    assert_success!(output.status().to_owned());
    h.executor.apply_write_set(output.write_set());

    let alice_after = read_coin(&h, alice.address());
    let bob_after = read_coin(&h, bob.address());

    assert_eq!(alice_start, alice_after);
    assert!(bob_start > bob_after);
}

#[test]
fn test_normal_tx_with_fee_payer_insufficient_funds() {
    let mut h = MoveHarness::new_with_features(vec![FeatureFlag::GAS_PAYER_ENABLED], vec![]);

    let alice = h.new_account_at(AccountAddress::from_hex_literal("0xa11ce").unwrap());
    let bob = h.new_account_with_balance_and_sequence_number(1, 0);

    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    assert_success!(h.publish_package(&acc, &common::test_dir_path("string_args.data/pack")));

    let fun: MemberId = str::parse("0xcafe::test::nothing").unwrap();
    let entry = EntryFunction::new(fun.module_id, fun.member_id, vec![], vec![]);
    let transaction = TransactionBuilder::new(alice.clone())
        .fee_payer(bob)
        .entry_function(entry)
        .sequence_number(h.sequence_number(alice.address()))
        .max_gas_amount(1_000_000)
        .gas_unit_price(1)
        .sign_fee_payer();

    let output = h.executor.execute_transaction(transaction);
    assert!(transaction_status_eq(
        output.status(),
        &TransactionStatus::Discard(StatusCode::INSUFFICIENT_BALANCE_FOR_TRANSACTION_FEE)
    ));
}
