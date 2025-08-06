// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_language_e2e_tests::{common_transactions::peer_to_peer_txn, executor::FakeExecutor};
use aptos_types::{
    account_address::AccountAddress,
    on_chain_config::FeatureFlag,
    transaction::{ExecutionStatus, TransactionStatus},
    vm_status::DiscardedVMStatus,
};
use move_core_types::{value::MoveValue, vm_status::StatusCode};
use rstest::rstest;

#[rstest(
    stateless_account,
    use_orderless_transactions,
    case(true, false),
    case(true, true),
    case(false, false),
    case(false, true)
)]
fn invariant_violation_error(stateless_account: bool, use_orderless_transactions: bool) {
    let _scenario = fail::FailScenario::setup();
    fail::cfg("aptos_vm::execute_script_or_entry_function", "100%return").unwrap();

    ::aptos_logger::Logger::init_for_testing();

    let mut executor = FakeExecutor::from_head_genesis();

    let sender =
        executor.create_raw_account_data(1_000_000, if stateless_account { None } else { Some(0) });
    let receiver = executor.create_raw_account_data(100_000, Some(10));
    executor.add_account_data(&sender);
    executor.add_account_data(&receiver);

    let transfer_amount = 1_000;
    let txn = peer_to_peer_txn(
        sender.account(),
        receiver.account(),
        Some(0),
        transfer_amount,
        0,
        executor.get_block_time_seconds(),
        true,
        use_orderless_transactions,
    );

    // execute transaction
    let output = executor.execute_transaction(txn.clone());

    // CHARGE_INVARIANT_VIOLATION enabled at genesis so this txn is kept.
    assert_eq!(
        output.status(),
        &TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(
            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR
        ))),
    );

    // Disable the CHARGE_INVARIANT_VIOLATION flag.
    executor.exec("features", "change_feature_flags_internal", vec![], vec![
        MoveValue::Signer(AccountAddress::ONE)
            .simple_serialize()
            .unwrap(),
        MoveValue::Vector(vec![]).simple_serialize().unwrap(),
        MoveValue::Vector(vec![MoveValue::U64(
            FeatureFlag::CHARGE_INVARIANT_VIOLATION as u64,
        )])
        .simple_serialize()
        .unwrap(),
    ]);

    let output = executor.execute_transaction(txn);

    // With CHARGE_INVARIANT_VIOLATION disabled this transaction will be discarded.
    assert_eq!(
        output.status(),
        &TransactionStatus::Discard(DiscardedVMStatus::UNKNOWN_INVARIANT_VIOLATION_ERROR),
    );
}
