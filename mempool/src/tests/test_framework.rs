// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    core_mempool::CoreMempool,
    shared_mempool::{
        start_shared_mempool,
        types::{MempoolMessageId, MempoolSenderBucket},
    },
    tests::common::{self, TestTransaction},
    MempoolClientRequest, MempoolClientSender, MempoolSyncMsg, QuorumStoreRequest,
};
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_config::{
    config::NodeConfig,
    network_id::{NetworkId, PeerNetworkId},
};
use aptos_event_notifications::{ReconfigNotification, ReconfigNotificationListener};
use aptos_id_generator::U32IdGenerator;
use aptos_infallible::{Mutex, RwLock};
use aptos_mempool_notifications::MempoolNotifier;
use aptos_network::{
    application::{
        interface::{NetworkClient, NetworkServiceEvents},
        storage::PeersAndMetadata,
    },
    peer_manager::{ConnectionRequestSender, PeerManagerRequest, PeerManagerRequestSender},
    protocols::{
        network::{
            NetworkEvents, NetworkSender, NewNetworkEvents, NewNetworkSender, ReceivedMessage,
        },
        wire::{
            handshake::v1::ProtocolId::MempoolDirectSend,
            messaging::v1::{DirectSendMsg, NetworkMessage, RpcRequest},
        },
    },
    testutils::{
        builder::TestFrameworkBuilder,
        test_framework::TestFramework,
        test_node::{
            ApplicationNode, InboundNetworkHandle, NodeId, OutboundMessageReceiver, TestNode,
        },
    },
    ProtocolId,
};
use aptos_storage_interface::mock::MockDbReaderWriter;
use aptos_types::{
    account_address::AccountAddress,
    mempool_status::MempoolStatusCode,
    on_chain_config::{InMemoryOnChainConfig, OnChainConfigPayload},
    transaction::{ReplayProtector, SignedTransaction},
};
use aptos_vm_validator::mocks::mock_vm_validator::MockVMValidator;
use futures::{channel::oneshot, SinkExt};
use maplit::btreemap;
use std::{collections::HashMap, hash::Hash, sync::Arc};
use tokio::{runtime::Handle, time::Duration};
use tokio_stream::StreamExt;

/// An individual mempool node that runs in it's own runtime.
///
/// TODO: Add ability to mock StateSync updates to remove transactions
/// TODO: Add ability to reject transactions via Consensus
#[allow(dead_code)]
pub struct MempoolNode {
    /// The [`CoreMempool`] storage of the node
    pub mempool: Arc<Mutex<CoreMempool>>,
    /// A generator for [`MempoolSyncMsg`] request ids.
    pub request_id_generator: U32IdGenerator,

    // Mempool specific channels
    /// Used for incoming JSON-RPC requests (e.g. adding new transactions)
    pub mempool_client_sender: MempoolClientSender,
    /// Used for quorum store requests
    pub consensus_to_mempool_sender: futures::channel::mpsc::Sender<QuorumStoreRequest>,
    /// Used for StateSync commit notifications
    pub mempool_notifications: MempoolNotifier,

    // Networking specifics
    node_id: NodeId,
    peer_network_ids: HashMap<NetworkId, PeerNetworkId>,
    peers_and_metadata: Arc<PeersAndMetadata>,

    inbound_handles: HashMap<NetworkId, InboundNetworkHandle>,
    outbound_handles: HashMap<NetworkId, OutboundMessageReceiver>,
    other_inbound_handles: HashMap<PeerNetworkId, InboundNetworkHandle>,
}

impl std::fmt::Display for MempoolNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.node_id)
    }
}

impl ApplicationNode for MempoolNode {
    fn node_id(&self) -> NodeId {
        self.node_id
    }

    fn default_protocols(&self) -> &[ProtocolId] {
        &[ProtocolId::MempoolDirectSend]
    }

    fn get_inbound_handle(&self, network_id: NetworkId) -> InboundNetworkHandle {
        self.inbound_handles
            .get(&network_id)
            .unwrap_or_else(|| panic!("Must have inbound handle for network {}", network_id))
            .clone()
    }

