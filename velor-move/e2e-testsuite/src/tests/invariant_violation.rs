// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_language_e2e_tests::{common_transactions::peer_to_peer_txn, executor::FakeExecutor};
use velor_types::{
    account_address::AccountAddress,
    on_chain_config::FeatureFlag,
    transaction::{ExecutionStatus, TransactionStatus},
    vm_status::DiscardedVMStatus,
};
use move_core_types::{value::MoveValue, vm_status::StatusCode};

#[test]
fn invariant_violation_error() {
    let _scenario = fail::FailScenario::setup();
    fail::cfg("velor_vm::execute_script_or_entry_function", "100%return").unwrap();

    ::velor_logger::Logger::init_for_testing();

    let mut executor = FakeExecutor::from_head_genesis();

    let sender = executor.create_raw_account_data(1_000_000, 10);
    let receiver = executor.create_raw_account_data(100_000, 10);
    executor.add_account_data(&sender);
    executor.add_account_data(&receiver);

    let transfer_amount = 1_000;
    let txn = peer_to_peer_txn(sender.account(), receiver.account(), 10, transfer_amount, 0);

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
