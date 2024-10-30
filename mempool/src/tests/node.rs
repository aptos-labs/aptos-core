// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    core_mempool::{AccountSequenceNumberInfo, CoreMempool, TimelineState},
    network::{BroadcastPeerPriority, MempoolSyncMsg},
    shared_mempool::{start_shared_mempool, types::SharedMempoolNotification},
    tests::common::TestTransaction,
};
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_config::{
    config::{Identity, NodeConfig, PeerRole, RoleType},
    network_id::{NetworkId, PeerNetworkId},
};
use aptos_crypto::{x25519::PrivateKey, Uniform};
use aptos_event_notifications::{ReconfigNotification, ReconfigNotificationListener};
use aptos_infallible::{Mutex, MutexGuard, RwLock};
use aptos_netcore::transport::ConnectionOrigin;
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
        wire::handshake::v1::ProtocolId::MempoolDirectSend,
    },
    transport::ConnectionMetadata,
    ProtocolId,
};
use aptos_storage_interface::mock::MockDbReaderWriter;
use aptos_types::{
    on_chain_config::{InMemoryOnChainConfig, OnChainConfigPayload},
    transaction::ReplayProtector,
    PeerId,
};
use aptos_vm_validator::mocks::mock_vm_validator::MockVMValidator;
use enum_dispatch::enum_dispatch;
use futures::{
    channel::mpsc::{self, unbounded, UnboundedReceiver},
    FutureExt, StreamExt,
};
use rand::rngs::StdRng;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tokio::runtime::Runtime;

type MempoolNetworkHandle = (
    NetworkId,
    NetworkSender<MempoolSyncMsg>,
    NetworkEvents<MempoolSyncMsg>,
);

/// This is a simple node identifier for testing
/// This keeps track of the `NodeType` and a simple index
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialOrd, PartialEq)]
pub struct NodeId {
    pub node_type: NodeType,
    num: u32,
}

impl NodeId {
    pub(crate) fn new(node_type: NodeType, num: u32) -> Self {
        NodeId { node_type, num }
    }
}

/// Yet another type, to determine the differences between
/// Validators, ValidatorFullNodes, and FullNodes
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialOrd, PartialEq)]
pub enum NodeType {
    Validator,
    ValidatorFullNode,
    FullNode,
}

/// A union type for all types of simulated nodes
#[enum_dispatch(NodeInfoTrait)]
#[derive(Clone, Debug)]
pub enum NodeInfo {
    Validator(ValidatorNodeInfo),
    ValidatorFull(ValidatorFullNodeInfo),
    Full(FullNodeInfo),
}

/// Accessors to the union type of all simulated nodes
#[enum_dispatch]
pub trait NodeInfoTrait {
    fn supported_networks(&self) -> Vec<NetworkId>;

    fn find_common_network<T: NodeInfoTrait>(&self, other: &T) -> NetworkId {
        let supported: HashSet<_> = self.supported_networks().into_iter().collect();
        let other_supported: HashSet<_> = other.supported_networks().into_iter().collect();
        if supported.contains(&NetworkId::Validator)
            && other_supported.contains(&NetworkId::Validator)
        {
            NetworkId::Validator
        } else if supported.contains(&NetworkId::Public)
            && other_supported.contains(&NetworkId::Public)
        {
            NetworkId::Public
        } else if supported.contains(&NetworkId::Vfn) && other_supported.contains(&NetworkId::Vfn) {
            NetworkId::Vfn
        } else {
            panic!("Expected a common network")
        }
    }

    fn peer_network_ids(&self) -> Vec<PeerNetworkId> {
        self.supported_networks()
            .into_iter()
            .map(|network| self.peer_network_id(network))
            .collect()
    }

    fn peer_id(&self, network_id: NetworkId) -> PeerId;

    fn peer_network_id(&self, network_id: NetworkId) -> PeerNetworkId {
        PeerNetworkId::new(network_id, self.peer_id(network_id))
    }

