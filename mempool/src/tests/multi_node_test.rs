// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    network::MempoolSyncMsg,
    shared_mempool::types::SharedMempoolNotification,
    tests::{
        common,
        common::TestTransaction,
        node::{
            public_full_node_config, validator_config, vfn_config, Node, NodeId, NodeInfo,
            NodeInfoTrait, NodeType,
        },
    },
};
use aptos_config::{
    config::{NodeConfig, PeerRole},
    network_id::{NetworkId, PeerNetworkId},
};
use aptos_netcore::transport::ConnectionOrigin;
use aptos_network::{
    peer_manager::PeerManagerRequest,
    protocols::{
        direct_send::Message,
        network::ReceivedMessage,
        wire::messaging::v1::{DirectSendMsg, NetworkMessage},
    },
    ProtocolId,
};
use aptos_types::{
    transaction::{ReplayProtector, SignedTransaction},
    PeerId,
};
use maplit::btreemap;
use rand::{rngs::StdRng, SeedableRng};
use std::collections::HashMap;
use tokio::runtime::Runtime;

/// A struct holding a list of overriding configurations for mempool
#[derive(Clone, Copy)]
struct MempoolOverrideConfig {
    broadcast_batch_size: Option<usize>,
    max_broadcast_batch_bytes: Option<usize>,
    mempool_size: Option<usize>,
    max_broadcasts_per_peer: Option<usize>,
    ack_timeout_ms: Option<u64>,
    backoff_interval_ms: Option<u64>,
    tick_interval_ms: Option<u64>,
}

impl MempoolOverrideConfig {
    fn new() -> MempoolOverrideConfig {
        MempoolOverrideConfig {
            broadcast_batch_size: Some(1),
            max_broadcast_batch_bytes: None,
            mempool_size: None,
            max_broadcasts_per_peer: None,
            ack_timeout_ms: None,
            backoff_interval_ms: None,
            tick_interval_ms: None,
        }
    }
}

/// A test harness representing a combined network of Nodes and the mempool interactions between them
#[derive(Default)]
struct TestHarness {
    nodes: HashMap<NodeId, Node>,
    /// A mapping of `PeerNetworkId` to `NodeId`.  Used for reverse mapping network requests.
    peer_to_node_id: HashMap<PeerNetworkId, NodeId>,
}

impl TestHarness {
    /// Builds a validator only network for testing the SharedMempool interactions
    fn bootstrap_validator_network(
        validator_nodes_count: u32,
        validator_mempool_config: Option<MempoolOverrideConfig>,
    ) -> (TestHarness, Vec<NodeId>, Runtime) {
        // Create and enter a new runtime
        let runtime = Runtime::new().unwrap();
        let _entered_runtime = runtime.enter();

        // Bootstrap the network
        let (harness, mut peers) = Self::bootstrap_network(
            validator_nodes_count,
            validator_mempool_config,
            false,
            None,
            0,
            None,
        );
        let validators = peers.remove(&PeerRole::Validator).unwrap();

        (harness, validators, runtime)
    }

    /// Builds a fully functional network with Validators, attached VFNs, and full nodes
    /// Note: None of these nodes are told about each other, and must manually be done afterwards
    fn bootstrap_network(
        validator_nodes_count: u32,
        validator_mempool_config: Option<MempoolOverrideConfig>,
        vfns_attached: bool,
        vfn_mempool_config: Option<MempoolOverrideConfig>,
        other_full_nodes_count: u32,
        fn_mempool_config: Option<MempoolOverrideConfig>,
    ) -> (Self, HashMap<PeerRole, Vec<NodeId>>) {
        let mut harness = Self::default();
        let mut rng = StdRng::from_seed([0u8; 32]);
        let mut peers = HashMap::<PeerRole, Vec<NodeId>>::new();

        // Build up validators
        for idx in 0..validator_nodes_count {
            let node_id = harness.add_validator(&mut rng, idx, validator_mempool_config);
            peers.entry(PeerRole::Validator).or_default().push(node_id);
            let validator_peer_id = harness.node(&node_id).peer_id(NetworkId::Validator);

            // Build up VFNs if we've determined we want those too
            if vfns_attached {
                let node_id = harness.add_vfn(&mut rng, idx, vfn_mempool_config, validator_peer_id);
                peers
                    .entry(PeerRole::ValidatorFullNode)
                    .or_default()
                    .push(node_id);
            }
        }

        // Create any additional full nodes
        for idx in validator_nodes_count
            ..other_full_nodes_count
                .checked_add(validator_nodes_count)
                .unwrap()
        {
            let node_id = harness.add_public_full_node(&mut rng, idx, fn_mempool_config);
            peers.entry(PeerRole::Unknown).or_default().push(node_id)
        }

        (harness, peers)
    }

