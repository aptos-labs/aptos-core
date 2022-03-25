// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    application::storage::PeerMetadataStorage,
    peer_manager::{ConnectionRequestSender, PeerManagerRequestSender},
    protocols::network::{NewNetworkEvents, NewNetworkSender},
    testutils::test_node::{
        ApplicationNetworkHandle, ApplicationNode, InboundNetworkHandle, NodeId,
        OutboundMessageReceiver,
    },
};
use aptos_config::{
    config::NodeConfig,
    network_id::{NetworkId, PeerNetworkId},
};
use channel::message_queues::QueueStyle;
use std::{collections::HashMap, hash::Hash, sync::Arc, vec::Vec};

/// A trait describing a test framework for a specific application
///
/// This is essentially an abstract implementation, to get around how rust handles traits
/// there are functions to get required variables in the implementation.
///
pub trait TestFramework<Node: ApplicationNode + Sync> {
    /// Constructor for the [`TestFramework`]
    fn new(nodes: HashMap<NodeId, Node>) -> Self;

    /// A constructor for `Node` specific to the application
    fn build_node(node_id: NodeId, config: NodeConfig, peer_network_ids: &[PeerNetworkId]) -> Node;

    /// In order to have separate tasks, we have to pull these out of the framework
    fn take_node(&mut self, node_id: NodeId) -> Node;
}

/// Setup the multiple networks built for a specific node
pub fn setup_node_networks<NetworkSender: NewNetworkSender, NetworkEvents: NewNetworkEvents>(
    network_ids: &[NetworkId],
) -> (
    Vec<ApplicationNetworkHandle<NetworkSender, NetworkEvents>>,
    HashMap<NetworkId, InboundNetworkHandle>,
    HashMap<NetworkId, OutboundMessageReceiver>,
    Arc<PeerMetadataStorage>,
) {
    let mut application_handles = Vec::new();
    let mut inbound_handles = HashMap::new();
    let mut outbound_handles = HashMap::new();

    let peer_metadata_storage = PeerMetadataStorage::new(network_ids);

    // Build each individual network
    for network_id in network_ids {
        let (application_handle, inbound_handle, outbound_handle) =
            setup_network(*network_id, peer_metadata_storage.clone());
        application_handles.push(application_handle);
        inbound_handles.insert(*network_id, inbound_handle);
        outbound_handles.insert(*network_id, outbound_handle);
    }

    (
        application_handles,
        inbound_handles,
        outbound_handles,
        peer_metadata_storage,
    )
}

/// Builds all the channels used for networking
fn setup_network<NetworkSender: NewNetworkSender, NetworkEvents: NewNetworkEvents>(
    network_id: NetworkId,
    peer_metadata_storage: Arc<PeerMetadataStorage>,
) -> (
    ApplicationNetworkHandle<NetworkSender, NetworkEvents>,
    InboundNetworkHandle,
    OutboundMessageReceiver,
) {
    let (reqs_inbound_sender, reqs_inbound_receiver) = aptos_channel();
    let (reqs_outbound_sender, reqs_outbound_receiver) = aptos_channel();
    let (connection_outbound_sender, _connection_outbound_receiver) = aptos_channel();
    let (connection_inbound_sender, connection_inbound_receiver) =
        crate::peer_manager::conn_notifs_channel::new();
    let network_sender = NetworkSender::new(
        PeerManagerRequestSender::new(reqs_outbound_sender),
        ConnectionRequestSender::new(connection_outbound_sender),
    );
    let network_events = NetworkEvents::new(reqs_inbound_receiver, connection_inbound_receiver);

    (
        (network_id, network_sender, network_events),
        InboundNetworkHandle {
            inbound_message_sender: reqs_inbound_sender,
            connection_update_sender: connection_inbound_sender,
            peer_metadata_storage,
        },
        reqs_outbound_receiver,
    )
}

/// A generic FIFO Aptos channel
fn aptos_channel<K: Eq + Hash + Clone, T>() -> (
    channel::aptos_channel::Sender<K, T>,
    channel::aptos_channel::Receiver<K, T>,
) {
    static MAX_QUEUE_SIZE: usize = 8;
    channel::aptos_channel::new(QueueStyle::FIFO, MAX_QUEUE_SIZE, None)
}
