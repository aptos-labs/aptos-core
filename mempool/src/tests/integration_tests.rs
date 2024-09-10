// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::tests::{
    common::TestTransaction,
    test_framework::{test_transaction, MempoolNode, MempoolTestFrameworkBuilder},
};
use aptos_config::network_id::PeerNetworkId;
use aptos_netcore::transport::ConnectionOrigin;
use aptos_network::{
    testutils::{
        test_framework::TestFramework,
        test_node::{
            pfn_pfn_mock_connection, pfn_vfn_mock_connection, validator_mock_connection,
            vfn_validator_mock_connection, vfn_vfn_mock_connection, NodeId, TestNode,
        },
    },
    transport::ConnectionMetadata,
    ProtocolId,
};
use once_cell::sync::Lazy;
use std::time::Duration;

const ALL_PROTOCOLS: [ProtocolId; 1] = [ProtocolId::MempoolDirectSend];
static ALL_TXNS: Lazy<Vec<TestTransaction>> =
    Lazy::new(|| vec![test_transaction(0), test_transaction(1)]);
static TXN_1: Lazy<Vec<TestTransaction>> = Lazy::new(|| vec![test_transaction(0)]);
static TXN_2: Lazy<Vec<TestTransaction>> = Lazy::new(|| vec![test_transaction(1)]);

fn inbound_node_combinations() -> [(MempoolNode, (PeerNetworkId, ConnectionMetadata)); 6] {
    [
        (
            MempoolTestFrameworkBuilder::single_validator(),
            validator_mock_connection(ConnectionOrigin::Inbound, &ALL_PROTOCOLS),
        ),
        (
            MempoolTestFrameworkBuilder::single_validator(),
            validator_mock_connection(ConnectionOrigin::Outbound, &ALL_PROTOCOLS),
        ),
        (
            MempoolTestFrameworkBuilder::single_validator(),
            vfn_validator_mock_connection(ConnectionOrigin::Inbound, &ALL_PROTOCOLS),
        ),
        (
            MempoolTestFrameworkBuilder::single_vfn(),
            vfn_vfn_mock_connection(ConnectionOrigin::Inbound, &ALL_PROTOCOLS),
        ),
        (
            MempoolTestFrameworkBuilder::single_vfn(),
            pfn_vfn_mock_connection(ConnectionOrigin::Inbound, &ALL_PROTOCOLS),
        ),
        (
            MempoolTestFrameworkBuilder::single_pfn(),
            pfn_pfn_mock_connection(ConnectionOrigin::Inbound, &ALL_PROTOCOLS),
        ),
    ]
}

fn outbound_node_combinations() -> [(MempoolNode, (PeerNetworkId, ConnectionMetadata)); 6] {
    [
        (
            MempoolTestFrameworkBuilder::single_validator(),
            validator_mock_connection(ConnectionOrigin::Inbound, &ALL_PROTOCOLS),
        ),
        (
            MempoolTestFrameworkBuilder::single_validator(),
            validator_mock_connection(ConnectionOrigin::Outbound, &ALL_PROTOCOLS),
        ),
        (
            MempoolTestFrameworkBuilder::single_vfn(),
            vfn_validator_mock_connection(ConnectionOrigin::Outbound, &ALL_PROTOCOLS),
        ),
        (
            MempoolTestFrameworkBuilder::single_vfn(),
            vfn_vfn_mock_connection(ConnectionOrigin::Outbound, &ALL_PROTOCOLS),
        ),
        (
            MempoolTestFrameworkBuilder::single_pfn(),
            pfn_vfn_mock_connection(ConnectionOrigin::Outbound, &ALL_PROTOCOLS),
        ),
        (
            MempoolTestFrameworkBuilder::single_pfn(),
            pfn_pfn_mock_connection(ConnectionOrigin::Outbound, &ALL_PROTOCOLS),
        ),
    ]
}

