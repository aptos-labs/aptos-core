// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    core_mempool::{CoreMempool, TimelineState},
    network::{MempoolNetworkEvents, MempoolSyncMsg},
    shared_mempool::{
        network::MempoolNetworkSender, start_shared_mempool, types::SharedMempoolNotification,
    },
    tests::common::TestTransaction,
};
use channel::{diem_channel, message_queues::QueueStyle};
use diem_config::{
    config::{Identity, NodeConfig, PeerRole, RoleType},
    network_id::{NetworkContext, NetworkId, PeerNetworkId},
};
use diem_crypto::{x25519::PrivateKey, Uniform};
use diem_infallible::{Mutex, MutexGuard, RwLock};
use diem_types::{
    account_config::AccountSequenceInfo, on_chain_config::ON_CHAIN_CONFIG_REGISTRY,
    transaction::GovernanceRole, PeerId,
};
use enum_dispatch::enum_dispatch;
use event_notifications::EventSubscriptionService;
use futures::{
    channel::mpsc::{self, unbounded, UnboundedReceiver},
    FutureExt, StreamExt,
};
use netcore::transport::ConnectionOrigin;
use network::{
    application::storage::PeerMetadataStorage,
    peer_manager::{
        conn_notifs_channel, ConnectionNotification, ConnectionRequestSender,
        PeerManagerNotification, PeerManagerRequest, PeerManagerRequestSender,
    },
    protocols::network::{NetworkEvents, NewNetworkEvents, NewNetworkSender},
    transport::ConnectionMetadata,
    ProtocolId,
};
use rand::rngs::StdRng;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use storage_interface::{mock::MockDbReaderWriter, DbReaderWriter};
use tokio::runtime::{Builder, Runtime};
use vm_validator::mocks::mock_vm_validator::MockVMValidator;