    fn add_validator(
        &mut self,
        rng: &mut StdRng,
        idx: u32,
        mempool_config: Option<MempoolOverrideConfig>,
    ) -> NodeId {
        let (validator, mut v_config) = validator_config(rng);
        Self::update_config(&mut v_config, mempool_config);

        let node_id = NodeId::new(NodeType::Validator, idx);
        let validator_node = NodeInfo::Validator(validator);
        self.add_node(node_id, validator_node, v_config);
        node_id
    }

    fn add_vfn(
        &mut self,
        rng: &mut StdRng,
        idx: u32,
        mempool_config: Option<MempoolOverrideConfig>,
        peer_id: PeerId,
    ) -> NodeId {
        let (vfn, mut vfn_config) = vfn_config(rng, peer_id);
        Self::update_config(&mut vfn_config, mempool_config);

        let node_id = NodeId::new(NodeType::ValidatorFullNode, idx);
        let vfn_node = NodeInfo::ValidatorFull(vfn);
        self.add_node(node_id, vfn_node, vfn_config);
        node_id
    }

    fn add_public_full_node(
        &mut self,
        rng: &mut StdRng,
        idx: u32,
        mempool_config: Option<MempoolOverrideConfig>,
    ) -> NodeId {
        let (full_node, mut fn_config) = public_full_node_config(rng, PeerRole::Unknown);
        Self::update_config(&mut fn_config, mempool_config);

        let node_id = NodeId::new(NodeType::FullNode, idx);
        let full_node = NodeInfo::Full(full_node);
        self.add_node(node_id, full_node, fn_config);
        node_id
    }

    /// Updates configs to adjust for test specific mempool configurations
    /// These adjust the reliability & timeliness of the tests based on what settings are set
    fn update_config(config: &mut NodeConfig, mempool_config: Option<MempoolOverrideConfig>) {
        if let Some(mempool_config) = mempool_config {
            if let Some(batch_size) = mempool_config.broadcast_batch_size {
                config.mempool.shared_mempool_batch_size = batch_size;
            }
            if let Some(max_batch_bytes) = mempool_config.max_broadcast_batch_bytes {
                config.mempool.shared_mempool_max_batch_bytes = max_batch_bytes as u64;
            }
            if let Some(mempool_size) = mempool_config.mempool_size {
                config.mempool.capacity = mempool_size;
            }

            // Set the ack timeout duration to 0 to avoid sleeping to test rebroadcast scenario (broadcast must timeout for this).
            config.mempool.shared_mempool_ack_timeout_ms =
                mempool_config.ack_timeout_ms.unwrap_or(0);

            if let Some(max_broadcasts_per_peer) = mempool_config.max_broadcasts_per_peer {
                config.mempool.max_broadcasts_per_peer = max_broadcasts_per_peer;
            }

            if let Some(backoff_interval_ms) = mempool_config.backoff_interval_ms {
                config.mempool.shared_mempool_backoff_interval_ms = backoff_interval_ms;
            }

            if let Some(tick_interval_ms) = mempool_config.tick_interval_ms {
                config.mempool.shared_mempool_tick_interval_ms = tick_interval_ms;
            }
        }
    }

    fn add_node(&mut self, node_id: NodeId, node_info: NodeInfo, node_config: NodeConfig) {
        for peer_network_id in node_info.peer_network_ids().into_iter() {
            // VFN addresses aren't unique
            if peer_network_id.network_id() != NetworkId::Vfn {
                self.peer_to_node_id.insert(peer_network_id, node_id);
            }
        }

        self.nodes
            .insert(node_id, Node::new(node_info, node_config));
    }

