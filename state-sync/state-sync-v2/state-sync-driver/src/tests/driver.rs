// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::Error,
    tests::{
        utils,
        utils::{create_ledger_info_at_version, create_test_transaction},
    },
};
use claim::{assert_err, assert_matches, assert_ok, assert_some};
use consensus_notifications::ConsensusNotificationSender;
use diem_config::config::{NodeConfig, RoleType};
use diem_types::waypoint::Waypoint;
use futures::{executor::block_on, future::join, FutureExt, StreamExt};
use mempool_notifications::MempoolNotifier;

#[tokio::test]
async fn test_bootstrap_notification() {
    // Create a driver for a validator with a waypoint at version 0
    let (mut validator_driver, consensus_notifier, _, _) = utils::create_validator_driver();

    // Send a new commit notification to the driver to complete bootstrapping
    let result = consensus_notifier
        .notify_new_commit(vec![create_test_transaction()], vec![])
        .await;
    assert_ok!(result);

    // Get a client for the driver
    let driver_client = validator_driver.create_driver_client();

    // Verify the client is notified (the node has already bootstrapped)
    assert_ok!(driver_client.notify_once_bootstrapped().await);

    // TODO(joshlind): verify listeners are also eventually notified when the node bootstraps
}

#[tokio::test]
async fn test_consensus_commit_notification() {
    // Create a driver for a full node
    let (mut full_node_driver, consensus_notifier, _, _) = utils::create_full_node_driver();

    // Verify that full nodes can't process commit notifications
    let result = consensus_notifier
        .notify_new_commit(vec![create_test_transaction()], vec![])
        .await;
    assert_err!(result);

    // Create a driver for a validator with a waypoint at version 0
    let (mut validator_driver, consensus_notifier, mut mempool_listener, mut reconfig_listener) =
        utils::create_validator_driver();

    // Send a new commit notification to the driver
    let result = consensus_notifier
        .notify_new_commit(vec![create_test_transaction()], vec![])
        .await;
    assert_ok!(result);

    // Verify that mempool is notified
    let mempool_commit_notification = mempool_listener.select_next_some().now_or_never().unwrap();
    let result = mempool_listener.ack_commit_notification(mempool_commit_notification);
    assert_ok!(result);

    // Verify reconfiguration notifications are published
    assert_some!(reconfig_listener
        .notification_receiver
        .select_next_some()
        .now_or_never());
}

#[tokio::test]
async fn test_consensus_sync_request() {
    // Create a driver for a full node
    let (mut full_node_driver, consensus_notifier, _, _) = utils::create_full_node_driver();

    // Verify that full nodes can't process sync requests
    let result = consensus_notifier
        .sync_to_target(create_ledger_info_at_version(0))
        .await;
    assert_err!(result);

    // Create a driver for a validator with a waypoint at version 0
    let (mut validator_driver, consensus_notifier, _, _) = utils::create_validator_driver();

    // Send a new commit notification to the driver to complete bootstrapping
    let result = consensus_notifier
        .notify_new_commit(vec![create_test_transaction()], vec![])
        .await;
    assert_ok!(result);

    // Verify the driver notifies consensus that it has already reached the target
    let result = consensus_notifier
        .sync_to_target(create_ledger_info_at_version(0))
        .await;
    assert_ok!(result);

    // Create a driver for a validator with a waypoint version higher than 0
    let waypoint_version = 10;
    let waypoint_ledger_info = utils::create_ledger_info_at_version(waypoint_version);
    let waypoint = Waypoint::new_any(waypoint_ledger_info.ledger_info());
    let (mut validator_driver, consensus_notifier, _, _) =
        utils::create_driver_with_config_and_waypoint(NodeConfig::default(), waypoint);

    // Verify the validator sync request should error because the node has not bootstrapped
    let result = consensus_notifier
        .sync_to_target(create_ledger_info_at_version(0))
        .await;
    assert_err!(result);

    // TODO(joshlind): update the database and ensure that consensus is eventually notified
}
