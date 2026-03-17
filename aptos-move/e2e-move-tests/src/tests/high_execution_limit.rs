// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! End-to-end tests for the high-execution-limit transaction feature.
//!
//! Design principles:
//!  * Tests run with a greatly-reduced flat fee (1_000 octas) so accounts don't need huge balances.
//!  * Slots are still 10 per epoch (the on-chain default), so exhaustion tests can drain them.
//!  * A regular entry-function payload wrapped in TransactionExtraConfig::V2 is used to trigger
//!    the high-execution-limit path; real compute-heavy work is not required for most assertions.

use crate::{assert_success, assert_vm_status, tests::common, MoveHarness};
use aptos_cached_packages::aptos_stdlib::aptos_coin_transfer;
use aptos_framework::BuildOptions;
use aptos_language_e2e_tests::account::{Account, TransactionBuilder};
use aptos_package_builder::PackageBuilder;
use aptos_types::{
    account_address::AccountAddress,
    account_config::AggregatorResource,
    on_chain_config::FeatureFlag,
    transaction::{
        EntryFunction, ExecutionStatus, SignedTransaction, TransactionExecutable,
        TransactionExtraConfig, TransactionPayload, TransactionPayloadInner, TransactionStatus,
    },
};
use move_core_types::{
    ident_str, language_storage::ModuleId, parser::parse_struct_tag, vm_status::StatusCode,
};
use serde::Deserialize;
use std::str::FromStr;

const HIGH_BALANCE: u64 = 10_000_000_000_000;

const MAX_PER_EPOCH: u64 = 10;

#[derive(Deserialize)]
struct HighExecutionLimitConfig {
    available: AggregatorResource<u64>,
    #[allow(dead_code)]
    max_per_epoch: u64,
}

fn high_execution_transactions_available(h: &MoveHarness) -> u64 {
    *h.read_resource::<HighExecutionLimitConfig>(
        &AccountAddress::ONE,
        parse_struct_tag("0x1::high_execution_limit::HighExecutionLimitConfig").unwrap(),
    )
    .expect("HighExecutionLimitConfig resource not found")
    .available
    .get()
}

fn build_high_execution_limit_txn_payload(acc: &Account) -> TransactionPayload {
    let payload = aptos_coin_transfer(*acc.address(), 0);
    match payload {
        TransactionPayload::EntryFunction(entry_func) => {
            TransactionPayload::Payload(TransactionPayloadInner::V1 {
                executable: TransactionExecutable::EntryFunction(entry_func),
                extra_config: TransactionExtraConfig::V2 {
                    multisig_address: None,
                    replay_protection_nonce: None,
                    high_execution_limit_request: true,
                },
            })
        },
        _ => unreachable!("Coin transfer is an entry-function payload"),
    }
}

fn build_high_execution_limit_txn(h: &mut MoveHarness, acc: &Account) -> SignedTransaction {
    let payload = build_high_execution_limit_txn_payload(acc);
    h.create_transaction_payload(acc, payload)
}

#[test]
fn test_high_execution_limit_txn_disabled() {
    let mut h = MoveHarness::new_with_features(vec![], vec![
        FeatureFlag::HIGH_EXECUTION_LIMIT_TRANSACTIONS,
    ]);
    let acc = h.new_account_with_balance_and_sequence_number(HIGH_BALANCE, 0);

    let txn = build_high_execution_limit_txn(&mut h, &acc);
    let output = h.run_raw(txn);
    match output.status() {
        TransactionStatus::Discard(s) => {
            assert_eq!(*s, StatusCode::FEATURE_UNDER_GATING);
        },
        _ => panic!("Transaction should be discarded"),
    }
}

#[test]
fn test_successful_high_execution_limit_txn() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_with_balance_and_sequence_number(HIGH_BALANCE, 0);

    let before = high_execution_transactions_available(&mut h);
    let txn = build_high_execution_limit_txn(&mut h, &acc);
    assert_success!(h.run(txn));
    assert_eq!(high_execution_transactions_available(&mut h), before - 1);
}