    /// `RoleType` of the `Node`
    fn role(&self) -> RoleType;

    /// `PeerRole` for use in the upstream / downstream peers
    fn peer_role(&self) -> PeerRole;
}

#[derive(Clone, Debug)]
pub struct ValidatorNodeInfo {
    peer_id: PeerId,
    vfn_peer_id: PeerId,
}

impl ValidatorNodeInfo {
    fn new(peer_id: PeerId, vfn_peer_id: PeerId) -> Self {
        ValidatorNodeInfo {
            peer_id,
            vfn_peer_id,
        }
    }
}

impl NodeInfoTrait for ValidatorNodeInfo {
    fn supported_networks(&self) -> Vec<NetworkId> {
        vec![NetworkId::Validator, NetworkId::Vfn]
    }

    fn peer_id(&self, network_id: NetworkId) -> PeerId {
        match network_id {
            NetworkId::Validator => self.peer_id,
            NetworkId::Vfn => self.vfn_peer_id,
            NetworkId::Public => panic!("Invalid network id for validator"),
        }
    }

    fn role(&self) -> RoleType {
        RoleType::Validator
    }

    fn peer_role(&self) -> PeerRole {
        PeerRole::Validator
    }
}

#[derive(Clone, Debug)]
pub struct ValidatorFullNodeInfo {
    peer_id: PeerId,
    vfn_peer_id: PeerId,
}

impl ValidatorFullNodeInfo {
    fn new(peer_id: PeerId, vfn_peer_id: PeerId) -> Self {
        ValidatorFullNodeInfo {
            peer_id,
            vfn_peer_id,
        }
    }
}

impl NodeInfoTrait for ValidatorFullNodeInfo {
    fn supported_networks(&self) -> Vec<NetworkId> {
        vec![NetworkId::Public, NetworkId::Vfn]
    }

    fn peer_id(&self, network_id: NetworkId) -> PeerId {
        match network_id {
            NetworkId::Public => self.peer_id,
            NetworkId::Vfn => self.vfn_peer_id,
            NetworkId::Validator => panic!("Invalid network id for validator full node"),
        }
    }

    fn role(&self) -> RoleType {
        RoleType::FullNode
    }

    fn peer_role(&self) -> PeerRole {
        PeerRole::ValidatorFullNode
    }
}

#[derive(Clone, Debug)]
pub struct FullNodeInfo {
    peer_id: PeerId,
    peer_role: PeerRole,
}

impl FullNodeInfo {
    fn new(peer_id: PeerId, peer_role: PeerRole) -> Self {
        FullNodeInfo { peer_id, peer_role }
    }
}

impl NodeInfoTrait for FullNodeInfo {
    fn supported_networks(&self) -> Vec<NetworkId> {
        vec![NetworkId::Public]
    }

    fn peer_id(&self, network_id: NetworkId) -> PeerId {
        if NetworkId::Public == network_id {
            self.peer_id
        } else {
            panic!("Invalid network id for public full node")
        }
    }

    fn role(&self) -> RoleType {
        RoleType::FullNode
    }

    fn peer_role(&self) -> PeerRole {
        self.peer_role
    }
}

/// Provides a `NodeInfo` and `NodeConfig` for a validator
pub fn validator_config(rng: &mut StdRng) -> (ValidatorNodeInfo, NodeConfig) {
    let config = NodeConfig::generate_random_config_with_template(
        &NodeConfig::get_default_validator_config(),
        rng,
    );

    let peer_id = config
        .validator_network
        .as_ref()
        .expect("Validator must have a validator network")
        .peer_id();
    (
        ValidatorNodeInfo::new(peer_id, PeerId::from_hex_literal("0xDEADBEEF").unwrap()),
        config,
    )
}

