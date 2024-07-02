// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{network::ApplicationNetworkInterfaces, services};
use aptos_admin_service::AdminService;
use aptos_config::config::NodeConfig;
use aptos_consensus::{
    consensus_observer::{
        network_message::ConsensusObserverMessage, publisher::ConsensusPublisher,
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
use aptos_logger::debug;
use aptos_mempool::QuorumStoreRequest;
use aptos_safety_rules::safety_rules_manager::load_consensus_key_from_secure_storage;
use aptos_storage_interface::DbReaderWriter;
use aptos_validator_transaction_pool::VTxnPoolState;
use futures::channel::mpsc::Sender;
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Creates and returns the consensus observer runtime (if either the
/// observer or publisher is enabled).
pub fn create_consensus_observer_runtime(
    node_config: &NodeConfig,
    consensus_observer_network_interfaces: Option<
        ApplicationNetworkInterfaces<ConsensusObserverMessage>,
    >,
    consensus_publisher: Option<Arc<ConsensusPublisher>>,
    consensus_notifier: ConsensusNotifier,
    consensus_to_mempool_sender: Sender<QuorumStoreRequest>,
    db_rw: DbReaderWriter,
    consensus_observer_reconfig_subscription: Option<
        ReconfigNotificationListener<DbBackedOnChainConfig>,
    >,
) -> Option<Runtime> {
    if node_config
        .consensus_observer
        .is_observer_or_publisher_enabled()
    {
        // Fetch the network interfaces and reconfig subscription
        let consensus_observer_network_interfaces = consensus_observer_network_interfaces
            .expect("Consensus observer is enabled, but network interfaces are missing!");

        // Start the consensus observer runtime
        let consensus_observer_runtime = start_consensus_observer(
            node_config,
            consensus_observer_network_interfaces.network_client,
            consensus_observer_network_interfaces.network_service_events,
            consensus_publisher,
            Arc::new(consensus_notifier),
            consensus_to_mempool_sender,
            db_rw,
            consensus_observer_reconfig_subscription,
        );
        Some(consensus_observer_runtime)
    } else {
        None
    }
}

/// Creates and returns the consensus publisher and runtime (if enabled)
pub fn create_consensus_publisher(
    node_config: &NodeConfig,
    consensus_observer_network_interfaces: &Option<
        ApplicationNetworkInterfaces<ConsensusObserverMessage>,
    >,
) -> (Option<Runtime>, Option<Arc<ConsensusPublisher>>) {
    if node_config.consensus_observer.publisher_enabled {
        // Get the network interfaces
        let consensus_observer_network_interfaces = consensus_observer_network_interfaces
            .as_ref()
            .expect("Consensus publisher is enabled, but network interfaces are missing!");

        // Create the publisher runtime
        let runtime = aptos_runtimes::spawn_named_runtime("publisher".into(), None);

        // Create the consensus publisher
        let (consensus_publisher, outbound_message_receiver) = ConsensusPublisher::new(
            consensus_observer_network_interfaces.network_client.clone(),
            node_config.consensus_observer,
        );

        // Start the consensus publisher
        runtime.spawn(consensus_publisher.clone().start(outbound_message_receiver));

        // Return the runtime and publisher
        (Some(runtime), Some(Arc::new(consensus_publisher)))
    } else {
        (None, None)
    }
}

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
    let maybe_dkg_dealer_sk =
        load_consensus_key_from_secure_storage(&node_config.consensus.safety_rules);
    debug!("maybe_dkg_dealer_sk={:?}", maybe_dkg_dealer_sk);

    let vtxn_pool = VTxnPoolState::default();
    let dkg_runtime = match (dkg_network_interfaces, maybe_dkg_dealer_sk) {
        (Some(interfaces), Ok(dkg_dealer_sk)) => {
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
                dkg_dealer_sk,
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
    let maybe_jwk_consensus_key =
        load_consensus_key_from_secure_storage(&node_config.consensus.safety_rules);
    debug!(
        "jwk_consensus_key_err={:?}",
        maybe_jwk_consensus_key.as_ref().err()
    );

    let jwk_consensus_runtime = match (jwk_consensus_network_interfaces, maybe_jwk_consensus_key) {
        (Some(interfaces), Ok(consensus_key)) => {
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
                consensus_key,
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
