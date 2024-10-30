// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

// Note[Orderless]: Done
use aptos_cached_packages::aptos_stdlib;
use aptos_language_e2e_tests::{
    common_transactions::peer_to_peer_txn, executor::FakeExecutor, feature_flags_for_orderless,
};
use aptos_types::{
    account_config::CORE_CODE_ADDRESS,
    on_chain_config::{AptosVersion, OnChainConfig},
    transaction::TransactionStatus,
};
use aptos_vm::data_cache::AsMoveResolver;
use rstest::rstest;

#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
fn initial_aptos_version(use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let mut executor = FakeExecutor::from_head_genesis();
    executor.enable_features(
        feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
        vec![],
    );
    let resolver = executor.get_state_view().as_move_resolver();
    let version = aptos_types::on_chain_config::APTOS_MAX_KNOWN_VERSION;

    assert_eq!(AptosVersion::fetch_config(&resolver).unwrap(), version);
    let account = executor.new_account_at(CORE_CODE_ADDRESS, Some(0));
    let txn_0 = account
        .transaction()
        .payload(aptos_stdlib::version_set_for_next_epoch(version.major + 1))
        .sequence_number(0)
        .upgrade_payload(use_txn_payload_v2_format, use_orderless_transactions)
        .sign();
    let txn_1 = account
        .transaction()
        .payload(aptos_stdlib::aptos_governance_force_end_epoch())
        .sequence_number(1)
        .upgrade_payload(use_txn_payload_v2_format, use_orderless_transactions)
        .sign();
    executor.new_block();
    executor.execute_and_apply(txn_0);
    executor.new_block();
    executor.execute_and_apply(txn_1);

    let resolver = executor.get_state_view().as_move_resolver();
    assert_eq!(
        AptosVersion::fetch_config(&resolver).unwrap(),
        AptosVersion {
            major: version.major + 1
        }
    );
}

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
fn drop_txn_after_reconfiguration(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut executor = FakeExecutor::from_head_genesis();
    executor.enable_features(
        feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
        vec![],
    );
    let resolver = executor.get_state_view().as_move_resolver();
    let version = aptos_types::on_chain_config::APTOS_MAX_KNOWN_VERSION;
    assert_eq!(AptosVersion::fetch_config(&resolver).unwrap(), version);

    let txn = executor
        .new_account_at(CORE_CODE_ADDRESS, Some(0))
        .transaction()
        .payload(aptos_stdlib::aptos_governance_force_end_epoch())
        .sequence_number(0)
        .upgrade_payload(use_txn_payload_v2_format, use_orderless_transactions)
        .sign();
    executor.new_block();

    let sender = executor
        .create_raw_account_data(1_000_000, if stateless_account { None } else { Some(10) });
    let receiver = executor.create_raw_account_data(100_000, Some(10));
    // TODO[Orderless]: Shouldn't this sequence number be 10 instead of 11?
    let txn2 = peer_to_peer_txn(
        sender.account(),
        receiver.account(),
        if use_orderless_transactions {
            None
        } else {
            Some(11)
        },
        1000,
        0,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );

    let mut output = executor.execute_block(vec![txn, txn2]).unwrap();
    assert_eq!(output.pop().unwrap().status(), &TransactionStatus::Retry)
}