/// Provides a `NodeInfo` and `NodeConfig` for a ValidatorFullNode
pub fn vfn_config(rng: &mut StdRng, peer_id: PeerId) -> (ValidatorFullNodeInfo, NodeConfig) {
    let mut vfn_config = NodeConfig::generate_random_config_with_template(
        &NodeConfig::get_default_vfn_config(),
        rng,
    );

    vfn_config
        .full_node_networks
        .iter_mut()
        .find(|network| network.network_id == NetworkId::Public)
        .as_mut()
        .unwrap()
        .identity = Identity::from_config(PrivateKey::generate_for_testing(), peer_id);

    let networks: HashMap<_, _> = vfn_config
        .full_node_networks
        .iter()
        .map(|network| (network.network_id, network.peer_id()))
        .collect();
    (
        ValidatorFullNodeInfo::new(
            *networks
                .get(&NetworkId::Public)
                .expect("VFN config should have a public network"),
            *networks
                .get(&NetworkId::Vfn)
                .expect("VFN config should have a vfn network"),
        ),
        vfn_config,
    )
}

/// Provides a `NodeInfo` and `NodeConfig` for a public full node
pub fn public_full_node_config(
    rng: &mut StdRng,
    peer_role: PeerRole,
) -> (FullNodeInfo, NodeConfig) {
    let fn_config = NodeConfig::generate_random_config_with_template(
        &NodeConfig::get_default_pfn_config(),
        rng,
    );

    let peer_id = fn_config
        .full_node_networks
        .iter()
        .find(|network| network.network_id == NetworkId::Public)
        .expect("Full Node must have a public network")
        .peer_id();
    (FullNodeInfo::new(peer_id, peer_role), fn_config)
}

/// A struct representing a node, it's network interfaces, mempool, and a mempool event subscriber
pub struct Node {
    /// The identifying Node
    node_info: NodeInfo,
    /// `CoreMempool` for this node
    mempool: Arc<Mutex<CoreMempool>>,
    /// Network interfaces for a node
    network_interfaces: HashMap<NetworkId, NodeNetworkInterface>,
    /// Tokio runtime
    runtime: Arc<Runtime>,
    /// Subscriber for mempool events
    subscriber: UnboundedReceiver<SharedMempoolNotification>,
    /// Global peer connection data
    peers_and_metadata: Arc<PeersAndMetadata>,
}

/// Reimplement `NodeInfoTrait` for simplicity
impl NodeInfoTrait for Node {
    fn supported_networks(&self) -> Vec<NetworkId> {
        self.node_info.supported_networks()
    }

    fn peer_id(&self, network_id: NetworkId) -> PeerId {
        self.node_info.peer_id(network_id)
    }

    fn role(&self) -> RoleType {
        self.node_info.role()
    }

    fn peer_role(&self) -> PeerRole {
        self.node_info.peer_role()
    }
}

impl Node {
    /// Sets up a single node by starting up mempool and any network handles
    pub fn new(node: NodeInfo, config: NodeConfig) -> Node {
        let (network_interfaces, network_client, network_service_events, peers_and_metadata) =
            setup_node_network_interfaces(&node);
        let (mempool, runtime, subscriber) = start_node_mempool(
            config,
            network_client,
            network_service_events,
            peers_and_metadata.clone(),
        );

        Node {
            node_info: node,
            mempool,
            network_interfaces,
            runtime: Arc::new(runtime),
            subscriber,
            peers_and_metadata,
        }
    }

    /// Retrieves a `CoreMempool`
    pub fn mempool(&self) -> MutexGuard<CoreMempool> {
        self.mempool.lock()
    }

    /// Queues transactions for sending on a node.  Must use `broadcast_txns` to send to other nodes
    pub fn add_txns(&self, txns: Vec<TestTransaction>) {
        let mut mempool = self.mempool();
        for txn in txns {
            let transaction = txn.make_signed_transaction_with_max_gas_amount(5);
            let account_sequence_number = match transaction.replay_protector() {
                ReplayProtector::SequenceNumber(_) => AccountSequenceNumberInfo::Required(0),
                ReplayProtector::Nonce(_) => AccountSequenceNumberInfo::NotRequired,
            };
            mempool.add_txn(
                transaction.clone(),
                transaction.gas_unit_price(),
                account_sequence_number,
                TimelineState::NotReady,
                false,
                None,
                Some(BroadcastPeerPriority::Primary),
            );
        }
    }

