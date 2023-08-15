// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{bootstrap_api, indexer, mpsc::Receiver, network::ApplicationNetworkInterfaces};
use aptos_build_info::build_information;
use aptos_config::config::NodeConfig;
use aptos_consensus::network_interface::ConsensusMsg;
use aptos_consensus_notifications::ConsensusNotifier;
use aptos_event_notifications::{DbBackedOnChainConfig, ReconfigNotificationListener};
use aptos_indexer_grpc_fullnode::runtime::bootstrap as bootstrap_indexer_grpc;
use aptos_logger::{debug, telemetry_log_writer::TelemetryLog, LoggerFilterUpdater};
use aptos_mempool::{network::MempoolSyncMsg, MempoolClientRequest, QuorumStoreRequest};
use aptos_mempool_notifications::MempoolNotificationListener;
use aptos_network::application::{interface::NetworkClientInterface, storage::PeersAndMetadata};
use aptos_peer_monitoring_service_server::{
    network::PeerMonitoringServiceNetworkEvents, storage::StorageReader,
    PeerMonitoringServiceServer,
};
use aptos_peer_monitoring_service_types::PeerMonitoringServiceMessage;
use aptos_storage_interface::{DbReader, DbReaderWriter};
use aptos_time_service::TimeService;
use aptos_types::chain_id::ChainId;
use futures::channel::{mpsc, mpsc::Sender};
use std::{sync::Arc, time::Instant};
use tokio::runtime::Runtime;

const AC_SMP_CHANNEL_BUFFER_SIZE: usize = 1_024;
const INTRA_NODE_CHANNEL_BUFFER_SIZE: usize = 1;

/// Bootstraps the API and the indexer. Returns the Mempool client
/// receiver, and both the api and indexer runtimes.
pub fn bootstrap_api_and_indexer(
    node_config: &NodeConfig,
    aptos_db: Arc<dyn DbReader>,
    chain_id: ChainId,
) -> anyhow::Result<(
    Receiver<MempoolClientRequest>,
    Option<Runtime>,
    Option<Runtime>,
    Option<Runtime>,
)> {
    // Create the mempool client and sender
    let (mempool_client_sender, mempool_client_receiver) =
        mpsc::channel(AC_SMP_CHANNEL_BUFFER_SIZE);

    // Create the API runtime
    let api_runtime = if node_config.api.enabled {
        Some(bootstrap_api(
            node_config,
            chain_id,
            aptos_db.clone(),
            mempool_client_sender.clone(),
        )?)
    } else {
        None
    };

    // Creates the indexer grpc runtime
    let indexer_grpc = bootstrap_indexer_grpc(
        node_config,
        chain_id,
        aptos_db.clone(),
        mempool_client_sender.clone(),
    );

    // Create the indexer runtime
    let indexer_runtime =
        indexer::bootstrap_indexer(node_config, chain_id, aptos_db, mempool_client_sender)?;

    Ok((
        mempool_client_receiver,
        api_runtime,
        indexer_runtime,
        indexer_grpc,
    ))
}

/// Starts consensus and returns the runtime
pub fn start_consensus_runtime(
    node_config: &mut NodeConfig,
    db_rw: DbReaderWriter,
    consensus_reconfig_subscription: Option<ReconfigNotificationListener<DbBackedOnChainConfig>>,
    consensus_network_interfaces: ApplicationNetworkInterfaces<ConsensusMsg>,
    consensus_notifier: ConsensusNotifier,
    consensus_to_mempool_sender: Sender<QuorumStoreRequest>,
) -> Runtime {
    let instant = Instant::now();
    let consensus_runtime = aptos_consensus::consensus_provider::start_consensus(
        node_config,
        consensus_network_interfaces.network_client,
        consensus_network_interfaces.network_service_events,
        Arc::new(consensus_notifier),
        consensus_to_mempool_sender,
        db_rw,
        consensus_reconfig_subscription
            .expect("Consensus requires a reconfiguration subscription!"),
    );
    debug!("Consensus started in {} ms", instant.elapsed().as_millis());
    consensus_runtime
}

