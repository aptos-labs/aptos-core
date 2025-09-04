// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use velor_cached_packages::velor_stdlib;
use velor_language_e2e_tests::{common_transactions::peer_to_peer_txn, executor::FakeExecutor};
use velor_types::{
    account_config::CORE_CODE_ADDRESS,
    on_chain_config::{VelorVersion, OnChainConfig},
    transaction::TransactionStatus,
};
use velor_vm::data_cache::AsMoveResolver;

#[test]
fn initial_velor_version() {
    let mut executor = FakeExecutor::from_head_genesis();
    let resolver = executor.get_state_view().as_move_resolver();
    let version = velor_types::on_chain_config::VELOR_MAX_KNOWN_VERSION;

    assert_eq!(VelorVersion::fetch_config(&resolver).unwrap(), version);
    let account = executor.new_account_at(CORE_CODE_ADDRESS);
    let txn_0 = account
        .transaction()
        .payload(velor_stdlib::version_set_for_next_epoch(version.major + 1))
        .sequence_number(0)
        .sign();
    let txn_1 = account
        .transaction()
        .payload(velor_stdlib::velor_governance_force_end_epoch())
        .sequence_number(1)
        .sign();
    executor.new_block();
    executor.execute_and_apply(txn_0);
    executor.new_block();
    executor.execute_and_apply(txn_1);

    let resolver = executor.get_state_view().as_move_resolver();
    assert_eq!(
        VelorVersion::fetch_config(&resolver).unwrap(),
        VelorVersion {
            major: version.major + 1
        }
    );
}

#[test]
fn drop_txn_after_reconfiguration() {
    let mut executor = FakeExecutor::from_head_genesis();
    let resolver = executor.get_state_view().as_move_resolver();
    let version = velor_types::on_chain_config::VELOR_MAX_KNOWN_VERSION;
    assert_eq!(VelorVersion::fetch_config(&resolver).unwrap(), version);

    let txn = executor
        .new_account_at(CORE_CODE_ADDRESS)
        .transaction()
        .payload(velor_stdlib::velor_governance_force_end_epoch())
        .sequence_number(0)
        .sign();
    executor.new_block();

    let sender = executor.create_raw_account_data(1_000_000, 10);
    let receiver = executor.create_raw_account_data(100_000, 10);
    let txn2 = peer_to_peer_txn(sender.account(), receiver.account(), 11, 1000, 0);

    let mut output = executor.execute_block(vec![txn, txn2]).unwrap();
    assert_eq!(output.pop().unwrap().status(), &TransactionStatus::Retry)
}
