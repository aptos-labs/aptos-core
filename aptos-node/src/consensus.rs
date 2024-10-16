// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{network::ApplicationNetworkInterfaces, services};
use aptos_admin_service::AdminService;
use aptos_channels::aptos_channel::Receiver;
use aptos_config::config::NodeConfig;
use aptos_consensus::{
    consensus_observer::{
        network::{
            network_events::ConsensusObserverNetworkEvents,
            network_handler::{
                ConsensusObserverNetworkHandler, ConsensusObserverNetworkMessage,
                ConsensusPublisherNetworkMessage,
            },
            observer_client::ConsensusObserverClient,
            observer_message::ConsensusObserverMessage,
        },
        publisher::consensus_publisher::ConsensusPublisher,
    },
    consensus_provider::start_consensus_observer,
    network_interface::ConsensusMsg,
};
use aptos_consensus_notifications::ConsensusNotifier;
use aptos_dkg_runtime::{start_dkg_runtime, DKGMessage};
use aptos_event_notifications::{
    DbBackedOnChainConfig, EventNotificationListener, ReconfigNotificationListener,
};
use aptos_jwk_consensus::{start_jwk_consensus_runtime, types::JWKConsensusMsg};
use aptos_mempool::QuorumStoreRequest;
use aptos_network::application::interface::{NetworkClient, NetworkServiceEvents};
use aptos_storage_interface::DbReaderWriter;
use aptos_validator_transaction_pool::VTxnPoolState;
use futures::channel::mpsc::Sender;
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Creates and starts the consensus runtime (if enabled)
pub fn create_consensus_runtime(
    node_config: &NodeConfig,
    db_rw: DbReaderWriter,
    consensus_reconfig_subscription: Option<ReconfigNotificationListener<DbBackedOnChainConfig>>,
    consensus_network_interfaces: Option<ApplicationNetworkInterfaces<ConsensusMsg>>,
    consensus_notifier: ConsensusNotifier,
    consensus_to_mempool_sender: Sender<QuorumStoreRequest>,
    vtxn_pool: VTxnPoolState,
    consensus_publisher: Option<Arc<ConsensusPublisher>>,
    admin_service: &mut AdminService,
) -> Option<Runtime> {
    consensus_network_interfaces.map(|consensus_network_interfaces| {
        let (consensus_runtime, consensus_db, quorum_store_db) = services::start_consensus_runtime(
            node_config,
            db_rw.clone(),
            consensus_reconfig_subscription,
            consensus_network_interfaces,
            consensus_notifier.clone(),
            consensus_to_mempool_sender.clone(),
            vtxn_pool,
            consensus_publisher.clone(),
        );
        admin_service.set_consensus_dbs(consensus_db, quorum_store_db);

        consensus_runtime
    })
}

/// Creates and starts the DKG runtime (if enabled)
pub fn create_dkg_runtime(
    node_config: &mut NodeConfig,
    dkg_subscriptions: Option<(
        ReconfigNotificationListener<DbBackedOnChainConfig>,
        EventNotificationListener,
    )>,
    dkg_network_interfaces: Option<ApplicationNetworkInterfaces<DKGMessage>>,
) -> (VTxnPoolState, Option<Runtime>) {
    let vtxn_pool = VTxnPoolState::default();
    let dkg_runtime = match dkg_network_interfaces {
        Some(interfaces) => {
            let ApplicationNetworkInterfaces {
                network_client,
                network_service_events,
            } = interfaces;
            let (reconfig_events, dkg_start_events) = dkg_subscriptions
                .expect("DKG needs to listen to NewEpochEvents events and DKGStartEvents");
            let my_addr = node_config.validator_network.as_ref().unwrap().peer_id();
            let rb_config = node_config.consensus.rand_rb_config.clone();
            let dkg_runtime = start_dkg_runtime(
                my_addr,
                &node_config.consensus.safety_rules,
                network_client,
                network_service_events,
                reconfig_events,
                dkg_start_events,
                vtxn_pool.clone(),
                rb_config,
                node_config.randomness_override_seq_num,
            );
            Some(dkg_runtime)
        },
        _ => None,
    };

    (vtxn_pool, dkg_runtime)
}

/// Creates and starts the JWK consensus runtime (if enabled)
pub fn create_jwk_consensus_runtime(
    node_config: &mut NodeConfig,
    jwk_consensus_subscriptions: Option<(
        ReconfigNotificationListener<DbBackedOnChainConfig>,
        EventNotificationListener,
    )>,
    jwk_consensus_network_interfaces: Option<ApplicationNetworkInterfaces<JWKConsensusMsg>>,
    vtxn_pool: &VTxnPoolState,
) -> Option<Runtime> {
    let jwk_consensus_runtime = match jwk_consensus_network_interfaces {
        Some(interfaces) => {
            let ApplicationNetworkInterfaces {
                network_client,
                network_service_events,
            } = interfaces;
            let (reconfig_events, onchain_jwk_updated_events) = jwk_consensus_subscriptions.expect(
                "JWK consensus needs to listen to NewEpochEvents and OnChainJWKMapUpdated events.",
            );
            let my_addr = node_config.validator_network.as_ref().unwrap().peer_id();
            let jwk_consensus_runtime = start_jwk_consensus_runtime(
                my_addr,
                &node_config.consensus.safety_rules,
                network_client,
                network_service_events,
                reconfig_events,
                onchain_jwk_updated_events,
                vtxn_pool.clone(),
            );
            Some(jwk_consensus_runtime)
        },
        _ => None,
    };
    jwk_consensus_runtime
}