#[test]
fn test_regular_txn_unaffected() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_with_balance_and_sequence_number(HIGH_BALANCE, 0);

    assert_eq!(high_execution_transactions_available(&mut h), MAX_PER_EPOCH);
    let txn = build_high_execution_limit_txn(&mut h, &acc);
    assert_success!(h.run(txn));
    assert_eq!(
        high_execution_transactions_available(&mut h),
        MAX_PER_EPOCH - 1
    );

    let payload = aptos_coin_transfer(*acc.address(), 0);
    assert_success!(h.run_transaction_payload(&acc, payload));
    assert_eq!(
        high_execution_transactions_available(&mut h),
        MAX_PER_EPOCH - 1
    );
}

#[test]
fn test_high_execution_limit_txn_availability() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_with_balance_and_sequence_number(HIGH_BALANCE, 0);

    for _ in 0..MAX_PER_EPOCH {
        let txn = build_high_execution_limit_txn(&mut h, &acc);
        assert_success!(h.run(txn));
    }
    assert_eq!(high_execution_transactions_available(&mut h), 0);

    // Transaction must be rejected in the prologue phase.
    let txn = build_high_execution_limit_txn(&mut h, &acc);
    let output = h.run_raw(txn);
    assert!(matches!(
        output.status(),
        TransactionStatus::Discard(StatusCode::HIGH_EXECUTION_LIMIT_COUNTER_EXHAUSTED)
    ));
    assert_eq!(high_execution_transactions_available(&mut h), 0);

    h.new_epoch();
    assert_eq!(high_execution_transactions_available(&mut h), MAX_PER_EPOCH);

    let acc = h.new_account_with_balance_and_sequence_number(HIGH_BALANCE, 0);
    let txn = build_high_execution_limit_txn(&mut h, &acc);
    assert_success!(h.run(txn));
    assert_eq!(
        high_execution_transactions_available(&mut h),
        MAX_PER_EPOCH - 1
    );
}

#[test]
fn test_high_execution_limit_fee_charged_on_success() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_with_balance_and_sequence_number(HIGH_BALANCE, 0);

    let before = h.read_aptos_balance(acc.address());
    let txn = build_high_execution_limit_txn(&mut h, &acc);
    let output = h.run_raw(txn);
    assert_success!(output.status().clone());
    let after = h.read_aptos_balance(acc.address());

    let high_execution_limit_fee = u64::from(h.get_gas_params().1.vm.txn.high_execution_limit_fee);
    assert_eq!(
        before - after,
        high_execution_limit_fee + output.gas_used() * h.default_gas_unit_price,
    );
}

#[test]
fn test_high_execution_limit_fee_charged_on_out_of_gas() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_with_balance_and_sequence_number(HIGH_BALANCE, 0);

    let code_addr = AccountAddress::from_str("0xbeef").unwrap();
    let code_acc = h.new_account_with_balance_at(code_addr, HIGH_BALANCE);
    assert_success!(h.publish_package_cache_building(
        &code_acc,
        &common::test_dir_path("infinite_loop.data/empty_loop"),
    ));

    let before = high_execution_transactions_available(&mut h);
    let balance_before = h.read_aptos_balance(acc.address());

    let payload = TransactionPayload::Payload(TransactionPayloadInner::V1 {
        executable: TransactionExecutable::EntryFunction(EntryFunction::new(
            ModuleId::new(code_addr, ident_str!("test").to_owned()),
            ident_str!("run").to_owned(),
            vec![],
            vec![],
        )),
        extra_config: TransactionExtraConfig::V2 {
            multisig_address: None,
            replay_protection_nonce: None,
            high_execution_limit_request: true,
        },
    });
    let txn = h.create_transaction_payload(&acc, payload);
    let output = h.run_raw(txn);

    assert_vm_status!(output.status().clone(), StatusCode::EXECUTION_LIMIT_REACHED);
    assert_eq!(high_execution_transactions_available(&mut h), before - 1);

    let balance_after = h.read_aptos_balance(acc.address());
    let high_execution_limit_fee = u64::from(h.get_gas_params().1.vm.txn.high_execution_limit_fee);
    assert_eq!(
        balance_before - balance_after,
        high_execution_limit_fee + output.gas_used() * h.default_gas_unit_price,
    );
}