    fn add_inbound_handle_for_peer(
        &mut self,
        peer_network_id: PeerNetworkId,
        handle: InboundNetworkHandle,
    ) {
        if self
            .other_inbound_handles
            .insert(peer_network_id, handle)
            .is_some()
        {
            panic!(
                "Double added handle for {} on {}",
                peer_network_id, self.node_id
            )
        }
    }

    fn get_inbound_handle_for_peer(&self, peer_network_id: PeerNetworkId) -> InboundNetworkHandle {
        self.other_inbound_handles
            .get(&peer_network_id)
            .expect("Must have inbound handle for other peer")
            .clone()
    }

    fn get_outbound_handle(&mut self, network_id: NetworkId) -> &mut OutboundMessageReceiver {
        self.outbound_handles.get_mut(&network_id).unwrap()
    }

    fn get_peers_and_metadata(&self) -> &PeersAndMetadata {
        &self.peers_and_metadata
    }

    fn peer_network_ids(&self) -> &HashMap<NetworkId, PeerNetworkId> {
        &self.peer_network_ids
    }
}

impl MempoolNode {
    /// Queues transactions for sending on a node, uses client
    pub async fn add_txns_via_client(&mut self, txns: &[TestTransaction]) {
        for txn in sign_transactions(txns) {
            let (sender, receiver) = oneshot::channel();

            self.mempool_client_sender
                .send(MempoolClientRequest::SubmitTransaction(txn, sender))
                .await
                .unwrap();
            let status = receiver.await.unwrap().unwrap();
            assert_eq!(status.0.code, MempoolStatusCode::Accepted)
        }
    }

    pub async fn commit_txns(&mut self, txns: &[TestTransaction]) {
        for txn in sign_transactions(txns) {
            self.mempool
                .lock()
                .commit_transaction(&txn.sender(), txn.replay_protector());
        }
    }

    pub async fn get_parking_lot_txns_via_client(&mut self) -> Vec<(AccountAddress, u64)> {
        let (sender, receiver) = oneshot::channel();
        self.mempool_client_sender
            .send(MempoolClientRequest::GetAddressesFromParkingLot(sender))
            .await
            .unwrap();
        receiver.await.unwrap()
    }

    /// Asynchronously waits for up to 1 second for txns to appear in mempool
    pub async fn wait_on_txns_in_mempool(&self, txns: &[TestTransaction]) {
        for _ in 0..10 {
            let block = self
                .mempool
                .lock()
                .get_batch(100, 102400, true, btreemap![]);

            if block_contains_all_transactions(&block, txns) {
                break;
            }

            tokio::time::sleep(Duration::from_millis(100)).await
        }
    }

    pub fn assert_only_txns_in_mempool(&self, txns: &[TestTransaction]) {
        if let Err((actual, expected)) =
            self.assert_condition_on_mempool_txns(txns, block_only_contains_transactions)
        {
            panic!(
                "Expected to contain test transactions {:?}, but got {:?}",
                expected, actual
            );
        }
    }

    pub fn assert_txns_in_mempool(&self, txns: &[TestTransaction]) {
        if let Err((actual, expected)) =
            self.assert_condition_on_mempool_txns(txns, block_contains_all_transactions)
        {
            panic!(
                "Expected to contain test transactions {:?}, but got {:?}",
                expected, actual
            );
        }
    }

    pub fn assert_txns_not_in_mempool(&self, txns: &[TestTransaction]) {
        if let Err((actual, expected)) = self.assert_condition_on_mempool_txns(txns, {
            |actual, expected| !block_contains_any_transaction(actual, expected)
        }) {
            panic!(
                "Expected to not contain test transactions {:?}, but got {:?}",
                expected, actual
            );
        }
    }

    fn assert_condition_on_mempool_txns<
        Condition: FnOnce(&[SignedTransaction], &[TestTransaction]) -> bool,
    >(
        &self,
        txns: &[TestTransaction],
        condition: Condition,
    ) -> Result<
        (),
        (
            Vec<(AccountAddress, ReplayProtector)>,
            Vec<(AccountAddress, ReplayProtector)>,
        ),
    > {
        let block = self
            .mempool
            .lock()
            .get_batch(100, 102400, true, btreemap![]);
        if !condition(&block, txns) {
            let actual: Vec<_> = block
                .iter()
                .map(|txn| (txn.sender(), txn.replay_protector()))
                .collect();
            let expected: Vec<_> = txns
                .iter()
                .map(|txn| (txn.address, txn.replay_protector))
                .collect();
            Err((actual, expected))
        } else {
            Ok(())
        }
    }

