// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    core_mempool::CoreMempool,
    network::MempoolSyncMsg,
    shared_mempool::{
        coordinator::{coordinator, gc_coordinator, snapshot_job},
        types::{MempoolEventsReceiver, SharedMempool, SharedMempoolNotification},
    },
    QuorumStoreRequest,
};
use velor_config::config::{NodeConfig, NodeType};
use velor_event_notifications::{DbBackedOnChainConfig, ReconfigNotificationListener};
use velor_infallible::{Mutex, RwLock};
use velor_logger::Level;
use velor_mempool_notifications::MempoolNotificationListener;
use velor_network::application::{
    interface::{NetworkClient, NetworkServiceEvents},
    storage::PeersAndMetadata,
};
use velor_storage_interface::DbReader;
use velor_types::on_chain_config::OnChainConfigProvider;
use velor_vm_validator::vm_validator::{PooledVMValidator, TransactionValidation};
use futures::channel::mpsc::{Receiver, UnboundedSender};
use std::sync::Arc;
use tokio::runtime::{Handle, Runtime};

/// Bootstrap of SharedMempool.
/// Creates a separate Tokio Runtime that runs the following routines:
///   - outbound_sync_task (task that periodically broadcasts transactions to peers).
///   - inbound_network_task (task that handles inbound mempool messages and network events).
///   - gc_task (task that performs GC of all expired transactions by SystemTTL).
pub(crate) fn start_shared_mempool<TransactionValidator, ConfigProvider>(
    executor: &Handle,
    config: &NodeConfig,
    mempool: Arc<Mutex<CoreMempool>>,
    network_client: NetworkClient<MempoolSyncMsg>,
    network_service_events: NetworkServiceEvents<MempoolSyncMsg>,
    client_events: MempoolEventsReceiver,
    quorum_store_requests: Receiver<QuorumStoreRequest>,
    mempool_listener: MempoolNotificationListener,
    mempool_reconfig_events: ReconfigNotificationListener<ConfigProvider>,
    db: Arc<dyn DbReader>,
    validator: Arc<RwLock<TransactionValidator>>,
    subscribers: Vec<UnboundedSender<SharedMempoolNotification>>,
    peers_and_metadata: Arc<PeersAndMetadata>,
) where
    TransactionValidator: TransactionValidation + 'static,
    ConfigProvider: OnChainConfigProvider,
{
    let node_type = NodeType::extract_from_config(config);
    let transaction_filter_config = config.transaction_filters.mempool_filter.clone();
    let smp: SharedMempool<NetworkClient<MempoolSyncMsg>, TransactionValidator> =
        SharedMempool::new(
            mempool.clone(),
            config.mempool.clone(),
            transaction_filter_config,
            network_client,
            db,
            validator,
            subscribers,
            node_type,
        );

    executor.spawn(coordinator(
        smp,
        executor.clone(),
        network_service_events,
        client_events,
        quorum_store_requests,
        mempool_listener,
        mempool_reconfig_events,
        config.mempool.shared_mempool_peer_update_interval_ms,
        peers_and_metadata,
    ));

    executor.spawn(gc_coordinator(
        mempool.clone(),
        config.mempool.system_transaction_gc_interval_ms,
    ));

    if velor_logger::enabled!(Level::Trace) {
        executor.spawn(snapshot_job(
            mempool,
            config.mempool.mempool_snapshot_interval_secs,
        ));
    }
}

pub fn bootstrap(
    config: &NodeConfig,
    db: Arc<dyn DbReader>,
    network_client: NetworkClient<MempoolSyncMsg>,
    network_service_events: NetworkServiceEvents<MempoolSyncMsg>,
    client_events: MempoolEventsReceiver,
    quorum_store_requests: Receiver<QuorumStoreRequest>,
    mempool_listener: MempoolNotificationListener,
    mempool_reconfig_events: ReconfigNotificationListener<DbBackedOnChainConfig>,
    peers_and_metadata: Arc<PeersAndMetadata>,
) -> Runtime {
    let runtime = velor_runtimes::spawn_named_runtime("shared-mem".into(), None);
    let mempool = Arc::new(Mutex::new(CoreMempool::new(config)));
    let vm_validator = Arc::new(RwLock::new(PooledVMValidator::new(
        Arc::clone(&db),
        num_cpus::get(),
    )));
    start_shared_mempool(
        runtime.handle(),
        config,
        mempool,
        network_client,
        network_service_events,
        client_events,
        quorum_store_requests,
        mempool_listener,
        mempool_reconfig_events,
        db,
        vm_validator,
        vec![],
        peers_and_metadata,
    );
    runtime
}
