// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_abort, assert_success, build_package, tests::common, MoveHarness};
use aptos_framework::BuiltPackage;
use aptos_language_e2e_tests::account::{Account, TransactionBuilder};
use aptos_types::{
    account_address::AccountAddress,
    move_utils::{as_move_value::AsMoveValue, MemberId},
    on_chain_config::OnChainConfig,
    randomness::PerBlockRandomness,
    transaction::{ExecutionStatus, Script, TransactionStatus},
};
use claims::{assert_gt, assert_lt, assert_ok};
use move_core_types::{
    ident_str,
    language_storage::ModuleId,
    value::{serialize_values, MoveValue},
    vm_status::AbortLocation,
};

// Error codes from randomness module.
const E_API_USE_SUSCEPTIBLE_TO_TEST_AND_ABORT: u64 = 1;

#[test]
fn test_and_abort_defense_is_sound_and_correct() {
    let mut h = MoveHarness::new();

    // These scripts call a public entry function and a public function. The randomness API will reject both calls.
    for dir in [
        "randomness_unsafe_public_entry.data/pack",
        "randomness_unsafe_public.data/pack",
    ] {
        println!("Testing {dir}");
        // This will redeploy the package, so backwards compatibility must be maintained in these directories.
        let (_, package) =
            deploy_code(AccountAddress::ONE, dir, &mut h).expect("building package must succeed");

        let status = run_script(&mut h, &package);
        assert_abort!(status, E_API_USE_SUSCEPTIBLE_TO_TEST_AND_ABORT);
    }

    // The randomness module is initialized, but the randomness seed is not set.
    set_randomness_seed(&mut h);

    // This is a safe call that the randomness API should allow through.
    let status = run_entry_func(
        &mut h,
        "0xa11ce",
        "0x1::some_randapp::safe_private_entry_call",
    );
    assert_success!(status);

    // This is a safe call that the randomness API should allow through.
    // (I suppose that, since TXNs with private entry function payloads are okay, increasing the
    // visibility to public(friend) should not create any problems.)
    let status = run_entry_func(
        &mut h,
        "0xa11ce",
        "0x1::some_randapp::safe_friend_entry_call",
    );
    assert_success!(status);
}

#[test]
fn test_only_private_entry_function_can_be_annotated() {
    // Make sure building a package fails.
    let mut h = MoveHarness::new();
    assert!(deploy_code(
        AccountAddress::ONE,
        "randomness.data/invalid_pack_non_entry",
        &mut h
    )
    .is_err());
    assert!(deploy_code(
        AccountAddress::ONE,
        "randomness.data/invalid_pack_public_entry",
        &mut h
    )
    .is_err());
}

#[test]
fn test_unbiasable_annotation() {
    let mut h = MoveHarness::new();
    deploy_code(AccountAddress::ONE, "randomness.data/pack", &mut h)
        .expect("building package must succeed");
    set_randomness_seed(&mut h);

    let should_succeed = [
        "0x1::test::ok_if_not_annotated_and_not_using_randomness",
        "0x1::test::ok_if_annotated_and_not_using_randomness",
        "0x1::test::ok_if_annotated_and_using_randomness",
    ];

    for entry_func in should_succeed {
        let status = run_entry_func(&mut h, "0xa11ce", entry_func);
        assert_success!(status);
    }

    // Non-annotated functions which use randomness fail at runtime.
    let entry_func = "0x1::test::fail_if_not_annotated_and_using_randomness";
    let status = run_entry_func(&mut h, "0xa11ce", entry_func);
    let status = assert_ok!(status.as_kept_status());

    if let ExecutionStatus::MoveAbort {
        location,
        code,
        info: _,
    } = status
    {
        assert_eq!(
            location,
            AbortLocation::Module(ModuleId::new(
                AccountAddress::ONE,
                ident_str!("randomness").to_owned()
            ))
        );
        assert_eq!(code, E_API_USE_SUSCEPTIBLE_TO_TEST_AND_ABORT);
    } else {
        unreachable!("Non-annotated entry call function should result in Move abort")
    }
}