    pub async fn receive_message(
        &mut self,
        protocol_id: ProtocolId,
        remote_peer_network_id: PeerNetworkId,
        txns: &[TestTransaction],
    ) {
        let network_id = remote_peer_network_id.network_id();
        let remote_peer_id = remote_peer_network_id.peer_id();
        let inbound_handle = self.get_inbound_handle(network_id);
        let message_id_in_request = MempoolMessageId::from_timeline_ids(vec![(
            0 as MempoolSenderBucket,
            (vec![1].into(), vec![10].into()),
        )]);
        let msg = MempoolSyncMsg::BroadcastTransactionsRequest {
            message_id: message_id_in_request.clone(),
            transactions: sign_transactions(txns),
        };
        let data = protocol_id.to_bytes(&msg).unwrap();
        let (notif, maybe_receiver) = match protocol_id {
            ProtocolId::MempoolDirectSend => (
                ReceivedMessage {
                    message: NetworkMessage::DirectSendMsg(DirectSendMsg {
                        protocol_id,
                        priority: 0,
                        raw_msg: data,
                    }),
                    sender: PeerNetworkId::new(network_id, remote_peer_id),
                    receive_timestamp_micros: 0,
                    rpc_replier: None,
                },
                None,
            ),
            ProtocolId::MempoolRpc => {
                let (res_tx, res_rx) = oneshot::channel();
                let rmsg = ReceivedMessage {
                    message: NetworkMessage::RpcRequest(RpcRequest {
                        protocol_id,
                        request_id: 0,
                        priority: 0,
                        raw_request: data,
                    }),
                    sender: PeerNetworkId::new(network_id, remote_peer_id),
                    receive_timestamp_micros: 0,
                    rpc_replier: Some(Arc::new(res_tx)),
                };
                (rmsg, Some(res_rx))
            },

            protocol_id => panic!("Invalid protocol id found: {:?}", protocol_id),
        };
        inbound_handle
            .inbound_message_sender
            .push((remote_peer_id, protocol_id), notif)
            .unwrap();

        let response: MempoolSyncMsg = if let Some(res_rx) = maybe_receiver {
            let response = res_rx.await.unwrap().unwrap();
            protocol_id.from_bytes(&response).unwrap()
        } else {
            match self.get_outbound_handle(network_id).next().await.unwrap() {
                PeerManagerRequest::SendDirectSend(peer_id, msg) => {
                    assert_eq!(peer_id, remote_peer_id);
                    msg.protocol_id.from_bytes(&msg.mdata).unwrap()
                },
                _ => panic!("Should not be getting an RPC response"),
            }
        };
        if let MempoolSyncMsg::BroadcastTransactionsResponse {
            message_id: message_id_in_response,
            retry,
            backoff,
        } = response
        {
            assert_eq!(message_id_in_response, message_id_in_request);
            assert!(!retry);
            assert!(!backoff);
        } else {
            panic!("Expected a response!");
        }
    }

    pub async fn send_broadcast_and_receive_ack(
        &mut self,
        expected_peer_network_id: PeerNetworkId,
        expected_txns: &[TestTransaction],
    ) {
        self.send_broadcast_and_receive_response(
            expected_peer_network_id,
            expected_txns,
            false,
            false,
        )
        .await
    }

    pub async fn send_broadcast_and_receive_retry(
        &mut self,
        expected_peer_network_id: PeerNetworkId,
        expected_txns: &[TestTransaction],
    ) {
        // Don't backoff so the test is faster
        self.send_broadcast_and_receive_response(
            expected_peer_network_id,
            expected_txns,
            true,
            false,
        )
        .await
    }