#[test]
fn test_high_execution_limit_fee_charged_on_move_abort() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_with_balance_and_sequence_number(HIGH_BALANCE, 0);

    let mut builder = PackageBuilder::new("Package");
    let source = format!(
        "module {}::foo {{ public entry fun bar() {{ abort 33 }} }}",
        acc.address().to_hex_literal()
    );
    builder.add_source("m.move", &source);
    let path = builder.write_to_temp().unwrap();
    assert_success!(h.publish_package_with_options(&acc, path.path(), BuildOptions::move_2()));

    let before = high_execution_transactions_available(&mut h);
    let balance_before = h.read_aptos_balance(acc.address());

    let payload = TransactionPayload::Payload(TransactionPayloadInner::V1 {
        executable: TransactionExecutable::EntryFunction(EntryFunction::new(
            ModuleId::new(*acc.address(), ident_str!("foo").to_owned()),
            ident_str!("bar").to_owned(),
            vec![],
            vec![],
        )),
        extra_config: TransactionExtraConfig::V2 {
            multisig_address: None,
            replay_protection_nonce: None,
            high_execution_limit_request: true,
        },
    });
    let txn = h.create_transaction_payload(&acc, payload);
    let output = h.run_raw(txn);
    assert!(matches!(
        output.status().clone(),
        TransactionStatus::Keep(ExecutionStatus::MoveAbort { .. })
    ));

    assert_eq!(high_execution_transactions_available(&mut h), before - 1);

    let balance_after = h.read_aptos_balance(acc.address());
    let high_execution_limit_fee = u64::from(h.get_gas_params().1.vm.txn.high_execution_limit_fee);
    assert_eq!(
        balance_before - balance_after,
        high_execution_limit_fee + output.gas_used() * h.default_gas_unit_price,
    );
}

#[test]
fn test_high_execution_limit_fee_charged_for_fee_payer() {
    let mut h = MoveHarness::new();
    let sender = h.new_account_with_balance_and_sequence_number(0, 0);
    let payer = h.new_account_with_balance_and_sequence_number(HIGH_BALANCE, 0);

    let payload = build_high_execution_limit_txn_payload(&sender);
    let txn = TransactionBuilder::new(sender.clone())
        .fee_payer(payer.clone())
        .payload(payload)
        .sequence_number(0)
        .max_gas_amount(500_000_000)
        .gas_unit_price(1)
        .sign_fee_payer();

    let payer_balance_before = h.read_aptos_balance(payer.address());
    let sender_balance_before = h.read_aptos_balance(sender.address());

    let output = h.run_raw(txn);
    assert_success!(output.status().to_owned());

    let payer_balance_after = h.read_aptos_balance(payer.address());
    let sender_balance_after = h.read_aptos_balance(sender.address());
    assert_eq!(sender_balance_before, sender_balance_after);
    assert_eq!(
        high_execution_transactions_available(&mut h),
        MAX_PER_EPOCH - 1
    );

    let high_execution_limit_fee = u64::from(h.get_gas_params().1.vm.txn.high_execution_limit_fee);
    assert_eq!(
        payer_balance_before - payer_balance_after,
        // Gas unit price was set to 1.
        high_execution_limit_fee + output.gas_used(),
    );
}

#[test]
fn test_insufficient_balance_for_high_execution_limit_fee() {
    let mut h = MoveHarness::new();

    // Not enough to cover the fee.
    let max_gas_amount = 500_000;
    let gas_unit_price = 1;
    let acc = h.new_account_with_balance_and_sequence_number(max_gas_amount * gas_unit_price, 0);

    let payload = build_high_execution_limit_txn_payload(&acc);
    let txn = TransactionBuilder::new(acc.clone())
        .payload(payload)
        .sequence_number(0)
        .max_gas_amount(max_gas_amount)
        .gas_unit_price(gas_unit_price)
        .sign();

    assert!(matches!(
        h.run_raw(txn).status(),
        TransactionStatus::Discard(StatusCode::INSUFFICIENT_BALANCE_FOR_TRANSACTION_FEE)
    ),);
    assert_eq!(high_execution_transactions_available(&mut h), MAX_PER_EPOCH);

    let txn = TransactionBuilder::new(acc.clone())
        .payload(aptos_coin_transfer(*acc.address(), 0))
        .sequence_number(0)
        .max_gas_amount(max_gas_amount)
        .gas_unit_price(gas_unit_price)
        .sign();
    assert_success!(h.run(txn));
}