    fn node(&self, node_id: &NodeId) -> &Node {
        self.nodes.get(node_id).unwrap()
    }

    fn mut_node(&mut self, node_id: &NodeId) -> &mut Node {
        self.nodes.get_mut(node_id).unwrap()
    }

    /// Queues transactions for sending on a node.  Must use `broadcast_txns` to send to other nodes
    fn add_txns(&self, node_id: &NodeId, txns: Vec<TestTransaction>) {
        self.node(node_id).add_txns(txns)
    }

    fn find_common_network(&self, node_a: &NodeId, node_b: &NodeId) -> NetworkId {
        let node_a = self.node(node_a);
        let node_b = self.node(node_b);
        node_a.find_common_network(node_b)
    }

    /// Connect two nodes, Dialer -> Receiver, direction is important
    fn connect(&mut self, dialer_id: &NodeId, receiver_id: &NodeId) {
        self.connect_with_network(
            self.find_common_network(dialer_id, receiver_id),
            dialer_id,
            receiver_id,
        )
    }

    fn connect_with_network(
        &mut self,
        network_id: NetworkId,
        dialer_id: &NodeId,
        receiver_id: &NodeId,
    ) {
        // Tell receiver about dialer
        let dialer = self.node(dialer_id);
        let dialer_peer_network_id = dialer.peer_network_id(network_id);
        let dialer_role = dialer.peer_role();
        let receiver = self.mut_node(receiver_id);

        receiver.send_new_peer_event(
            dialer_peer_network_id,
            dialer_role,
            ConnectionOrigin::Inbound,
        );

        // Tell dialer about receiver
        let receiver = self.node(receiver_id);
        let receiver_peer_network_id = receiver.peer_network_id(network_id);
        let receiver_role = receiver.peer_role();
        let dialer = self.mut_node(dialer_id);
        dialer.send_new_peer_event(
            receiver_peer_network_id,
            receiver_role,
            ConnectionOrigin::Outbound,
        );
    }

    /// Blocks, expecting the next event to be the type provided
    fn wait_for_event(&mut self, node_id: &NodeId, expected: SharedMempoolNotification) {
        self.mut_node(node_id).wait_for_event(expected);
    }

    fn check_no_events(&mut self, node_id: &NodeId) {
        self.mut_node(node_id).check_no_subscriber_events();
    }

    /// Checks that a node has no pending messages to send.
    fn assert_no_message_sent(&mut self, node_id: &NodeId, network_id: NetworkId) {
        self.check_no_events(node_id);
        self.mut_node(node_id)
            .check_no_network_messages_sent(network_id);
    }

    fn handle_txns(
        &mut self,
        transactions: Vec<SignedTransaction>,
        sender_id: &NodeId,
        network_id: NetworkId,
        num_transactions_in_message: Option<usize>, // If specified, checks the number of txns in the message
        max_num_transactions_in_message: Option<usize>, // If specified, checks the max number of txns in the message
        check_txns_in_mempool: bool, // Check whether all txns in this broadcast are accepted into recipient's mempool
        execute_send: bool, // If true, actually delivers msg to remote peer; else, drop the message (useful for testing unreliable msg delivery)
        drop_ack: bool,     // If true, drop ack from remote peer to this peer
        remote_peer_id: PeerId,
        msg: Message,
    ) -> (Vec<SignedTransaction>, PeerId) {
        let sender = self.mut_node(sender_id);
        let sender_peer_id = sender.peer_id(network_id);

        // Check that the number of received transactions exactly matches
        if let Some(num_transactions_in_message) = num_transactions_in_message {
            assert_eq!(num_transactions_in_message, transactions.len());
        }

        // Check that the number of transactions in the message is within than the max
        if let Some(max_num_transactions_in_message) = max_num_transactions_in_message {
            assert!(transactions.len() <= max_num_transactions_in_message);
        }

        // If we don't want to forward the request, let's just drop it
        if !execute_send {
            return (transactions, remote_peer_id);
        }

        // Otherwise, let's forward it
        let lookup_peer_network_id = match network_id {
            NetworkId::Vfn => {
                // If this is a validator broadcasting on Vfn we have a problem
                assert!(!sender.supported_networks().contains(&NetworkId::Validator));
                // VFN should have same PeerId but different network from validator
                PeerNetworkId::new(NetworkId::Validator, sender.peer_id(NetworkId::Public))
            },
            _ => PeerNetworkId::new(network_id, remote_peer_id),
        };
        let receiver_id = *self.peer_to_node_id.get(&lookup_peer_network_id).unwrap();
        let receiver = self.mut_node(&receiver_id);
        let rmsg = ReceivedMessage {
            message: NetworkMessage::DirectSendMsg(DirectSendMsg {
                protocol_id: msg.protocol_id,
                priority: 0,
                raw_msg: msg.mdata.into(),
            }),
            sender: PeerNetworkId::new(network_id, sender_peer_id),
            receive_timestamp_micros: 0,
            rpc_replier: None,
        };

        receiver.send_network_req(network_id, ProtocolId::MempoolDirectSend, rmsg);
        receiver.wait_for_event(SharedMempoolNotification::NewTransactions);

        // Verify transaction was inserted into Mempool
        if check_txns_in_mempool {
            let block = self
                .node(sender_id)
                .mempool()
                .get_batch(100, 102400, true, btreemap![]);
            for txn in transactions.iter() {
                assert!(block.contains(txn));
            }
        }

        // Sends an ACK response
        if !drop_ack {
            self.deliver_response(&receiver_id, network_id);
        }
        (transactions, remote_peer_id)
    }