/// Tests all possible inbound "downstream" peers
#[tokio::test]
async fn single_inbound_node_test() {
    for (mut node, (other_peer_network_id, other_metadata)) in inbound_node_combinations() {
        node.connect_self(other_peer_network_id.network_id(), other_metadata);

        // Let's also send it an incoming request with more txns and respond with an ack (DirectSend & Rpc)
        node.receive_message(
            ProtocolId::MempoolDirectSend,
            other_peer_network_id,
            &ALL_TXNS,
        )
        .await;
        node.assert_only_txns_in_mempool(&ALL_TXNS);
    }
}

/// Tests all possible outbound "upstream" peers
#[tokio::test]
async fn single_outbound_node_test() {
    for (mut node, (other_peer_network_id, other_metadata)) in outbound_node_combinations() {
        // Add transactions
        node.assert_txns_not_in_mempool(&TXN_1);
        node.add_txns_via_client(&TXN_1).await;

        // After we connect, all messages should be received and broadcast upstream
        node.connect_self(other_peer_network_id.network_id(), other_metadata);
        node.send_broadcast_and_receive_ack(other_peer_network_id, &TXN_1)
            .await;
        node.assert_only_txns_in_mempool(&TXN_1);

        // Adding more txns should also broadcast them upstream
        node.add_txns_via_client(&TXN_2).await;
        node.send_broadcast_and_receive_ack(other_peer_network_id, &TXN_2)
            .await;
        node.assert_only_txns_in_mempool(&ALL_TXNS);
    }
}

/// Tests if the node is a VFN, and it's getting forwarded messages from a PFN.  It should forward
/// messages to the upstream VAL.  Upstream and downstream nodes are mocked.
#[tokio::test]
async fn vfn_middle_man_test() {
    let mut node = MempoolTestFrameworkBuilder::single_vfn();
    let (validator_peer_network_id, validator_metadata) =
        vfn_validator_mock_connection(ConnectionOrigin::Outbound, &ALL_PROTOCOLS);

    let (fn_peer_network_id, fn_metadata) =
        pfn_vfn_mock_connection(ConnectionOrigin::Inbound, &ALL_PROTOCOLS);

    // Connect upstream Validator and downstream FN
    node.connect_self(validator_peer_network_id.network_id(), validator_metadata);
    node.connect_self(fn_peer_network_id.network_id(), fn_metadata);

    // Incoming transactions should be accepted
    node.receive_message(ProtocolId::MempoolDirectSend, fn_peer_network_id, &ALL_TXNS)
        .await;
    node.assert_only_txns_in_mempool(&ALL_TXNS);

    // And they should be forwarded upstream
    node.send_broadcast_and_receive_ack(validator_peer_network_id, &ALL_TXNS)
        .await;
}

/// Tests when a node skips an ack
#[tokio::test]
async fn test_skip_ack_rebroadcast() {
    let mut node = MempoolTestFrameworkBuilder::single_validator();
    let (other_peer_network_id, other_metadata) =
        validator_mock_connection(ConnectionOrigin::Inbound, &ALL_PROTOCOLS);

    // Add transactions
    node.assert_txns_not_in_mempool(&ALL_TXNS);
    node.add_txns_via_client(&ALL_TXNS).await;

    node.connect_self(other_peer_network_id.network_id(), other_metadata.clone());

    // After we drop, we should rebroadcast successfully
    node.drop_next_network_msg(other_peer_network_id.network_id())
        .await;
    node.send_broadcast_and_receive_ack(other_peer_network_id, &ALL_TXNS)
        .await;
}

/// Tests when a node gets disconnected. Node should pick up after the second sending
/// TODO: also add an outbound test to ensure it'll broadcast all transactions again
#[tokio::test]
async fn test_interrupt_in_sync_inbound() {
    for (mut node, (other_peer_network_id, other_metadata)) in inbound_node_combinations() {
        // First txn is received
        node.connect_self(other_peer_network_id.network_id(), other_metadata.clone());
        node.receive_message(ProtocolId::MempoolDirectSend, other_peer_network_id, &TXN_1)
            .await;
        node.assert_only_txns_in_mempool(&TXN_1);

        // Drop the connection, and a reconnect should merge txns
        node.disconnect_self(other_peer_network_id.network_id(), other_metadata.clone());

        // Now receiving all should be okay
        node.receive_message(
            ProtocolId::MempoolDirectSend,
            other_peer_network_id,
            &ALL_TXNS,
        )
        .await;
        node.assert_only_txns_in_mempool(&ALL_TXNS);
    }
}