/// Creates and starts the consensus observer and publisher (if enabled)
pub fn create_consensus_observer_and_publisher(
    node_config: &NodeConfig,
    consensus_observer_interfaces: Option<ApplicationNetworkInterfaces<ConsensusObserverMessage>>,
    consensus_notifier: ConsensusNotifier,
    consensus_to_mempool_sender: Sender<QuorumStoreRequest>,
    db_rw: DbReaderWriter,
    consensus_observer_reconfig_subscription: Option<
        ReconfigNotificationListener<DbBackedOnChainConfig>,
    >,
) -> (
    Option<Runtime>,
    Option<Runtime>,
    Option<Arc<ConsensusPublisher>>,
) {
    // If none of the consensus observer or publisher are enabled, return early
    if !node_config
        .consensus_observer
        .is_observer_or_publisher_enabled()
    {
        return (None, None, None);
    }

    // Fetch the consensus observer network client and events
    let consensus_observer_interfaces = consensus_observer_interfaces
        .expect("Consensus observer is enabled, but the network interfaces are missing!");
    let consensus_observer_client = consensus_observer_interfaces.network_client;
    let consensus_observer_events = consensus_observer_interfaces.network_service_events;

    // Create the consensus observer client and network handler
    let consensus_observer_client =
        Arc::new(ConsensusObserverClient::new(consensus_observer_client));
    let (
        consensus_observer_runtime,
        consensus_observer_message_receiver,
        consensus_publisher_message_receiver,
    ) = create_observer_network_handler(node_config, consensus_observer_events);

    // Create the consensus publisher (if enabled)
    let (consensus_publisher_runtime, consensus_publisher) = create_consensus_publisher(
        node_config,
        consensus_observer_client.clone(),
        consensus_publisher_message_receiver,
    );

    // Create the consensus observer (if enabled)
    create_consensus_observer(
        node_config,
        &consensus_observer_runtime,
        consensus_observer_client,
        consensus_observer_message_receiver,
        consensus_publisher.clone(),
        consensus_notifier,
        consensus_to_mempool_sender,
        db_rw,
        consensus_observer_reconfig_subscription,
    );

    (
        Some(consensus_observer_runtime),
        consensus_publisher_runtime,
        consensus_publisher,
    )
}

/// Creates and starts the consensus observer (if enabled)
fn create_consensus_observer(
    node_config: &NodeConfig,
    consensus_observer_runtime: &Runtime,
    consensus_observer_client: Arc<
        ConsensusObserverClient<NetworkClient<ConsensusObserverMessage>>,
    >,
    consensus_observer_message_receiver: Receiver<(), ConsensusObserverNetworkMessage>,
    consensus_publisher: Option<Arc<ConsensusPublisher>>,
    state_sync_notifier: ConsensusNotifier,
    consensus_to_mempool_sender: Sender<QuorumStoreRequest>,
    db_rw: DbReaderWriter,
    observer_reconfig_subscription: Option<ReconfigNotificationListener<DbBackedOnChainConfig>>,
) {
    // If the observer is not enabled, return early
    if !node_config.consensus_observer.observer_enabled {
        return;
    }

    // Create the consensus observer
    start_consensus_observer(
        node_config,
        consensus_observer_runtime,
        consensus_observer_client,
        consensus_observer_message_receiver,
        consensus_publisher,
        Arc::new(state_sync_notifier),
        consensus_to_mempool_sender,
        db_rw,
        observer_reconfig_subscription,
    );
}

/// Creates and returns the consensus publisher and runtime (if enabled)
fn create_consensus_publisher(
    node_config: &NodeConfig,
    consensus_observer_client: Arc<
        ConsensusObserverClient<NetworkClient<ConsensusObserverMessage>>,
    >,
    publisher_message_receiver: Receiver<(), ConsensusPublisherNetworkMessage>,
) -> (Option<Runtime>, Option<Arc<ConsensusPublisher>>) {
    // If the publisher is not enabled, return early
    if !node_config.consensus_observer.publisher_enabled {
        return (None, None);
    }

    // Create the publisher runtime
    let runtime = aptos_runtimes::spawn_named_runtime("publisher".into(), None);

    // Create the consensus publisher
    let (consensus_publisher, outbound_message_receiver) =
        ConsensusPublisher::new(node_config.consensus_observer, consensus_observer_client);

    // Start the consensus publisher
    runtime.spawn(
        consensus_publisher
            .clone()
            .start(outbound_message_receiver, publisher_message_receiver),
    );

    // Return the runtime and publisher
    (Some(runtime), Some(Arc::new(consensus_publisher)))
}

/// Creates the consensus observer network handler, and returns the observer
/// runtime, observer message receiver, and publisher message receiver.
fn create_observer_network_handler(
    node_config: &NodeConfig,
    consensus_observer_events: NetworkServiceEvents<ConsensusObserverMessage>,
) -> (
    Runtime,
    Receiver<(), ConsensusObserverNetworkMessage>,
    Receiver<(), ConsensusPublisherNetworkMessage>,
) {
    // Create the consensus observer runtime
    let consensus_observer_runtime = aptos_runtimes::spawn_named_runtime("observer".into(), None);

    // Create the consensus observer network events
    let consensus_observer_events = ConsensusObserverNetworkEvents::new(consensus_observer_events);

    // Create the consensus observer network handler
    let (
        consensus_observer_network_handler,
        consensus_observer_message_receiver,
        consensus_publisher_message_receiver,
    ) = ConsensusObserverNetworkHandler::new(
        node_config.consensus_observer,
        consensus_observer_events,
    );

    // Start the consensus observer network handler
    consensus_observer_runtime.spawn(consensus_observer_network_handler.start());

    (
        consensus_observer_runtime,
        consensus_observer_message_receiver,
        consensus_publisher_message_receiver,
    )
}