    /// Broadcast Transactions queued up in the local mempool of the sender
    fn broadcast_txns(
        &mut self,
        sender_id: &NodeId,
        network_id: NetworkId,
        num_messages: usize,
        num_transactions_in_message: Option<usize>, // If specified, checks the number of txns in the message
        max_num_transactions_in_message: Option<usize>, // If specified, checks the max number of txns in the message
        check_txns_in_mempool: bool, // Check whether all txns in this broadcast are accepted into recipient's mempool
        execute_send: bool, // If true, actually delivers msg to remote peer; else, drop the message (useful for testing unreliable msg delivery)
        drop_ack: bool,     // If true, drop ack from remote peer to this peer
    ) -> (Vec<SignedTransaction>, PeerId) {
        // Await broadcast notification
        // Note: If there are other messages you're looking for, this could throw them away
        // Wait for the number of messages to be broadcasted on this node
        for _ in 0..num_messages {
            self.wait_for_event(sender_id, SharedMempoolNotification::Broadcast);
        }

        // Get the outgoing network request on the sender
        let sender = self.mut_node(sender_id);
        let network_req = sender.get_next_network_req(network_id);

        // Handle outgoing message
        match network_req {
            PeerManagerRequest::SendDirectSend(remote_peer_id, msg) => {
                let mempool_message = common::decompress_and_deserialize(&msg.mdata.to_vec());
                match mempool_message {
                    MempoolSyncMsg::BroadcastTransactionsRequest {
                        transactions,
                        message_id: _message_id,
                    } => self.handle_txns(
                        transactions,
                        sender_id,
                        network_id,
                        num_transactions_in_message,
                        max_num_transactions_in_message,
                        check_txns_in_mempool,
                        execute_send,
                        drop_ack,
                        remote_peer_id,
                        msg,
                    ),
                    MempoolSyncMsg::BroadcastTransactionsRequestWithReadyTime {
                        message_id: _message_id,
                        transactions,
                    } => self.handle_txns(
                        transactions.into_iter().map(|(txn, _, _)| txn).collect(),
                        sender_id,
                        network_id,
                        num_transactions_in_message,
                        max_num_transactions_in_message,
                        check_txns_in_mempool,
                        execute_send,
                        drop_ack,
                        remote_peer_id,
                        msg,
                    ),
                    req => {
                        panic!("Unexpected broadcast transactions response {:?}", req)
                    },
                }
            },
            req => {
                panic!(
                    "Unexpected peer manager request, didn't receive broadcast {:?}",
                    req
                )
            },
        }
    }