/// Tests that transactions will only be sent when they're ready (previous seq no have been sent)
#[tokio::test]
async fn test_ready_txns() {
    let mut node = MempoolTestFrameworkBuilder::single_validator();
    let (other_peer_network_id, other_metadata) =
        validator_mock_connection(ConnectionOrigin::Inbound, &ALL_PROTOCOLS);

    // Add 2nd txn
    node.assert_txns_not_in_mempool(&TXN_2);
    node.add_txns_via_client(&TXN_2).await;

    // No txns should be sent or ready
    node.connect_self(other_peer_network_id.network_id(), other_metadata.clone());
    node.assert_txns_not_in_mempool(&ALL_TXNS);
    node.wait_for_no_msg(
        other_peer_network_id.network_id(),
        Duration::from_millis(100),
    )
    .await;

    // Adding earlier txns should fill in the gaps, and now it should send all
    node.add_txns_via_client(&TXN_1).await;
    node.assert_txns_in_mempool(&ALL_TXNS);
    node.send_broadcast_and_receive_ack(other_peer_network_id, &ALL_TXNS)
        .await;
    node.assert_only_txns_in_mempool(&ALL_TXNS);
}

/// Test that in the validator network, messages won't be sent back to the original sender
#[tokio::test]
async fn test_broadcast_self_txns() {
    let mut node = MempoolTestFrameworkBuilder::single_validator();
    let (other_peer_network_id, other_metadata) =
        validator_mock_connection(ConnectionOrigin::Inbound, &ALL_PROTOCOLS);

    // Other node sends earlier txn
    node.connect_self(other_peer_network_id.network_id(), other_metadata.clone());
    node.receive_message(ProtocolId::MempoolDirectSend, other_peer_network_id, &TXN_1)
        .await;
    node.assert_txns_in_mempool(&TXN_1);

    // Add txns to current node
    node.add_txns_via_client(&TXN_2).await;
    node.assert_txns_in_mempool(&ALL_TXNS);

    // Txns should be sent to other node (but not the earlier txn)
    node.send_broadcast_and_receive_ack(other_peer_network_id, &TXN_2)
        .await;
}

/// Test that gas price updates work and push onward to other nodes
#[tokio::test]
async fn test_update_gas_price() {
    let new_txn = TestTransaction::new(1, 0, 100);
    let new_txn = &[new_txn];

    let mut node = MempoolTestFrameworkBuilder::single_validator();
    let (other_peer_network_id, other_metadata) =
        validator_mock_connection(ConnectionOrigin::Outbound, &ALL_PROTOCOLS);

    // Get first txn
    node.add_txns_via_client(&TXN_1).await;
    node.assert_txns_in_mempool(&TXN_1);

    // Send to other node
    node.connect_self(other_peer_network_id.network_id(), other_metadata.clone());
    node.send_broadcast_and_receive_ack(other_peer_network_id, &TXN_1)
        .await;

    // Update txn
    node.add_txns_via_client(new_txn).await;
    node.assert_only_txns_in_mempool(new_txn);

    // Updated txn should be sent
    node.send_broadcast_and_receive_ack(other_peer_network_id, new_txn)
        .await;
}

