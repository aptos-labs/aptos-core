// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Native collateral writes carry the primary balance as an aggregator
//! snapshot, which the block executor replaces with its committed value.

use crate::{assert_abort, assert_success, tests::common, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_language_e2e_tests::executor::{ExecutorMode, FakeExecutor};
use aptos_transaction_simulation::Account;
use aptos_types::{
    account_address::AccountAddress,
    move_utils::MemberId,
    on_chain_config::FeatureFlag,
    state_store::{
        native_collateral::NativeCollateral,
        state_key::{inner::CollateralScope, StateKey},
    },
    transaction::{ExecutionStatus, TransactionOutput, TransactionStatus},
};
use std::str::FromStr;

const EXCHANGE: &str = "0xcafe";

fn exchange_address() -> AccountAddress {
    AccountAddress::from_hex_literal(EXCHANGE).unwrap()
}

fn fun(name: &str) -> MemberId {
    MemberId::from_str(&format!("{EXCHANGE}::collateral_test::{name}")).unwrap()
}

fn target(i: u64) -> AccountAddress {
    AccountAddress::from_hex_literal(&format!("0xbeef{i:04x}")).unwrap()
}

fn setup(mode: ExecutorMode) -> (MoveHarness, Account) {
    let mut h =
        MoveHarness::new_with_executor(FakeExecutor::from_head_genesis().set_executor_mode(mode));
    h.enable_features(
        vec![FeatureFlag::TRADING_NATIVE, FeatureFlag::NATIVE_COLLATERAL],
        vec![],
    );
    let exchange = h.new_account_at(exchange_address());
    assert_success!(h.publish_package_with_options(
        &exchange,
        &common::test_dir_path("native_collateral.data/pack"),
        BuildOptions::move_2().set_latest_language(),
    ));
    let framework = h.aptos_framework_account();
    assert_success!(
        h.run_entry_function(&framework, fun("register"), vec![], vec![bcs::to_bytes(
            &exchange_address()
        )
        .unwrap(),])
    );
    assert_success!(h.run_entry_function(&exchange, fun("init"), vec![], vec![]));
    (h, exchange)
}

/// The single collateral write in `output`, at `account`'s cross scope.
fn collateral_written(output: &TransactionOutput, account: AccountAddress) -> NativeCollateral {
    assert_eq!(
        output.status(),
        &TransactionStatus::Keep(ExecutionStatus::Success)
    );
    let writes: Vec<_> = output.write_set().native_collateral_iter().collect();
    assert_eq!(writes.len(), 1);
    let (key, op) = writes[0];
    assert_eq!(
        key,
        &StateKey::collateral(exchange_address(), account, CollateralScope::Cross)
    );
    NativeCollateral::deserialize(op.as_write_op().bytes().unwrap()).unwrap()
}

/// Every transaction credits the same aggregator and writes it as collateral.
/// Each write must hold the balance as of that transaction's position in the
/// block, so the snapshot resolved at commit, not at speculative execution.
#[test]
fn credits_in_one_block_write_the_running_balance() {
    let (mut h, _) = setup(ExecutorMode::BothComparison);
    let senders: Vec<_> = (0..20).map(|_| h.new_account_with_key_pair()).collect();

    let mut balance = 0i64;
    for block in 0..2u64 {
        let txns = senders
            .iter()
            .enumerate()
            .map(|(i, sender)| {
                let amount = block * 100 + i as u64 + 1;
                h.create_entry_function(sender, fun("credit"), vec![], vec![
                    bcs::to_bytes(&target(i as u64)).unwrap(),
                    bcs::to_bytes(&amount).unwrap(),
                ])
            })
            .collect();
        let outputs = h.run_block_get_output(txns);
        for (i, output) in outputs.iter().enumerate() {
            balance += (block * 100 + i as u64 + 1) as i64;
            let collateral = collateral_written(output, target(i as u64));
            assert_eq!(collateral.primary(), balance);
        }
    }
}

#[test]
fn balance_constructor_writes_a_signed_balance() {
    let (mut h, exchange) = setup(ExecutorMode::BothComparison);
    let txn = h.create_entry_function(&exchange, fun("set_balance"), vec![], vec![
        bcs::to_bytes(&target(0)).unwrap(),
        bcs::to_bytes(&-5i64).unwrap(),
    ]);
    let outputs = h.run_block_get_output(vec![txn]);
    assert_eq!(collateral_written(&outputs[0], target(0)).primary(), -5);
}

#[test]
fn collateral_writes_require_delayed_fields() {
    let (mut h, exchange) = setup(ExecutorMode::SequentialOnly);
    h.enable_features(vec![], vec![FeatureFlag::AGGREGATOR_V2_DELAYED_FIELDS]);
    let status = h.run_entry_function(&exchange, fun("set_balance"), vec![], vec![
        bcs::to_bytes(&target(0)).unwrap(),
        bcs::to_bytes(&1i64).unwrap(),
    ]);
    assert_abort!(status, 3);
}

#[test]
fn collateral_writes_require_native_collateral() {
    let (mut h, exchange) = setup(ExecutorMode::SequentialOnly);
    h.enable_features(vec![], vec![FeatureFlag::NATIVE_COLLATERAL]);
    let status = h.run_entry_function(&exchange, fun("set_balance"), vec![], vec![
        bcs::to_bytes(&target(0)).unwrap(),
        bcs::to_bytes(&1i64).unwrap(),
    ]);
    assert_abort!(status, 1);
}