/// Create the mempool runtime and start mempool
pub fn start_mempool_runtime_and_get_consensus_sender(
    node_config: &mut NodeConfig,
    db_rw: &DbReaderWriter,
    mempool_reconfig_subscription: ReconfigNotificationListener<DbBackedOnChainConfig>,
    network_interfaces: ApplicationNetworkInterfaces<MempoolSyncMsg>,
    mempool_listener: MempoolNotificationListener,
    mempool_client_receiver: Receiver<MempoolClientRequest>,
    peers_and_metadata: Arc<PeersAndMetadata>,
) -> (Runtime, Sender<QuorumStoreRequest>) {
    // Create a communication channel between consensus and mempool
    let (consensus_to_mempool_sender, consensus_to_mempool_receiver) =
        mpsc::channel(INTRA_NODE_CHANNEL_BUFFER_SIZE);

    // Bootstrap and start mempool
    let instant = Instant::now();
    let mempool = aptos_mempool::bootstrap(
        node_config,
        Arc::clone(&db_rw.reader),
        network_interfaces.network_client,
        network_interfaces.network_service_events,
        mempool_client_receiver,
        consensus_to_mempool_receiver,
        mempool_listener,
        mempool_reconfig_subscription,
        peers_and_metadata,
    );
    debug!("Mempool started in {} ms", instant.elapsed().as_millis());

    (mempool, consensus_to_mempool_sender)
}

/// Spawns a new thread for the node inspection service
pub fn start_node_inspection_service(
    node_config: &NodeConfig,
    peers_and_metadata: Arc<PeersAndMetadata>,
) {
    aptos_inspection_service::start_inspection_service(node_config.clone(), peers_and_metadata)
}

/// Starts the peer monitoring service and returns the runtime
pub fn start_peer_monitoring_service(
    node_config: &NodeConfig,
    network_interfaces: ApplicationNetworkInterfaces<PeerMonitoringServiceMessage>,
    db_reader: Arc<dyn DbReader>,
) -> Runtime {
    // Get the network client and events
    let network_client = network_interfaces.network_client;
    let network_service_events = network_interfaces.network_service_events;

    // Create a new runtime for the monitoring service
    let peer_monitoring_service_runtime =
        aptos_runtimes::spawn_named_runtime("peer-mon".into(), None);

    // Create and spawn the peer monitoring server
    let peer_monitoring_network_events =
        PeerMonitoringServiceNetworkEvents::new(network_service_events);
    let peer_monitoring_server = PeerMonitoringServiceServer::new(
        node_config.clone(),
        peer_monitoring_service_runtime.handle().clone(),
        peer_monitoring_network_events,
        network_client.get_peers_and_metadata(),
        StorageReader::new(db_reader),
        TimeService::real(),
    );
    peer_monitoring_service_runtime.spawn(peer_monitoring_server.start());

    // Spawn the peer monitoring client
    if node_config
        .peer_monitoring_service
        .enable_peer_monitoring_client
    {
        peer_monitoring_service_runtime.spawn(
            aptos_peer_monitoring_service_client::start_peer_monitor(
                node_config.clone(),
                network_client,
                Some(peer_monitoring_service_runtime.handle().clone()),
            ),
        );
    }

    // Return the runtime
    peer_monitoring_service_runtime
}

/// Starts the telemetry service and grabs the build information
pub fn start_telemetry_service(
    node_config: &NodeConfig,
    remote_log_rx: Option<Receiver<TelemetryLog>>,
    logger_filter_update_job: Option<LoggerFilterUpdater>,
    chain_id: ChainId,
) -> Option<Runtime> {
    let build_info = build_information!();
    aptos_telemetry::service::start_telemetry_service(
        node_config.clone(),
        chain_id,
        build_info,
        remote_log_rx,
        logger_filter_update_job,
    )
}