type MempoolNetworkHandle = (
    NetworkId,
    MempoolNetworkSender,
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
pub fn validator_config(rng: &mut StdRng, account_idx: u32) -> (ValidatorNodeInfo, NodeConfig) {
    let config =
        NodeConfig::random_with_template(account_idx, &NodeConfig::default_for_validator(), rng);

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
pub fn vfn_config(
    rng: &mut StdRng,
    account_idx: u32,
    peer_id: PeerId,
) -> (ValidatorFullNodeInfo, NodeConfig) {
    let mut vfn_config = NodeConfig::random_with_template(
        account_idx,
        &NodeConfig::default_for_validator_full_node(),
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
    account_idx: u32,
    peer_role: PeerRole,
) -> (FullNodeInfo, NodeConfig) {
    let fn_config = NodeConfig::random_with_template(
        account_idx,
        &NodeConfig::default_for_public_full_node(),
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
    peer_metadata_storage: Arc<PeerMetadataStorage>,
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
        let (network_interfaces, network_handles, peer_metadata_storage) =
            setup_node_network_interfaces(&node);
        let (mempool, runtime, subscriber) =
            start_node_mempool(config, network_handles, peer_metadata_storage.clone());

        Node {
            node_info: node,
            mempool,
            network_interfaces,
            runtime: Arc::new(runtime),
            subscriber,
            peer_metadata_storage,
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
            mempool.add_txn(
                transaction.clone(),
                0,
                transaction.gas_unit_price(),
                AccountSequenceInfo::Sequential(0),
                TimelineState::NotReady,
                GovernanceRole::NonGovernanceRole,
            );
        }
    }

    /// Notifies the `Node` of a `new_peer`
    pub fn send_new_peer_event(
        &mut self,
        new_peer: PeerNetworkId,
        peer_role: PeerRole,
        origin: ConnectionOrigin,
    ) {
        let mut metadata =
            ConnectionMetadata::mock_with_role_and_origin(new_peer.peer_id(), peer_role, origin);
        metadata
            .application_protocols
            .insert(ProtocolId::MempoolDirectSend);
        let notif = ConnectionNotification::NewPeer(metadata.clone(), NetworkContext::mock());
        self.peer_metadata_storage
            .insert_connection(new_peer.network_id(), metadata);
        self.send_connection_event(new_peer.network_id(), notif);
    }

    /// Sends a connection event, and waits for the notification to arrive
    fn send_connection_event(&mut self, network_id: NetworkId, notif: ConnectionNotification) {
        self.send_network_notif(network_id, notif);
        self.wait_for_event(SharedMempoolNotification::PeerStateChange);
    }

    /// Waits for a specific `SharedMempoolNotification` event
    pub fn wait_for_event(&mut self, expected: SharedMempoolNotification) {
        let event = self.runtime.block_on(self.subscriber.next()).unwrap();
        if event == expected {
            return;
        }

        panic!("Failed to get expected event '{:?}'", expected)
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

    /// Send network request `PeerManagerNotification` from a remote peer to the local node
    pub fn send_network_req(
        &mut self,
        network_id: NetworkId,
        protocol: ProtocolId,
        notif: PeerManagerNotification,
    ) {
        self.get_network_interface(network_id)
            .send_network_req(protocol, notif);
    }

    /// Sends a `ConnectionNotification` to the local node
    pub fn send_network_notif(&mut self, network_id: NetworkId, notif: ConnectionNotification) {
        self.get_network_interface(network_id)
            .send_connection_notif(notif)
    }
}

/// A simplistic view of the entire network stack for a given `NetworkId`
/// Allows us to mock out the network without dealing with the details
pub struct NodeNetworkInterface {
    /// Peer request receiver for messages
    pub(crate) network_reqs_rx: diem_channel::Receiver<(PeerId, ProtocolId), PeerManagerRequest>,
    /// Peer notification sender for sending outgoing messages to other peers
    pub(crate) network_notifs_tx:
        diem_channel::Sender<(PeerId, ProtocolId), PeerManagerNotification>,
    /// Sender for connecting / disconnecting peers
    pub(crate) network_conn_event_notifs_tx: conn_notifs_channel::Sender,
}

impl NodeNetworkInterface {
    fn get_next_network_req(&mut self, runtime: Arc<Runtime>) -> PeerManagerRequest {
        runtime.block_on(self.network_reqs_rx.next()).unwrap()
    }

    fn send_network_req(&mut self, protocol: ProtocolId, message: PeerManagerNotification) {
        let remote_peer_id = match &message {
            PeerManagerNotification::RecvRpc(peer_id, _) => *peer_id,
            PeerManagerNotification::RecvMessage(peer_id, _) => *peer_id,
        };

        self.network_notifs_tx
            .push((remote_peer_id, protocol), message)
            .unwrap()
    }

    /// Send a notification specifying, where a remote peer has it's state changed
    fn send_connection_notif(&mut self, notif: ConnectionNotification) {
        let peer_id = match &notif {
            ConnectionNotification::NewPeer(metadata, _) => metadata.remote_peer_id,
            ConnectionNotification::LostPeer(metadata, _, _) => metadata.remote_peer_id,
        };

        self.network_conn_event_notifs_tx
            .push(peer_id, notif)
            .unwrap()
    }
}

// Below here are static functions to help build a new `Node`

/// Sets up the network handles for a `Node`
fn setup_node_network_interfaces(
    node: &NodeInfo,
) -> (
    HashMap<NetworkId, NodeNetworkInterface>,
    Vec<MempoolNetworkHandle>,
    Arc<PeerMetadataStorage>,
) {
    let mut network_handles = vec![];
    let mut network_interfaces = HashMap::new();
    for network in node.supported_networks() {
        let (network_interface, network_handle) =
            setup_node_network_interface(PeerNetworkId::new(network, node.peer_id(network)));

        network_handles.push(network_handle);
        network_interfaces.insert(network, network_interface);
    }

    let network_ids: Vec<_> = network_handles
        .iter()
        .map(|(network_id, _, _)| *network_id)
        .collect();
    let peer_metadata_storage = PeerMetadataStorage::new(&network_ids);
    (network_interfaces, network_handles, peer_metadata_storage)
}

/// Builds a single network interface with associated queues, and attaches it to the top level network
fn setup_node_network_interface(
    peer_network_id: PeerNetworkId,
) -> (NodeNetworkInterface, MempoolNetworkHandle) {
    static MAX_QUEUE_SIZE: usize = 8;
    let (network_reqs_tx, network_reqs_rx) =
        diem_channel::new(QueueStyle::FIFO, MAX_QUEUE_SIZE, None);
    let (connection_reqs_tx, _) = diem_channel::new(QueueStyle::FIFO, MAX_QUEUE_SIZE, None);
    let (network_notifs_tx, network_notifs_rx) =
        diem_channel::new(QueueStyle::FIFO, MAX_QUEUE_SIZE, None);
    let (network_conn_event_notifs_tx, conn_status_rx) = conn_notifs_channel::new();
    let network_sender = MempoolNetworkSender::new(
        PeerManagerRequestSender::new(network_reqs_tx),
        ConnectionRequestSender::new(connection_reqs_tx),
    );
    let network_events = MempoolNetworkEvents::new(network_notifs_rx, conn_status_rx);

    (
        NodeNetworkInterface {
            network_reqs_rx,
            network_notifs_tx,
            network_conn_event_notifs_tx,
        },
        (peer_network_id.network_id(), network_sender, network_events),
    )
}

/// Starts up the mempool resources for a single node
fn start_node_mempool(
    config: NodeConfig,
    network_handles: Vec<MempoolNetworkHandle>,
    peer_metadata_storage: Arc<PeerMetadataStorage>,
) -> (
    Arc<Mutex<CoreMempool>>,
    Runtime,
    UnboundedReceiver<SharedMempoolNotification>,
) {
    let mempool = Arc::new(Mutex::new(CoreMempool::new(&config)));
    let (sender, subscriber) = unbounded();
    let (_ac_endpoint_sender, ac_endpoint_receiver) = mpsc::channel(1_024);
    let (_consensus_sender, consensus_events) = mpsc::channel(1_024);
    let (_mempool_notifier, mempool_listener) =
        mempool_notifications::new_mempool_notifier_listener_pair();
    let mut event_subscriber = EventSubscriptionService::new(
        ON_CHAIN_CONFIG_REGISTRY,
        Arc::new(RwLock::new(DbReaderWriter::new(MockDbReaderWriter))),
    );
    let reconfig_event_subscriber = event_subscriber.subscribe_to_reconfigurations().unwrap();
    let runtime = Builder::new_multi_thread()
        .thread_name("shared-mem")
        .enable_all()
        .build()
        .expect("[shared mempool] failed to create runtime");
    start_shared_mempool(
        runtime.handle(),
        &config,
        Arc::clone(&mempool),
        network_handles,
        ac_endpoint_receiver,
        consensus_events,
        mempool_listener,
        reconfig_event_subscriber,
        Arc::new(MockDbReaderWriter),
        Arc::new(RwLock::new(MockVMValidator)),
        vec![sender],
        peer_metadata_storage,
    );

    (mempool, runtime, subscriber)
}
