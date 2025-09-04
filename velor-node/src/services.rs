// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{bootstrap_api, indexer, mpsc::Receiver, network::ApplicationNetworkInterfaces};
use velor_admin_service::AdminService;
use velor_build_info::build_information;
use velor_config::config::NodeConfig;
use velor_consensus::{
    consensus_observer::publisher::consensus_publisher::ConsensusPublisher,
    network_interface::ConsensusMsg, persistent_liveness_storage::StorageWriteProxy,
    quorum_store::quorum_store_db::QuorumStoreDB,
};
use velor_consensus_notifications::ConsensusNotifier;
use velor_data_client::client::VelorDataClient;
use velor_db_indexer::{db_indexer::InternalIndexerDB, indexer_reader::IndexerReaders};
use velor_event_notifications::{DbBackedOnChainConfig, ReconfigNotificationListener};
use velor_indexer_grpc_fullnode::runtime::bootstrap as bootstrap_indexer_grpc;
use velor_indexer_grpc_table_info::runtime::{
    bootstrap as bootstrap_indexer_table_info, bootstrap_internal_indexer_db,
};
use velor_logger::{debug, telemetry_log_writer::TelemetryLog, LoggerFilterUpdater};
use velor_mempool::{
    network::MempoolSyncMsg, MempoolClientRequest, MempoolClientSender, QuorumStoreRequest,
};
use velor_mempool_notifications::MempoolNotificationListener;
use velor_network::application::{interface::NetworkClientInterface, storage::PeersAndMetadata};
use velor_network_benchmark::{run_netbench_service, NetbenchMessage};
use velor_peer_monitoring_service_server::{
    network::PeerMonitoringServiceNetworkEvents, storage::StorageReader,
    PeerMonitoringServiceServer,
};
use velor_peer_monitoring_service_types::PeerMonitoringServiceMessage;
use velor_storage_interface::{DbReader, DbReaderWriter};
use velor_time_service::TimeService;
use velor_types::{
    chain_id::ChainId, indexer::indexer_db_reader::IndexerReader, transaction::Version,
};
use velor_validator_transaction_pool::VTxnPoolState;
use futures::channel::{mpsc, mpsc::Sender, oneshot};
use std::{sync::Arc, time::Instant};
use tokio::{
    runtime::{Handle, Runtime},
    sync::watch::Receiver as WatchReceiver,
};

const AC_SMP_CHANNEL_BUFFER_SIZE: usize = 1_024;
const INTRA_NODE_CHANNEL_BUFFER_SIZE: usize = 1;

/// Bootstraps the API and the indexer. Returns the Mempool client
/// receiver, and both the api and indexer runtimes.
pub fn bootstrap_api_and_indexer(
    node_config: &NodeConfig,
    db_rw: DbReaderWriter,
    chain_id: ChainId,
    internal_indexer_db: Option<InternalIndexerDB>,
    update_receiver: Option<WatchReceiver<(Instant, Version)>>,
    api_port_tx: Option<oneshot::Sender<u16>>,
    indexer_grpc_port_tx: Option<oneshot::Sender<u16>>,
) -> anyhow::Result<(
    Receiver<MempoolClientRequest>,
    Option<Runtime>,
    Option<Runtime>,
    Option<Runtime>,
    Option<Runtime>,
    Option<Runtime>,
    MempoolClientSender,
)> {
    // Create the mempool client and sender
    let (mempool_client_sender, mempool_client_receiver) =
        mpsc::channel(AC_SMP_CHANNEL_BUFFER_SIZE);

    let (indexer_table_info_runtime, indexer_async_v2) = match bootstrap_indexer_table_info(
        node_config,
        chain_id,
        db_rw.clone(),
        mempool_client_sender.clone(),
    ) {
        Some((runtime, indexer_v2)) => (Some(runtime), Some(indexer_v2)),
        None => (None, None),
    };

    let (db_indexer_runtime, txn_event_reader) = match bootstrap_internal_indexer_db(
        node_config,
        db_rw.clone(),
        internal_indexer_db,
        update_receiver,
    ) {
        Some((runtime, db_indexer)) => (Some(runtime), Some(db_indexer)),
        None => (None, None),
    };

    let indexer_readers = IndexerReaders::new(indexer_async_v2, txn_event_reader);

    // Create the API runtime
    let indexer_reader: Option<Arc<dyn IndexerReader>> = indexer_readers.map(|readers| {
        let trait_object: Arc<dyn IndexerReader> = Arc::new(readers);
        trait_object
    });

    let api_runtime = if node_config.api.enabled {
        Some(bootstrap_api(
            node_config,
            chain_id,
            db_rw.reader.clone(),
            mempool_client_sender.clone(),
            indexer_reader.clone(),
            api_port_tx,
        )?)
    } else {
        None
    };

    // Creates the indexer grpc runtime
    let indexer_grpc = bootstrap_indexer_grpc(
        node_config,
        chain_id,
        db_rw.reader.clone(),
        mempool_client_sender.clone(),
        indexer_reader,
        indexer_grpc_port_tx,
    );

    // Create the indexer runtime
    let indexer_runtime = indexer::bootstrap_indexer(
        node_config,
        chain_id,
        db_rw.reader.clone(),
        mempool_client_sender.clone(),
    )?;

    Ok((
        mempool_client_receiver,
        api_runtime,
        indexer_table_info_runtime,
        indexer_runtime,
        indexer_grpc,
        db_indexer_runtime,
        mempool_client_sender,
    ))
}