    /// Delivers broadcast ACK from `peer`.
    fn deliver_response(&mut self, sender_id: &NodeId, network_id: NetworkId) {
        // Wait for an ACK to come in on the events
        self.wait_for_event(sender_id, SharedMempoolNotification::ACK);
        let sender = self.mut_node(sender_id);
        let sender_peer_id = sender.peer_id(network_id);
        let network_req = sender.get_next_network_req(network_id);

        match network_req {
            PeerManagerRequest::SendDirectSend(remote_peer_id, msg) => {
                let mempool_message = common::decompress_and_deserialize(&msg.mdata.to_vec());
                match mempool_message {
                    MempoolSyncMsg::BroadcastTransactionsResponse { .. } => {
                        // send it to peer
                        let lookup_peer_network_id = match network_id {
                            NetworkId::Vfn => {
                                // If this is a VFN responding to a validator we have a problem
                                assert!(!sender.supported_networks().contains(&NetworkId::Public));
                                // VFN should have same PeerId but different network from validator
                                PeerNetworkId::new(
                                    NetworkId::Public,
                                    sender.peer_id(NetworkId::Validator),
                                )
                            },
                            _ => PeerNetworkId::new(network_id, remote_peer_id),
                        };
                        let receiver_id =
                            *self.peer_to_node_id.get(&lookup_peer_network_id).unwrap();
                        let receiver = self.mut_node(&receiver_id);
                        let rmsg = ReceivedMessage {
                            message: NetworkMessage::DirectSendMsg(DirectSendMsg {
                                protocol_id: msg.protocol_id,
                                priority: 0,
                                raw_msg: msg.mdata.into(),
                            }),
                            sender: PeerNetworkId::new(network_id, sender_peer_id),
                            receive_timestamp_micros: 0,
                            rpc_replier: None,
                        };

                        receiver.send_network_req(network_id, ProtocolId::MempoolDirectSend, rmsg);
                    },
                    request => panic!(
                        "did not receive expected broadcast ACK, instead got {:?}",
                        request
                    ),
                }
            },
            request => panic!("Node did not ACK broadcast, instead got {:?}", request),
        }
    }
}

fn test_transactions(start: u64, num: u64) -> Vec<TestTransaction> {
    let mut txns = vec![];
    for seq_num in start..start.checked_add(num).unwrap() {
        txns.push(test_transaction(ReplayProtector::SequenceNumber(seq_num)))
    }
    txns
}

fn test_transactions_with_byte_limit(
    starting_sequence_number: u64,
    byte_limit: usize,
) -> Vec<TestTransaction> {
    let mut transactions = vec![];
    let mut sequence_number = starting_sequence_number;
    let mut byte_count = 0;

    // Continue to add transactions to the batch until we hit the byte limit
    loop {
        let transaction = test_transaction(ReplayProtector::SequenceNumber(sequence_number));
        let transaction_bytes = bcs::serialized_size(&transaction).unwrap();
        if (byte_count + transaction_bytes) < byte_limit {
            transactions.push(transaction);
            byte_count += transaction_bytes;
            sequence_number += 1;
        } else {
            return transactions;
        }
    }
}

fn test_transaction(replay_protector: ReplayProtector) -> TestTransaction {
    TestTransaction::new(1, replay_protector, 1)
}

