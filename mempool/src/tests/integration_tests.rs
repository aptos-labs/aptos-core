// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::tests::test_framework::{test_transactions, MempoolNode, MempoolTestFrameworkBuilder};
use diem_config::network_id::PeerNetworkId;
use netcore::transport::ConnectionOrigin;
use network::{
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

const ALL_PROTOCOLS: [ProtocolId; 1] = [ProtocolId::MempoolDirectSend];

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
        let all_txns = test_transactions(0, 2);
        let all_txns = all_txns.as_slice();
        node.connect_self(other_peer_network_id.network_id(), other_metadata);

        // Let's also send it an incoming request with more txns and respond with an ack (DirectSend & Rpc)
        node.send_message(
            ProtocolId::MempoolDirectSend,
            other_peer_network_id,
            &all_txns[0..1],
        )
        .await;
        node.assert_only_txns_in_mempool(&all_txns[0..1]);
    }
}

/// Tests all possible outbound "upstream" peers
#[tokio::test]
async fn single_outbound_node_test() {
    for (mut node, (other_peer_network_id, other_metadata)) in outbound_node_combinations() {
        let all_txns = test_transactions(0, 2);
        let all_txns = all_txns.as_slice();

        // Add transactions
        node.assert_txns_not_in_mempool(&all_txns[0..1]);
        node.add_txns_via_client(&all_txns[0..1]).await;

        // After we connect, all messages should be received and broadcast upstream
        node.connect_self(other_peer_network_id.network_id(), other_metadata);
        node.verify_broadcast_and_ack(other_peer_network_id, &all_txns[0..1])
            .await;
        node.assert_only_txns_in_mempool(&all_txns[0..1]);

        // Adding more txns should also broadcast them upstream
        node.add_txns_via_client(&all_txns[1..2]).await;
        node.verify_broadcast_and_ack(other_peer_network_id, &all_txns[1..2])
            .await;
        node.assert_only_txns_in_mempool(&all_txns[0..2]);
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

    let test_txns = test_transactions(0, 2);
    // Connect upstream Validator and downstream FN
    node.connect_self(validator_peer_network_id.network_id(), validator_metadata);
    node.connect_self(fn_peer_network_id.network_id(), fn_metadata);

    // Incoming transactions should be accepted
    node.send_message(
        ProtocolId::MempoolDirectSend,
        fn_peer_network_id,
        &test_txns,
    )
    .await;
    node.assert_only_txns_in_mempool(&test_txns);

    // And they should be forwarded upstream
    node.verify_broadcast_and_ack(validator_peer_network_id, &test_txns)
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
        let test_txns = test_transactions(0, 3);
        let pfn_txns = test_txns.clone();
        let val_txns = test_txns.clone();

        let pfn_vfn_network = pfn.find_common_network(&vfn).unwrap();
        let vfn_metadata =
            vfn.conn_metadata(pfn_vfn_network, ConnectionOrigin::Outbound, &[protocol_id]);
        let vfn_val_network = vfn.find_common_network(&val).unwrap();
        let val_metadata =
            val.conn_metadata(vfn_val_network, ConnectionOrigin::Outbound, &[protocol_id]);

        // NOTE: Always return node at end, or it will be dropped and channels closed
        let pfn_future = async move {
            pfn.connect(pfn_vfn_network, vfn_metadata);
            pfn.add_txns_via_client(&pfn_txns).await;

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

            val.wait_on_txns_in_mempool(&val_txns).await;
            val
        };

        let (pfn, vfn, val) = futures::future::join3(pfn_future, vfn_future, val_future).await;
        pfn.assert_only_txns_in_mempool(&test_txns);
        vfn.assert_only_txns_in_mempool(&test_txns);
        val.assert_only_txns_in_mempool(&test_txns);
    }
}