    /// Send a broadcast and receive a response
    async fn send_broadcast_and_receive_response(
        &mut self,
        expected_peer_network_id: PeerNetworkId,
        expected_txns: &[TestTransaction],
        retry: bool,
        backoff: bool,
    ) {
        let network_id = expected_peer_network_id.network_id();
        let expected_peer_id = expected_peer_network_id.peer_id();
        let inbound_handle = self.get_inbound_handle(network_id);
        let message = self.get_next_network_msg(network_id).await;
        let (peer_id, protocol_id, data, maybe_rpc_sender) = match message {
            PeerManagerRequest::SendRpc(peer_id, msg) => {
                (peer_id, msg.protocol_id, msg.data, Some(msg.res_tx))
            },
            PeerManagerRequest::SendDirectSend(peer_id, msg) => {
                (peer_id, msg.protocol_id, msg.mdata, None)
            },
        };
        assert_eq!(peer_id, expected_peer_id);
        let mempool_message = common::decompress_and_deserialize(&data.to_vec());
        let message_id = match mempool_message {
            MempoolSyncMsg::BroadcastTransactionsRequest {
                message_id,
                transactions,
            } => {
                if !block_only_contains_transactions(&transactions, expected_txns) {
                    let txns: Vec<_> = transactions
                        .iter()
                        .map(|txn| (txn.sender(), txn.replay_protector()))
                        .collect();
                    let expected_txns: Vec<_> = expected_txns
                        .iter()
                        .map(|txn| (txn.address, txn.replay_protector))
                        .collect();

                    panic!(
                        "Request doesn't match. Actual: {:?} Expected: {:?}",
                        txns, expected_txns
                    );
                }
                message_id
            },
            MempoolSyncMsg::BroadcastTransactionsRequestWithReadyTime {
                message_id,
                transactions,
            } => {
                let transactions: Vec<_> =
                    transactions.iter().map(|(txn, _, _)| txn.clone()).collect();
                if !block_only_contains_transactions(&transactions, expected_txns) {
                    let txns: Vec<_> = transactions
                        .iter()
                        .map(|txn| (txn.sender(), txn.replay_protector()))
                        .collect();
                    let expected_txns: Vec<_> = expected_txns
                        .iter()
                        .map(|txn| (txn.address, txn.replay_protector))
                        .collect();

                    panic!(
                        "Request doesn't match. Actual: {:?} Expected: {:?}",
                        txns, expected_txns
                    );
                }
                message_id
            },
            MempoolSyncMsg::BroadcastTransactionsResponse { .. } => {
                panic!("We aren't supposed to be getting as response here");
            },
        };
        let response = MempoolSyncMsg::BroadcastTransactionsResponse {
            message_id,
            retry,
            backoff,
        };
        let bytes = protocol_id.to_bytes(&response).unwrap();

        if let Some(rpc_sender) = maybe_rpc_sender {
            rpc_sender.send(Ok(bytes.into())).unwrap();
        } else {
            let notif = ReceivedMessage {
                message: NetworkMessage::DirectSendMsg(DirectSendMsg {
                    protocol_id,
                    priority: 0,
                    raw_msg: bytes,
                }),
                sender: PeerNetworkId::new(network_id, peer_id),
                receive_timestamp_micros: 0,
                rpc_replier: None,
            };
            inbound_handle
                .inbound_message_sender
                .push((peer_id, protocol_id), notif)
                .unwrap();
        }
    }
}

impl TestNode for MempoolNode {}

pub type MempoolTestFrameworkBuilder = TestFrameworkBuilder<MempoolTestFramework, MempoolNode>;

/// A [`TestFramework`] for [`MempoolNode`]s to test Mempool in a single and multi-node mock network
/// environment.
pub struct MempoolTestFramework {
    pub nodes: HashMap<NodeId, MempoolNode>,
}

impl TestFramework<MempoolNode> for MempoolTestFramework {
    fn new(nodes: HashMap<NodeId, MempoolNode>) -> Self {
        Self { nodes }
    }