    /// Notifies the `Node` of a `new_peer`
    pub fn send_new_peer_event(
        &mut self,
        peer_network_id: PeerNetworkId,
        peer_role: PeerRole,
        origin: ConnectionOrigin,
    ) {
        let mut metadata = ConnectionMetadata::mock_with_role_and_origin(
            peer_network_id.peer_id(),
            peer_role,
            origin,
        );
        metadata
            .application_protocols
            .insert(ProtocolId::MempoolDirectSend);
        self.peers_and_metadata
            .insert_connection_metadata(peer_network_id, metadata)
            .unwrap();
        self.wait_for_event(SharedMempoolNotification::PeerStateChange);
    }

    /// Waits for a specific `SharedMempoolNotification` event
    pub fn wait_for_event(&mut self, expected: SharedMempoolNotification) {
        let event = self.runtime.block_on(self.subscriber.next()).unwrap();
        if event == expected {
            return;
        }

        panic!(
            "Failed to get expected event '{:?}', instead: '{:?}'",
            expected, event
        )
    }

    /// Checks that there are no `SharedMempoolNotification`s on the subscriber
    pub fn check_no_subscriber_events(&mut self) {
        assert!(self.subscriber.select_next_some().now_or_never().is_none())
    }

    /// Checks that a node has no pending messages to send.
    pub fn check_no_network_messages_sent(&mut self, network_id: NetworkId) {
        self.check_no_subscriber_events();
        assert!(self
            .get_network_interface(network_id)
            .network_reqs_rx
            .select_next_some()
            .now_or_never()
            .is_none())
    }

    /// Retrieves a network interface for a specific `NetworkId` based on whether it's the primary network
    fn get_network_interface(&mut self, network_id: NetworkId) -> &mut NodeNetworkInterface {
        self.network_interfaces.get_mut(&network_id).unwrap()
    }

    /// Retrieves the next network request `PeerManagerRequest`
    pub fn get_next_network_req(&mut self, network_id: NetworkId) -> PeerManagerRequest {
        let runtime = self.runtime.clone();
        self.get_network_interface(network_id)
            .get_next_network_req(runtime)
    }

    /// Send network request `ReceivedMessage` from a remote peer to the local node
    pub fn send_network_req(
        &mut self,
        network_id: NetworkId,
        protocol: ProtocolId,
        notif: ReceivedMessage,
    ) {
        self.get_network_interface(network_id)
            .send_network_req(protocol, notif);
    }
}

/// A simplistic view of the entire network stack for a given `NetworkId`
/// Allows us to mock out the network without dealing with the details
pub struct NodeNetworkInterface {
    /// Peer request receiver for messages
    pub(crate) network_reqs_rx: aptos_channel::Receiver<(PeerId, ProtocolId), PeerManagerRequest>,
    /// Peer notification sender for sending outgoing messages to other peers
    pub(crate) network_notifs_tx: aptos_channel::Sender<(PeerId, ProtocolId), ReceivedMessage>,
}

impl NodeNetworkInterface {
    fn get_next_network_req(&mut self, runtime: Arc<Runtime>) -> PeerManagerRequest {
        runtime.block_on(self.network_reqs_rx.next()).unwrap()
    }

    fn send_network_req(&mut self, protocol: ProtocolId, message: ReceivedMessage) {
        let remote_peer_id = message.sender.peer_id();

        self.network_notifs_tx
            .push((remote_peer_id, protocol), message)
            .unwrap()
    }
}

// Below here are static functions to help build a new `Node`

