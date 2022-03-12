// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::tests::{
    utils,
    utils::{create_ledger_info_at_version, create_test_transaction},
};
use claim::assert_err;
use consensus_notifications::ConsensusNotificationSender;

// TODO(joshlind): extend these tests to cover more functionality!

#[tokio::test]
async fn test_consensus_commit_notification() {
    // Create a driver for a full node
    let (_full_node_driver, consensus_notifier, _, _) = utils::create_full_node_driver();

    // Verify that full nodes can't process commit notifications
    let result = consensus_notifier
        .notify_new_commit(vec![create_test_transaction()], vec![])
        .await;
    assert_err!(result);

    // Create a driver for a validator with a waypoint at version 0
    let (_validator_driver, consensus_notifier, _, _) = utils::create_validator_driver();

    // Send a new commit notification and verify the node isn't bootstrapped
    let result = consensus_notifier
        .notify_new_commit(vec![create_test_transaction()], vec![])
        .await;
    assert_err!(result);
}

#[tokio::test]
async fn test_consensus_sync_request() {
    // Create a driver for a full node
    let (_full_node_driver, consensus_notifier, _, _) = utils::create_full_node_driver();

    // Verify that full nodes can't process sync requests
    let result = consensus_notifier
        .sync_to_target(create_ledger_info_at_version(0))
        .await;
    assert_err!(result);

    // Create a driver for a validator with a waypoint at version 0
    let (_validator_driver, consensus_notifier, _, _) = utils::create_validator_driver();

    // Send a new sync request and verify the node isn't bootstrapped
    let result = consensus_notifier
        .sync_to_target(create_ledger_info_at_version(0))
        .await;
    assert_err!(result);
}