    fn build_node(
        node_id: NodeId,
        config: NodeConfig,
        peer_network_ids: &[PeerNetworkId],
    ) -> MempoolNode {
        // Collect mappings of network_id to peer_network_id
        let mut network_ids = Vec::new();
        let mut network_id_mapping = HashMap::new();
        for peer_network_id in peer_network_ids {
            let network_id = peer_network_id.network_id();
            assert!(
                !network_id_mapping.contains_key(&network_id),
                "Duplicate network id for node"
            );
            network_ids.push(network_id);
            network_id_mapping.insert(network_id, *peer_network_id);
        }

        let (
            network_client,
            network_service_events,
            inbound_handles,
            outbound_handles,
            peers_and_metadata,
        ) = setup_node_networks(&network_ids);
        let (mempool_client_sender, consensus_to_mempool_sender, mempool_notifications, mempool) =
            setup_mempool(
                config,
                network_client,
                network_service_events,
                peers_and_metadata.clone(),
            );

        MempoolNode {
            node_id,
            peer_network_ids: network_id_mapping,
            mempool,
            mempool_client_sender,
            consensus_to_mempool_sender,
            mempool_notifications,
            inbound_handles,
            outbound_handles,
            other_inbound_handles: HashMap::new(),
            peers_and_metadata,
            request_id_generator: U32IdGenerator::new(),
        }
    }

    fn take_node(&mut self, node_id: NodeId) -> MempoolNode {
        self.nodes.remove(&node_id).expect("Node must exist")
    }
}

/// Setup the multiple networks built for a specific node
pub fn setup_node_networks(
    network_ids: &[NetworkId],
) -> (
    NetworkClient<MempoolSyncMsg>,
    NetworkServiceEvents<MempoolSyncMsg>,
    HashMap<NetworkId, InboundNetworkHandle>,
    HashMap<NetworkId, OutboundMessageReceiver>,
    Arc<PeersAndMetadata>,
) {
    let peers_and_metadata = PeersAndMetadata::new(network_ids);

    // Build each individual network
    let mut network_senders = HashMap::new();
    let mut network_and_events = HashMap::new();
    let mut inbound_handles = HashMap::new();
    let mut outbound_handles = HashMap::new();
    for network_id in network_ids {
        let (network_sender, network_events, inbound_handle, outbound_handle) =
            setup_network(peers_and_metadata.clone());

        network_senders.insert(*network_id, network_sender);
        network_and_events.insert(*network_id, network_events);
        inbound_handles.insert(*network_id, inbound_handle);
        outbound_handles.insert(*network_id, outbound_handle);
    }

    // Create a network client and service events
    let network_client = NetworkClient::new(
        vec![MempoolDirectSend],
        vec![],
        network_senders,
        peers_and_metadata.clone(),
    );
    let network_service_events = NetworkServiceEvents::new(network_and_events);

    (
        network_client,
        network_service_events,
        inbound_handles,
        outbound_handles,
        peers_and_metadata,
    )
}

/// Builds all the channels used for networking
fn setup_network(
    peers_and_metadata: Arc<PeersAndMetadata>,
) -> (
    NetworkSender<MempoolSyncMsg>,
    NetworkEvents<MempoolSyncMsg>,
    InboundNetworkHandle,
    OutboundMessageReceiver,
) {
    let (reqs_inbound_sender, reqs_inbound_receiver) = aptos_channel();
    let (reqs_outbound_sender, reqs_outbound_receiver) = aptos_channel();
    let (connection_outbound_sender, _connection_outbound_receiver) = aptos_channel();

    // Create the network sender and events
    let network_sender = NetworkSender::new(
        PeerManagerRequestSender::new(reqs_outbound_sender),
        ConnectionRequestSender::new(connection_outbound_sender),
    );
    let network_events = NetworkEvents::new(reqs_inbound_receiver, None, true);

    (
        network_sender,
        network_events,
        InboundNetworkHandle {
            inbound_message_sender: reqs_inbound_sender,
            peers_and_metadata,
        },
        reqs_outbound_receiver,
    )
}

/// A generic FIFO Aptos channel
fn aptos_channel<K: Eq + Hash + Clone, T>(
) -> (aptos_channel::Sender<K, T>, aptos_channel::Receiver<K, T>) {
    static MAX_QUEUE_SIZE: usize = 8;
    aptos_channel::new(QueueStyle::FIFO, MAX_QUEUE_SIZE, None)
}