/// In the event of a full mempool, retry and broadcast again
#[tokio::test]
async fn test_mempool_full_rebroadcast() {
    let mut node = MempoolTestFrameworkBuilder::single_validator();
    let (other_peer_network_id, other_metadata) =
        validator_mock_connection(ConnectionOrigin::Outbound, &ALL_PROTOCOLS);

    // Get first txn
    node.add_txns_via_client(&ALL_TXNS).await;
    node.assert_txns_in_mempool(&ALL_TXNS);

    // Send to other node (which is full)
    node.connect_self(other_peer_network_id.network_id(), other_metadata.clone());
    node.send_broadcast_and_receive_retry(other_peer_network_id, &ALL_TXNS)
        .await;

    // Txn should be sent again later
    node.send_broadcast_and_receive_ack(other_peer_network_id, &ALL_TXNS)
        .await;
}

/// The retry broadcast can become empty due to commits. The next broadcast should ignore this empty broadcast.
#[tokio::test]
async fn test_rebroadcast_retry_is_empty() {
    let mut node = MempoolTestFrameworkBuilder::single_validator();
    let (other_peer_network_id, other_metadata) =
        validator_mock_connection(ConnectionOrigin::Outbound, &ALL_PROTOCOLS);

    // Get first txn
    node.add_txns_via_client(&TXN_1).await;
    node.assert_txns_in_mempool(&TXN_1);

    // Send to other node (which is full)
    node.connect_self(other_peer_network_id.network_id(), other_metadata.clone());
    node.send_broadcast_and_receive_retry(other_peer_network_id, &TXN_1)
        .await;

    // Add txn2. In the meantime, txn1 was committed.
    node.add_txns_via_client(&TXN_2).await;
    node.commit_txns(&TXN_1).await;

    // Txn should be sent again later
    node.send_broadcast_and_receive_ack(other_peer_network_id, &TXN_2)
        .await;
}

// -- Multi node tests below here --

/// Tests if the node is a VFN, and it's getting forwarded messages from a PFN.  It should forward
/// messages to the upstream VAL.  Upstream and downstream nodes also are running nodes.
#[tokio::test]
async fn fn_to_val_test() {
    for protocol_id in ALL_PROTOCOLS {
        let mut test_framework = MempoolTestFrameworkBuilder::new(1)
            .add_validator(0)
            .add_vfn(0)
            .add_pfn(0)
            .build();

        let mut val = test_framework.take_node(NodeId::validator(0));
        let mut vfn = test_framework.take_node(NodeId::vfn(0));
        let mut pfn = test_framework.take_node(NodeId::pfn(0));

        let pfn_vfn_network = pfn.find_common_network(&vfn).unwrap();
        let vfn_metadata =
            vfn.conn_metadata(pfn_vfn_network, ConnectionOrigin::Outbound, &[protocol_id]);
        let vfn_val_network = vfn.find_common_network(&val).unwrap();
        let val_metadata =
            val.conn_metadata(vfn_val_network, ConnectionOrigin::Outbound, &[protocol_id]);

        // NOTE: Always return node at end, or it will be dropped and channels closed
        let pfn_future = async move {
            pfn.add_txns_via_client(&ALL_TXNS).await;
            pfn.connect(pfn_vfn_network, vfn_metadata);

            // Forward to VFN
            pfn.send_next_network_msg(pfn_vfn_network).await;
            pfn
        };

        let vfn_future = async move {
            vfn.connect(vfn_val_network, val_metadata);

            // Respond to PFN (RPC doesn't need to do this)
            if protocol_id == ProtocolId::MempoolDirectSend {
                vfn.send_next_network_msg(pfn_vfn_network).await;
            }

            // Forward to VAL
            vfn.send_next_network_msg(vfn_val_network).await;
            vfn
        };

        let val_future = async move {
            // Respond to VFN (RPC doesn't need to do this)
            if protocol_id == ProtocolId::MempoolDirectSend {
                val.send_next_network_msg(vfn_val_network).await;
            }

            val.wait_on_txns_in_mempool(&ALL_TXNS).await;
            val
        };

        let (pfn, vfn, val) = futures::future::join3(pfn_future, vfn_future, val_future).await;
        pfn.assert_only_txns_in_mempool(&ALL_TXNS);
        vfn.assert_only_txns_in_mempool(&ALL_TXNS);
        val.assert_only_txns_in_mempool(&ALL_TXNS);
    }
}
