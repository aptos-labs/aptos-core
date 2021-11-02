// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::tests::test_framework::{test_transactions, MempoolTestFramework};
use diem_config::{
    config::PeerRole,
    network_id::{NetworkId, PeerNetworkId},
};
use diem_types::PeerId;
use futures::executor::block_on;
use netcore::transport::ConnectionOrigin;
use network::{
    testutils::{
        builder::TestFrameworkBuilder,
        test_framework::TestFramework,
        test_node::{mock_conn_metadata, ApplicationNode, NodeId, TestNode},
    },
    ProtocolId,
};

#[test]
fn single_node_test() {
    let mut test_framework: MempoolTestFramework =
        TestFrameworkBuilder::new(1).add_validator(0).build();
    let mut node = test_framework.take_node(NodeId::validator(0));
    let network_id = NetworkId::Validator;
    let other_peer_network_id = PeerNetworkId::new(network_id, PeerId::random());
    let other_metadata = mock_conn_metadata(
        other_peer_network_id,
        PeerRole::Validator,
        ConnectionOrigin::Outbound,
        Some(&[ProtocolId::MempoolDirectSend]),
    );
    let future = async move {
        let all_txns = test_transactions(0, 3);
        let all_txns = all_txns.as_slice();
        let inbound_handle = node.get_inbound_handle(network_id);
        node.assert_txns_not_in_mempool(&all_txns[0..1]);
        node.add_txns_via_client(&all_txns[0..1]).await;

        // After we connect, we should try to send messages to it
        inbound_handle.connect(
            node.peer_network_id(network_id).peer_id(),
            network_id,
            other_metadata,
        );

        // Respond and at this point, txn will have shown up
        node.verify_broadcast_and_ack(other_peer_network_id, &all_txns[0..1])
            .await;

        // Now submit another txn and check
        node.add_txns_via_client(&all_txns[1..2]).await;
        node.assert_txns_in_mempool(&all_txns[0..2]);
        node.verify_broadcast_and_ack(other_peer_network_id, &all_txns[1..2])
            .await;

        // Let's also send it an incoming request with more txns and respond with an ack (DirectSend)
        node.send_message(
            ProtocolId::MempoolDirectSend,
            other_peer_network_id,
            &all_txns[2..3],
        )
        .await;
        node.assert_txns_in_mempool(&all_txns[0..3]);
        node.commit_txns(&all_txns[0..3]);
        node.assert_txns_not_in_mempool(&all_txns[0..3]);
    };
    block_on(future);
}

/// Tests if the node is a VFN, and it's getting forwarded messages from a PFN.  It should forward
/// messages to the upstream VAL.  Upstream and downstream nodes are mocked.
#[test]
fn vfn_middle_man_test() {
    let mut test_framework: MempoolTestFramework = TestFrameworkBuilder::new(1).add_vfn(0).build();
    let mut node = test_framework.take_node(NodeId::vfn(0));
    let validator_peer_network_id = PeerNetworkId::new(NetworkId::Vfn, PeerId::random());
    let validator_metadata = mock_conn_metadata(
        validator_peer_network_id,
        PeerRole::Validator,
        ConnectionOrigin::Outbound,
        Some(&[ProtocolId::MempoolDirectSend]),
    );

    let fn_peer_network_id = PeerNetworkId::new(NetworkId::Vfn, PeerId::random());
    let fn_metadata = mock_conn_metadata(
        fn_peer_network_id,
        PeerRole::Unknown,
        ConnectionOrigin::Inbound,
        Some(&[ProtocolId::MempoolDirectSend]),
    );

    let future = async move {
        let test_txns = test_transactions(0, 2);
        let inbound_handle = node.get_inbound_handle(NetworkId::Vfn);
        // Connect upstream Validator and downstream FN
        inbound_handle.connect(
            node.peer_network_id(NetworkId::Vfn).peer_id(),
            NetworkId::Vfn,
            validator_metadata,
        );
        let inbound_handle = node.get_inbound_handle(NetworkId::Public);
        inbound_handle.connect(
            node.peer_network_id(NetworkId::Public).peer_id(),
            NetworkId::Public,
            fn_metadata,
        );

        // Incoming transactions should be accepted
        node.send_message(
            ProtocolId::MempoolDirectSend,
            fn_peer_network_id,
            &test_txns,
        )
        .await;
        node.assert_txns_in_mempool(&test_txns);

        // And they should be forwarded upstream
        node.verify_broadcast_and_ack(validator_peer_network_id, &test_txns)
            .await;
    };
    block_on(future);
}

/// Tests if the node is a VFN, and it's getting forwarded messages from a PFN.  It should forward
/// messages to the upstream VAL.  Upstream and downstream nodes also are running nodes.
#[test]
fn fn_to_val_test() {
    let mut test_framework: MempoolTestFramework = TestFrameworkBuilder::new(1)
        .add_validator(0)
        .add_vfn(0)
        .add_pfn(0)
        .build();

    let mut val = test_framework.take_node(NodeId::validator(0));
    let mut vfn = test_framework.take_node(NodeId::vfn(0));
    let mut pfn = test_framework.take_node(NodeId::pfn(0));
    let pfn_txns = test_transactions(0, 3);
    let vfn_txns = pfn_txns.clone();
    let val_txns = pfn_txns.clone();

    let pfn_vfn_network = pfn.find_common_network(&vfn).unwrap();
    let vfn_metadata = vfn.conn_metadata(
        pfn_vfn_network,
        ConnectionOrigin::Outbound,
        Some(&[ProtocolId::MempoolDirectSend]),
    );
    let vfn_val_network = vfn.find_common_network(&val).unwrap();
    let val_metadata = val.conn_metadata(
        vfn_val_network,
        ConnectionOrigin::Outbound,
        Some(&[ProtocolId::MempoolDirectSend]),
    );

    // NOTE: Always return node at end, or it will be dropped and channels closed
    let pfn_future = async move {
        pfn.connect(pfn_vfn_network, vfn_metadata);
        pfn.add_txns_via_client(&pfn_txns).await;
        pfn.assert_txns_in_mempool(&pfn_txns);
        // Forward to VFN
        pfn.send_next_network_msg(pfn_vfn_network).await;
        pfn
    };

    let vfn_future = async move {
        vfn.connect(vfn_val_network, val_metadata);

        // Respond to PFN
        vfn.send_next_network_msg(pfn_vfn_network).await;
        vfn.assert_txns_in_mempool(&vfn_txns);

        // Forward to VAL
        vfn.send_next_network_msg(vfn_val_network).await;
        vfn
    };

    let val_future = async move {
        // Respond to VFN
        val.send_next_network_msg(vfn_val_network).await;
        val.assert_txns_in_mempool(&val_txns);
        val
    };

    let _ = block_on(futures::future::join3(pfn_future, vfn_future, val_future));
}