#[test]
fn test_max_broadcast_limit() {
    let mut validator_mempool_config = MempoolOverrideConfig::new();
    validator_mempool_config.max_broadcasts_per_peer = Some(3);
    validator_mempool_config.ack_timeout_ms = Some(u64::MAX);
    validator_mempool_config.backoff_interval_ms = Some(50);

    let (mut harness, validators, _runtime) =
        TestHarness::bootstrap_validator_network(2, Some(validator_mempool_config));
    let (v_a, v_b) = (validators.first().unwrap(), validators.get(1).unwrap());

    let pool_txns = test_transactions(0, 6);
    harness.add_txns(v_a, pool_txns);

    // A and B discover each other
    harness.connect(v_b, v_a);

    // Test that for mempool broadcasts txns up till max broadcast, even if they are not ACK'ed
    let (txns, _) = harness.broadcast_txns(
        v_a,
        NetworkId::Validator,
        1,
        Some(1),
        None,
        true,
        true,
        true,
    );
    assert_eq!(0, txns.first().unwrap().sequence_number());

    for seq_num in 1..3 {
        let (txns, _) = harness.broadcast_txns(
            v_a,
            NetworkId::Validator,
            1,
            Some(1),
            None,
            true,
            false,
            false,
        );
        assert_eq!(seq_num, txns.first().unwrap().sequence_number());
    }

    // Check that mempool doesn't broadcast more than max_broadcasts_per_peer, even
    // if there are more txns in mempool.
    for _ in 0..10 {
        harness.assert_no_message_sent(v_a, NetworkId::Validator);
    }

    // Deliver ACK from B to A.
    // This should unblock A to send more broadcasts.
    harness.deliver_response(v_b, NetworkId::Validator);
    let (txns, _) = harness.broadcast_txns(
        v_a,
        NetworkId::Validator,
        1,
        Some(1),
        None,
        false,
        true,
        true,
    );
    assert_eq!(3, txns.first().unwrap().sequence_number());

    // Check that mempool doesn't broadcast more than max_broadcasts_per_peer, even
    // if there are more txns in mempool.
    for _ in 0..10 {
        harness.assert_no_message_sent(v_a, NetworkId::Validator);
    }
}

#[test]
fn test_max_batch_size() {
    // Test different max batch sizes
    for broadcast_batch_size in [5, 10, 20, 30] {
        let mut validator_mempool_config = MempoolOverrideConfig::new();
        validator_mempool_config.broadcast_batch_size = Some(broadcast_batch_size);
        validator_mempool_config.ack_timeout_ms = Some(u64::MAX);

        let (mut harness, validators, _runtime) =
            TestHarness::bootstrap_validator_network(2, Some(validator_mempool_config));
        let (v_a, v_b) = (validators.first().unwrap(), validators.get(1).unwrap());

        let num_batch_sends = 3;
        let pool_txns = test_transactions(0, (broadcast_batch_size * num_batch_sends * 10) as u64); // Add more than enough txns
        harness.add_txns(v_a, pool_txns);

        // A and B discover each other
        harness.connect(v_b, v_a);

        // Verify the batches are sent
        for _ in 0..num_batch_sends {
            // Verify that mempool broadcasts the transactions from A to B
            // with the expected batch size.
            let _ = harness.broadcast_txns(
                v_a,
                NetworkId::Validator,
                1,
                Some(broadcast_batch_size),
                None,
                true,
                true,
                true,
            );
            harness.deliver_response(v_b, NetworkId::Validator);
        }
    }
}

#[test]
fn test_max_network_byte_size() {
    // Test different max network batch sizes
    for max_broadcast_batch_bytes in [512, 1024, 3 * 1024] {
        let mut validator_mempool_config = MempoolOverrideConfig::new();
        validator_mempool_config.broadcast_batch_size = Some(10000000); // Large max. batch sizes to test chunking
        validator_mempool_config.max_broadcast_batch_bytes = Some(max_broadcast_batch_bytes);
        validator_mempool_config.ack_timeout_ms = Some(u64::MAX);

        // Calculate the expected size of each batch (based on the byte limit)
        let max_broadcast_batch_size =
            test_transactions_with_byte_limit(0, max_broadcast_batch_bytes).len();

        let (mut harness, validators, _runtime) =
            TestHarness::bootstrap_validator_network(2, Some(validator_mempool_config));
        let (v_a, v_b) = (validators.first().unwrap(), validators.get(1).unwrap());

        let num_batch_sends = 3;
        let pool_txns =
            test_transactions(0, (max_broadcast_batch_size * num_batch_sends * 10) as u64); // Add more than enough txns
        harness.add_txns(v_a, pool_txns);

        // A and B discover each other
        harness.connect(v_b, v_a);

        // Verify the batches are sent
        for _ in 0..num_batch_sends {
            // Verify that mempool broadcasts the transactions from A to B
            // with the max broadcast batch size.
            let _ = harness.broadcast_txns(
                v_a,
                NetworkId::Validator,
                1,
                None,
                Some(max_broadcast_batch_size),
                true,
                true,
                true,
            );
            harness.deliver_response(v_b, NetworkId::Validator);
        }
    }
}
