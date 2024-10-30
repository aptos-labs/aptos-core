// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Note[Orderless]: Done
use aptos_language_e2e_tests::{
    common_transactions::peer_to_peer_txn, executor::FakeExecutor, feature_flags_for_orderless,
};
use aptos_types::{
    account_address::AccountAddress,
    on_chain_config::FeatureFlag,
    transaction::{ExecutionStatus, TransactionStatus},
    vm_status::DiscardedVMStatus,
};
use move_core_types::{value::MoveValue, vm_status::StatusCode};
use rstest::rstest;

// TODO[Orderless]: Remove unneccessary cases later on.
#[rstest(
    sender_stateless_account,
    receiver_stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, true, false, false),
    case(true, true, true, false),
    case(true, true, true, true),
    case(true, false, false, false),
    case(true, false, true, false),
    case(true, false, true, true),
    case(false, true, false, false),
    case(false, true, true, false),
    case(false, true, true, true),
    case(false, false, false, false),
    case(false, false, true, false),
    case(false, false, true, true)
)]
fn invariant_violation_error(
    sender_stateless_account: bool,
    receiver_stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let _scenario = fail::FailScenario::setup();
    fail::cfg("aptos_vm::execute_script_or_entry_function", "100%return").unwrap();

    ::aptos_logger::Logger::init_for_testing();

    let mut executor = FakeExecutor::from_head_genesis();
    executor.enable_features(
        feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
        vec![],
    );

    let sender = executor.create_raw_account_data(
        1_000_000,
        if sender_stateless_account {
            None
        } else {
            Some(0)
        },
    );
    let receiver = executor.create_raw_account_data(
        100_000,
        if receiver_stateless_account {
            None
        } else {
            Some(10)
        },
    );
    executor.add_account_data(&sender);
    executor.add_account_data(&receiver);

    let transfer_amount = 1_000;
    let txn = peer_to_peer_txn(
        sender.account(),
        receiver.account(),
        if use_orderless_transactions {
            None
        } else {
            Some(0)
        },
        transfer_amount,
        0,
        use_txn_payload_v2_format,
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