/// Creates a full [`SharedMempool`] and mocks all of the database information.
fn setup_mempool(
    config: NodeConfig,
    network_client: NetworkClient<MempoolSyncMsg>,
    network_service_events: NetworkServiceEvents<MempoolSyncMsg>,
    peers_and_metadata: Arc<PeersAndMetadata>,
) -> (
    MempoolClientSender,
    futures::channel::mpsc::Sender<QuorumStoreRequest>,
    MempoolNotifier,
    Arc<Mutex<CoreMempool>>,
) {
    let (sender, _subscriber) = futures::channel::mpsc::unbounded();
    let (ac_endpoint_sender, ac_endpoint_receiver) = mpsc_channel();
    let (quorum_store_sender, quorum_store_receiver) = mpsc_channel();
    let (mempool_notifier, mempool_listener) =
        aptos_mempool_notifications::new_mempool_notifier_listener_pair(100);

    let mempool = Arc::new(Mutex::new(CoreMempool::new(&config)));
    let vm_validator = Arc::new(RwLock::new(MockVMValidator));
    let db_ro = Arc::new(MockDbReaderWriter);

    let (reconfig_sender, reconfig_events) = aptos_channel::new(QueueStyle::LIFO, 1, None);
    let reconfig_event_subscriber = ReconfigNotificationListener {
        notification_receiver: reconfig_events,
    };
    reconfig_sender
        .push((), ReconfigNotification {
            version: 1,
            on_chain_configs: OnChainConfigPayload::new(
                1,
                InMemoryOnChainConfig::new(HashMap::new()),
            ),
        })
        .unwrap();

    start_shared_mempool(
        &Handle::current(),
        &config,
        mempool.clone(),
        network_client,
        network_service_events,
        ac_endpoint_receiver,
        quorum_store_receiver,
        mempool_listener,
        reconfig_event_subscriber,
        db_ro,
        vm_validator,
        vec![sender],
        peers_and_metadata,
    );

    (
        ac_endpoint_sender,
        quorum_store_sender,
        mempool_notifier,
        mempool,
    )
}

fn mpsc_channel<T>() -> (
    futures::channel::mpsc::Sender<T>,
    futures::channel::mpsc::Receiver<T>,
) {
    futures::channel::mpsc::channel(1_024)
}

/// Creates a single [`TestTransaction`] with the given `seq_num`.
pub fn test_transaction(seq_num: u64) -> TestTransaction {
    TestTransaction {
        address: TestTransaction::get_address(1),
        replay_protector: ReplayProtector::SequenceNumber(seq_num),
        gas_price: 1,
        script: None,
    }
}

/// Tells us if a [`SignedTransaction`] block contains only the [`TestTransaction`]s
pub fn block_only_contains_transactions(
    block: &[SignedTransaction],
    txns: &[TestTransaction],
) -> bool {
    txns.iter()
        .all(|txn| block_contains_transaction(block, txn))
        && block.len() == txns.len()
}

/// Tells us if a [`SignedTransaction`] block contains all the [`TestTransaction`]s
pub fn block_contains_all_transactions(
    block: &[SignedTransaction],
    txns: &[TestTransaction],
) -> bool {
    txns.iter()
        .all(|txn| block_contains_transaction(block, txn))
}

/// Tells us if a [`SignedTransaction`] block contains any of the [`TestTransaction`]s
pub fn block_contains_any_transaction(
    block: &[SignedTransaction],
    txns: &[TestTransaction],
) -> bool {
    txns.iter()
        .any(|txn| block_contains_transaction(block, txn))
        && block.len() == txns.len()
}

/// Tells us if a [`SignedTransaction`] block contains the [`TestTransaction`]
fn block_contains_transaction(block: &[SignedTransaction], txn: &TestTransaction) -> bool {
    block.iter().any(|signed_txn| {
        signed_txn.replay_protector() == txn.replay_protector
            && signed_txn.sender() == txn.address
            && signed_txn.gas_unit_price() == txn.gas_price
    })
}

/// Signs [`TestTransaction`]s with a max gas amount
pub fn sign_transactions(txns: &[TestTransaction]) -> Vec<SignedTransaction> {
    txns.iter()
        .map(|txn| txn.make_signed_transaction_with_max_gas_amount(5))
        .collect()
}