/// Starts consensus and returns the runtime
pub fn start_consensus_runtime(
    node_config: &NodeConfig,
    db_rw: DbReaderWriter,
    consensus_reconfig_subscription: Option<ReconfigNotificationListener<DbBackedOnChainConfig>>,
    consensus_network_interfaces: ApplicationNetworkInterfaces<ConsensusMsg>,
    consensus_notifier: ConsensusNotifier,
    consensus_to_mempool_sender: Sender<QuorumStoreRequest>,
    vtxn_pool: VTxnPoolState,
    consensus_publisher: Option<Arc<ConsensusPublisher>>,
) -> (Runtime, Arc<StorageWriteProxy>, Arc<QuorumStoreDB>) {
    let instant = Instant::now();

    let reconfig_subscription = consensus_reconfig_subscription
        .expect("Consensus requires a reconfiguration subscription!");

    let consensus = velor_consensus::consensus_provider::start_consensus(
        node_config,
        consensus_network_interfaces.network_client,
        consensus_network_interfaces.network_service_events,
        Arc::new(consensus_notifier),
        consensus_to_mempool_sender,
        db_rw,
        reconfig_subscription,
        vtxn_pool,
        consensus_publisher,
    );
    debug!("Consensus started in {} ms", instant.elapsed().as_millis());

    consensus
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
    let mempool = velor_mempool::bootstrap(
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

/// Spawns a new thread for the admin service
pub fn start_admin_service(node_config: &NodeConfig) -> AdminService {
    AdminService::new(node_config)
}

/// Spawns a new thread for the node inspection service
pub fn start_node_inspection_service(
    node_config: &NodeConfig,
    velor_data_client: VelorDataClient,
    peers_and_metadata: Arc<PeersAndMetadata>,
) {
    velor_inspection_service::start_inspection_service(
        node_config.clone(),
        velor_data_client,
        peers_and_metadata,
    )
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
        velor_runtimes::spawn_named_runtime("peer-mon".into(), None);

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
            velor_peer_monitoring_service_client::start_peer_monitor(
                node_config.clone(),
                network_client,
                Some(peer_monitoring_service_runtime.handle().clone()),
            ),
        );
    }

    // Return the runtime
    peer_monitoring_service_runtime
}

pub fn start_netbench_service(
    node_config: &NodeConfig,
    network_interfaces: ApplicationNetworkInterfaces<NetbenchMessage>,
    runtime: &Handle,
) {
    let network_client = network_interfaces.network_client;
    runtime.spawn(run_netbench_service(
        node_config.clone(),
        network_client,
        network_interfaces.network_service_events,
        TimeService::real(),
    ));
}

/// Starts the telemetry service and grabs the build information
pub fn start_telemetry_service(
    node_config: &NodeConfig,
    remote_log_rx: Option<Receiver<TelemetryLog>>,
    logger_filter_update_job: Option<LoggerFilterUpdater>,
    chain_id: ChainId,
) -> Option<Runtime> {
    let build_info = build_information!();
    velor_telemetry::service::start_telemetry_service(
        node_config.clone(),
        chain_id,
        build_info,
        remote_log_rx,
        logger_filter_update_job,
    )
}