/// Sets up the network handles for a `Node`
fn setup_node_network_interfaces(
    node: &NodeInfo,
) -> (
    HashMap<NetworkId, NodeNetworkInterface>,
    NetworkClient<MempoolSyncMsg>,
    NetworkServiceEvents<MempoolSyncMsg>,
    Arc<PeersAndMetadata>,
) {
    // Create the peers and metadata
    let network_ids = node.supported_networks();
    let peers_and_metadata = PeersAndMetadata::new(&network_ids);

    // Create the network interfaces
    let mut network_senders = HashMap::new();
    let mut network_and_events = HashMap::new();
    let mut network_interfaces = HashMap::new();
    for network_id in network_ids {
        let (network_interface, network_handle) =
            setup_node_network_interface(PeerNetworkId::new(network_id, node.peer_id(network_id)));

        network_senders.insert(network_id, network_handle.1);
        network_and_events.insert(network_id, network_handle.2);
        network_interfaces.insert(network_id, network_interface);
    }

    // Create the client and service events
    let network_client = NetworkClient::new(
        vec![MempoolDirectSend],
        vec![],
        network_senders,
        peers_and_metadata.clone(),
    );
    let network_service_events = NetworkServiceEvents::new(network_and_events);

    (
        network_interfaces,
        network_client,
        network_service_events,
        peers_and_metadata,
    )
}

/// Builds a single network interface with associated queues, and attaches it to the top level network
fn setup_node_network_interface(
    peer_network_id: PeerNetworkId,
) -> (NodeNetworkInterface, MempoolNetworkHandle) {
    // Create the network sender and events receiver
    static MAX_QUEUE_SIZE: usize = 8;
    let (network_reqs_tx, network_reqs_rx) =
        aptos_channel::new(QueueStyle::FIFO, MAX_QUEUE_SIZE, None);
    let (connection_reqs_tx, _) = aptos_channel::new(QueueStyle::FIFO, MAX_QUEUE_SIZE, None);
    let (network_notifs_tx, network_notifs_rx) =
        aptos_channel::new(QueueStyle::FIFO, MAX_QUEUE_SIZE, None);
    let network_sender = NetworkSender::new(
        PeerManagerRequestSender::new(network_reqs_tx),
        ConnectionRequestSender::new(connection_reqs_tx),
    );
    let network_events = NetworkEvents::new(network_notifs_rx, None, true);

    (
        NodeNetworkInterface {
            network_reqs_rx,
            network_notifs_tx,
        },
        (peer_network_id.network_id(), network_sender, network_events),
    )
}

/// Starts up the mempool resources for a single node
fn start_node_mempool(
    config: NodeConfig,
    network_client: NetworkClient<MempoolSyncMsg>,
    network_service_events: NetworkServiceEvents<MempoolSyncMsg>,
    peers_and_metadata: Arc<PeersAndMetadata>,
) -> (
    Arc<Mutex<CoreMempool>>,
    Runtime,
    UnboundedReceiver<SharedMempoolNotification>,
) {
    let mempool = Arc::new(Mutex::new(CoreMempool::new(&config)));
    let (sender, subscriber) = unbounded();
    let (_ac_endpoint_sender, ac_endpoint_receiver) = mpsc::channel(1_024);
    let (_quorum_store_sender, quorum_store_receiver) = mpsc::channel(1_024);
    let (_mempool_notifier, mempool_listener) =
        aptos_mempool_notifications::new_mempool_notifier_listener_pair(100);
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

    let runtime = aptos_runtimes::spawn_named_runtime("shared-mem".into(), None);
    start_shared_mempool(
        runtime.handle(),
        &config,
        Arc::clone(&mempool),
        network_client,
        network_service_events,
        ac_endpoint_receiver,
        quorum_store_receiver,
        mempool_listener,
        reconfig_event_subscriber,
        Arc::new(MockDbReaderWriter),
        Arc::new(RwLock::new(MockVMValidator)),
        vec![sender],
        peers_and_metadata,
    );

    (mempool, runtime, subscriber)
}