#[test]
fn test_undergas_attack_prevention() {
    let mut h = MoveHarness::new();
    deploy_code(AccountAddress::ONE, "randomness.data/pack", &mut h)
        .expect("building package must succeed");
    set_randomness_seed(&mut h);

    h.set_default_gas_unit_price(1);

    // A function to send some amount to 2 people where how to split between the 2 is randomized.
    let func: MemberId = str::parse("0x1::test::transfer_lucky_money").unwrap();
    let recipient_0 = h.new_account_with_balance_and_sequence_number(0, 11);
    let recipient_1 = h.new_account_with_balance_and_sequence_number(0, 12);

    // A txn should be discarded if the sender balance is not enough to pay required deposit: 0.01 APT or 1_000_000 octa.
    let sender = h.new_account_with_balance_and_sequence_number(999_999, 123);
    let args = vec![
        1000_u64.as_move_value(),
        MoveValue::Address(*recipient_0.address()),
        MoveValue::Address(*recipient_1.address()),
    ];
    let status = h.run_entry_function(&sender, func.clone(), vec![], serialize_values(&args));
    assert!(status.is_discarded());
    assert_eq!(999_999_u64, h.read_aptos_balance(sender.address()));
    assert_eq!(0, h.read_aptos_balance(recipient_0.address()));
    assert_eq!(0, h.read_aptos_balance(recipient_1.address()));

    // A txn should abort but be kept if the sender doesn't have enough balance to complete the transfer.
    let sender = h.new_account_with_balance_and_sequence_number(1_001_000_000, 456);
    let args = vec![
        1_000_000_999_u64.as_move_value(), // 999 more than what sender has after prologue.
        MoveValue::Address(*recipient_0.address()),
        MoveValue::Address(*recipient_1.address()),
    ];
    let status = h.run_entry_function(&sender, func.clone(), vec![], serialize_values(&args));
    let status = assert_ok!(status.as_kept_status());
    assert!(matches!(status, ExecutionStatus::MoveAbort { .. }));
    let sender_balance = h.read_aptos_balance(sender.address());
    assert_gt!(sender_balance, 1_000_000); // At least the locked amount will be returned.
    assert_lt!(sender_balance, 1_001_000_000); // Sender lost gas fee.
    assert_eq!(0, h.read_aptos_balance(recipient_0.address()));
    assert_eq!(0, h.read_aptos_balance(recipient_1.address()));

    // Otherwise, the txn should finish normally.
    let sender = h.new_account_with_balance_and_sequence_number(1_001_000_000, 789);
    let args = vec![
        500_000_000_u64.as_move_value(), // half of what sender has after prologue.
        MoveValue::Address(*recipient_0.address()),
        MoveValue::Address(*recipient_1.address()),
    ];
    let status = h.run_entry_function(&sender, func.clone(), vec![], serialize_values(&args));
    let status = assert_ok!(status.as_kept_status());
    assert!(matches!(status, ExecutionStatus::Success));
    let sender_balance = h.read_aptos_balance(sender.address());
    assert_gt!(sender_balance, 1_000_000); // At least the locked amount will be returned.
    assert_lt!(sender_balance, 501_000_000); // Sender lost 500_000_000 + gas fee.
    assert_eq!(
        500_000_000,
        h.read_aptos_balance(recipient_0.address()) + h.read_aptos_balance(recipient_1.address())
    );
}

fn set_randomness_seed(h: &mut MoveHarness) {
    let fx = h.aptos_framework_account();
    let mut pbr = h
        .read_resource::<PerBlockRandomness>(fx.address(), PerBlockRandomness::struct_tag())
        .unwrap();
    assert!(pbr.seed.is_none());

    pbr.seed = Some((0..32).map(|_| 0u8).collect::<Vec<u8>>());
    assert_eq!(pbr.seed.as_ref().unwrap().len(), 32);
    h.set_resource(*fx.address(), PerBlockRandomness::struct_tag(), &pbr);
}

fn deploy_code(
    addr: AccountAddress,
    code_path: &str,
    harness: &mut MoveHarness,
) -> anyhow::Result<(Account, BuiltPackage)> {
    let account = harness.new_account_at(addr);

    let package = build_package(
        common::test_dir_path(code_path),
        aptos_framework::BuildOptions::default(),
    )?;

    let txn = harness.create_publish_built_package(&account, &package, |_| {});

    assert_success!(harness.run(txn));
    Ok((account, package))
}

fn run_script(h: &mut MoveHarness, package: &BuiltPackage) -> TransactionStatus {
    let alice = h.new_account_at(AccountAddress::from_hex_literal("0xa11ce").unwrap());
    let scripts = package.extract_script_code();
    let code = scripts[0].clone();

    let txn = TransactionBuilder::new(alice.clone())
        .script(Script::new(code, vec![], vec![]))
        .sequence_number(10)
        .max_gas_amount(1_000_000)
        .gas_unit_price(1)
        .sign();

    h.run(txn)
}

fn run_entry_func(h: &mut MoveHarness, signer: &str, name: &str) -> TransactionStatus {
    let alice = h.new_account_at(AccountAddress::from_hex_literal(signer).unwrap());

    println!("Running entry function '{name}'");
    h.run_entry_function(&alice, str::parse(name).unwrap(), vec![], vec![])
}
