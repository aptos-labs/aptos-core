// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    core_mempool::CoreMempool,
    network::{MempoolNetworkEvents, MempoolNetworkSender},
    shared_mempool::{
        coordinator::{coordinator, gc_coordinator, snapshot_job},
        types::{MempoolEventsReceiver, SharedMempool, SharedMempoolNotification},
    },
    QuorumStoreRequest,
};
use aptos_config::{config::NodeConfig, network_id::NetworkId};
use aptos_infallible::{Mutex, RwLock};

use aptos_logger::Level;
use event_notifications::ReconfigNotificationListener;
use futures::channel::mpsc::{self, Receiver, UnboundedSender};
use mempool_notifications::MempoolNotificationListener;
use network::application::storage::PeerMetadataStorage;
use rand::distributions::{Alphanumeric, DistString};
use std::io::Write;
use std::time::Duration;
use std::{collections::HashMap, sync::Arc};
use storage_interface::DbReader;
use tokio::runtime::{Builder, Handle, Runtime};
use vm_validator::vm_validator::{TransactionValidation, VMValidator};

/// Bootstrap of SharedMempool.
/// Creates a separate Tokio Runtime that runs the following routines:
///   - outbound_sync_task (task that periodically broadcasts transactions to peers).
///   - inbound_network_task (task that handles inbound mempool messages and network events).
///   - gc_task (task that performs GC of all expired transactions by SystemTTL).
pub(crate) fn start_shared_mempool<V>(
    executor: &Handle,
    config: &NodeConfig,
    mempool: Arc<Mutex<CoreMempool>>,
    // First element in tuple is the network ID.
    // See `NodeConfig::is_upstream_peer` for the definition of network ID.
    mempool_network_handles: Vec<(NetworkId, MempoolNetworkSender, MempoolNetworkEvents)>,
    client_events: MempoolEventsReceiver,
    quorum_store_requests: mpsc::Receiver<QuorumStoreRequest>,
    mempool_listener: MempoolNotificationListener,
    mempool_reconfig_events: ReconfigNotificationListener,
    db: Arc<dyn DbReader>,
    validator: Arc<RwLock<V>>,
    subscribers: Vec<UnboundedSender<SharedMempoolNotification>>,
    peer_metadata_storage: Arc<PeerMetadataStorage>,
) where
    V: TransactionValidation + 'static,
{
    let mut all_network_events = vec![];
    let mut network_senders = HashMap::new();
    for (network_id, network_sender, network_events) in mempool_network_handles.into_iter() {
        all_network_events.push((network_id, network_events));
        network_senders.insert(network_id, network_sender);
    }

    let smp = SharedMempool::new(
        mempool.clone(),
        config.mempool.clone(),
        network_senders,
        db,
        validator,
        subscribers,
        config.base.role,
        peer_metadata_storage,
    );

    let handle_clone = executor.clone();

    let system_transaction_gc_interval_ms = config.mempool.system_transaction_gc_interval_ms;
    let mempool_snapshot_interval_secs = config.mempool.mempool_snapshot_interval_secs;

    if aptos_logger::enabled!(Level::Trace) {
        executor.spawn(snapshot_job(
            mempool,
            config.mempool.mempool_snapshot_interval_secs,
        ));
    }

    executor.spawn(async move {
        let coordinator_monitor = tokio_metrics::TaskMonitor::new();
        let gc_monitor = tokio_metrics::TaskMonitor::new();
        let snapshot_monitor = tokio_metrics::TaskMonitor::new();

        tokio::task::Builder::new()
            .name("[Mempool] Coordinator Job")
            .spawn(coordinator_monitor.instrument(coordinator(
                smp,
                handle_clone.clone(),
                all_network_events,
                client_events,
                quorum_store_requests,
                mempool_listener,
                mempool_reconfig_events,
            )));

        tokio::task::Builder::new()
            .name("[Mempool] GC Job")
            .spawn(gc_monitor.instrument(gc_coordinator(
                mempool.clone(),
                system_transaction_gc_interval_ms,
            )));

        tokio::task::Builder::new()
            .name("[Mempool] Snapshot Job")
            .spawn(
                snapshot_monitor.instrument(snapshot_job(mempool, mempool_snapshot_interval_secs)),
            );

        let runtime_monitor = tokio_metrics::RuntimeMonitor::new(&handle_clone);
        let string = Alphanumeric.sample_string(&mut rand::thread_rng(), 8);
        let str_clone = string.clone();

        tokio::task::Builder::new()
            .name("Mempool Task Metrics Reporter")
            .spawn(async move {
                let coodrinator_intervals = coordinator_monitor.intervals();
                let gc_intervals = gc_monitor.intervals();
                let snapshot_intyervals = snapshot_monitor.intervals();

                let intervals = coodrinator_intervals.zip(gc_intervals.zip(snapshot_intyervals));

                let file_appender =
                    tracing_appender::rolling::minutely("/tmp", format!("{}.task_metrics", string));
                let (mut non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

                for (coordinator, (gc, snapshot)) in intervals {
                    non_blocking
                        .write_fmt(format_args!("coordinator = {:#?}\n", coordinator))
                        .expect("write_fmt should work");
                    non_blocking
                        .write_fmt(format_args!("gc = {:#?}\n", gc))
                        .expect("write_fmt should work");
                    non_blocking
                        .write_fmt(format_args!("snapshot = {:#?}\n", snapshot))
                        .expect("write_fmt should work");
                    tokio::time::sleep(Duration::from_millis(2000)).await;
                }
            });

        tokio::task::Builder::new()
            .name("Mempool Runtime Metrics Reporter")
            .spawn(async move {
                let file_appender = tracing_appender::rolling::minutely(
                    "/tmp",
                    format!("{}.runtime_metrics", str_clone),
                );
                let (mut non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

                for interval in runtime_monitor.intervals() {
                    // pretty-print the metric interval
                    non_blocking
                        .write_fmt(format_args!("{:?}\n", interval))
                        .expect("write_fmt should work");
                    // wait 2s
                    tokio::time::sleep(Duration::from_millis(2000)).await;
                }
            });
    });
}

pub fn bootstrap(
    config: &NodeConfig,
    db: Arc<dyn DbReader>,
    // The first element in the tuple is the ID of the network that this network is a handle to.
    // See `NodeConfig::is_upstream_peer` for the definition of network ID.
    mempool_network_handles: Vec<(NetworkId, MempoolNetworkSender, MempoolNetworkEvents)>,
    client_events: MempoolEventsReceiver,
    quorum_store_requests: Receiver<QuorumStoreRequest>,
    mempool_listener: MempoolNotificationListener,
    mempool_reconfig_events: ReconfigNotificationListener,
    peer_metadata_storage: Arc<PeerMetadataStorage>,
) -> Runtime {
    let runtime = Builder::new_multi_thread()
        .thread_name("shared-mem")
        .enable_all()
        .build()
        .expect("[shared mempool] failed to create runtime");
    let mempool = Arc::new(Mutex::new(CoreMempool::new(config)));
    let vm_validator = Arc::new(RwLock::new(VMValidator::new(Arc::clone(&db))));
    start_shared_mempool(
        runtime.handle(),
        config,
        mempool,
        mempool_network_handles,
        client_events,
        quorum_store_requests,
        mempool_listener,
        mempool_reconfig_events,
        db,
        vm_validator,
        vec![],
        peer_metadata_storage,
    );
    runtime
}
