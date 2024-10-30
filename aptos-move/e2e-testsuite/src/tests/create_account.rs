// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

// Note[Orderless]: Done
use aptos_language_e2e_tests::{
    common_transactions::create_account_txn, current_function_name, executor::FakeExecutor,
    feature_flags_for_orderless,
};
use aptos_types::transaction::{ExecutionStatus, TransactionStatus};
use rstest::rstest;

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn create_account(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut executor = FakeExecutor::from_head_genesis();
    executor.enable_features(
        feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
        vec![],
    );
    executor.set_golden_file(current_function_name!());

    // create and publish a sender with 1_000_000 coins
    // let sender = Account::new_aptos_root();
    let sender = executor.create_raw_account();
    let sender = executor.store_and_fund_account(
        sender,
        1_000_000,
        if stateless_account { None } else { Some(0) },
    );
    let new_account = executor.create_raw_account();

    // define the arguments to the create account transaction
    let initial_amount = 0;
    let txn = create_account_txn(
        sender.account(),
        &new_account,
        if use_orderless_transactions {
            None
        } else {
            Some(0)
        },
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );

    // execute transaction
    let output = executor.execute_transaction(txn);
    assert_eq!(
        output.status(),
        &TransactionStatus::Keep(ExecutionStatus::Success)
    );
    executor.apply_write_set(output.write_set());
    // check that numbers in stored DB are correct
    let updated_receiver_balance = executor
        .read_apt_coin_store_resource(&new_account)
        .expect("receiver balance must exist");
    assert_eq!(initial_amount, updated_receiver_balance.coin());
    if stateless_account && use_orderless_transactions {
        assert!(
            executor.read_account_resource(sender.account()).is_none(),
            "sender resource shouldn't be created with an orderless transaction"
        );
    } else {
        let updated_sender = executor
            .read_account_resource(sender.account())
            .expect("sender must exist");
        assert_eq!(
            if use_orderless_transactions { 0 } else { 1 },
            updated_sender.sequence_number()
        );
    }
}
