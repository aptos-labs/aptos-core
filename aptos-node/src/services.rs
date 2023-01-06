// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{bootstrap_api, indexer, mpsc::Receiver, ApplicationNetworkHandle};
use aptos_build_info::build_information;
use aptos_config::config::NodeConfig;
use aptos_consensus::network_interface::ConsensusMsg;
use aptos_consensus_notifications::ConsensusNotifier;
use aptos_event_notifications::ReconfigNotificationListener;
use aptos_logger::{debug, telemetry_log_writer::TelemetryLog, LoggerFilterUpdater};
use aptos_mempool::{network::MempoolSyncMsg, MempoolClientRequest, QuorumStoreRequest};
use aptos_mempool_notifications::MempoolNotificationListener;
use aptos_network::application::storage::PeerMetadataStorage;
use aptos_storage_interface::{DbReader, DbReaderWriter};
use aptos_types::chain_id::ChainId;
use futures::channel::{mpsc, mpsc::Sender};
use maplit::hashmap;
use std::{sync::Arc, thread, time::Instant};
use tokio::runtime::Runtime;

const AC_SMP_CHANNEL_BUFFER_SIZE: usize = 1_024;
const INTRA_NODE_CHANNEL_BUFFER_SIZE: usize = 1;

/// Bootstraps the API and the indexer. Returns the Mempool client
/// recevier, and both the api and indexer runtimes.
pub fn bootstrap_api_and_indexer(
    node_config: &NodeConfig,
    aptos_db: Arc<dyn DbReader>,
    chain_id: ChainId,
) -> anyhow::Result<(
    Receiver<MempoolClientRequest>,
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

    // Create the indexer runtime
    let indexer_runtime =
        indexer::bootstrap_indexer(node_config, chain_id, aptos_db, mempool_client_sender)?;

    Ok((mempool_client_receiver, api_runtime, indexer_runtime))
}

/// Starts consensus and returns the runtime
pub fn start_consensus_runtime(
    node_config: &mut NodeConfig,
    db_rw: DbReaderWriter,
    consensus_reconfig_subscription: Option<ReconfigNotificationListener>,
    peer_metadata_storage: Arc<PeerMetadataStorage>,
    consensus_network_handle: ApplicationNetworkHandle<ConsensusMsg>,
    consensus_notifier: ConsensusNotifier,
    consensus_to_mempool_sender: Sender<QuorumStoreRequest>,
) -> Runtime {
    let instant = Instant::now();
    let consensus_runtime = aptos_consensus::consensus_provider::start_consensus(
        node_config,
        hashmap! {consensus_network_handle.network_id => consensus_network_handle.network_sender},
        consensus_network_handle.network_events,
        Arc::new(consensus_notifier),
        consensus_to_mempool_sender,
        db_rw,
        consensus_reconfig_subscription
            .expect("Consensus requires a reconfiguration subscription!"),
        peer_metadata_storage,
    );
    debug!("Consensus started in {} ms", instant.elapsed().as_millis());
    consensus_runtime
}

/// Create the mempool runtime and start mempool
pub fn start_mempool_runtime_and_get_consensus_sender(
    node_config: &mut NodeConfig,
    db_rw: &DbReaderWriter,
    mempool_reconfig_subscription: ReconfigNotificationListener,
    peer_metadata_storage: &Arc<PeerMetadataStorage>,
    mempool_network_handles: Vec<ApplicationNetworkHandle<MempoolSyncMsg>>,
    mempool_listener: MempoolNotificationListener,
    mempool_client_receiver: Receiver<MempoolClientRequest>,
) -> (Runtime, Sender<QuorumStoreRequest>) {
    // Create a communication channel between consensus and mempool
    let (consensus_to_mempool_sender, consensus_to_mempool_receiver) =
        mpsc::channel(INTRA_NODE_CHANNEL_BUFFER_SIZE);

    // Destruct the mempool network handle.
    // TODO: the bootstrap method should be refactored to avoid using large tuples.
    let mut deconstructed_network_handles = vec![];
    for appplication_network_handle in mempool_network_handles {
        deconstructed_network_handles.push((
            appplication_network_handle.network_id,
            appplication_network_handle.network_sender,
            appplication_network_handle.network_events,
        ))
    }

    // Bootstrap and start mempool
    let instant = Instant::now();
    let mempool = aptos_mempool::bootstrap(
        node_config,
        Arc::clone(&db_rw.reader),
        deconstructed_network_handles,
        mempool_client_receiver,
        consensus_to_mempool_receiver,
        mempool_listener,
        mempool_reconfig_subscription,
        peer_metadata_storage.clone(),
    );
    debug!("Mempool started in {} ms", instant.elapsed().as_millis());

    (mempool, consensus_to_mempool_sender)
}

/// Spawns a new thread for the node inspection service
pub fn start_node_inspection_service(node_config: &NodeConfig) {
    let node_config = node_config.clone();
    thread::spawn(move || {
        aptos_inspection_service::inspection_service::start_inspection_service(node_config)
    });
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
