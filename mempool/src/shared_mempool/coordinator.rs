// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Processes that are directly spawned by shared mempool runtime initialization
use super::types::MempoolClientRequest;
use crate::{
    core_mempool::{CoreMempool, TimelineState},
    counters,
    logging::{LogEntry, LogEvent, LogSchema},
    network::{BroadcastPeerPriority, MempoolSyncMsg},
    shared_mempool::{
        tasks::{self, process_committed_transactions},
        types::{
            notify_subscribers, MempoolMessageId, ScheduledBroadcast, SharedMempool,
            SharedMempoolNotification,
        },
        use_case_history::UseCaseHistory,
    },
    MempoolEventsReceiver, QuorumStoreRequest,
};
use velor_bounded_executor::BoundedExecutor;
use velor_config::network_id::{NetworkId, PeerNetworkId};
use velor_event_notifications::ReconfigNotificationListener;
use velor_infallible::{Mutex, RwLock};
use velor_logger::prelude::*;
use velor_mempool_notifications::{MempoolCommitNotification, MempoolNotificationListener};
use velor_network::{
    application::{
        interface::{NetworkClientInterface, NetworkServiceEvents},
        storage::PeersAndMetadata,
    },
    protocols::network::Event,
};
use velor_types::{
    on_chain_config::{OnChainConfigPayload, OnChainConfigProvider},
    transaction::SignedTransaction,
    PeerId,
};
use velor_vm_validator::vm_validator::TransactionValidation;
use futures::{
    channel::mpsc,
    stream::{select_all, FuturesUnordered},
    FutureExt, StreamExt,
};
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant, SystemTime},
};
use tokio::{runtime::Handle, time::interval};
use tokio_stream::wrappers::IntervalStream;

/// Coordinator that handles inbound network events and outbound txn broadcasts.
pub(crate) async fn coordinator<NetworkClient, TransactionValidator, ConfigProvider>(
    mut smp: SharedMempool<NetworkClient, TransactionValidator>,
    executor: Handle,
    network_service_events: NetworkServiceEvents<MempoolSyncMsg>,
    mut client_events: MempoolEventsReceiver,
    mut quorum_store_requests: mpsc::Receiver<QuorumStoreRequest>,
    mempool_listener: MempoolNotificationListener,
    mut mempool_reconfig_events: ReconfigNotificationListener<ConfigProvider>,
    peer_update_interval_ms: u64,
    peers_and_metadata: Arc<PeersAndMetadata>,
) where
    NetworkClient: NetworkClientInterface<MempoolSyncMsg> + 'static,
    TransactionValidator: TransactionValidation + 'static,
    ConfigProvider: OnChainConfigProvider,
{
    info!(LogSchema::event_log(
        LogEntry::CoordinatorRuntime,
        LogEvent::Start
    ));

    // Transform events to also include the network id
    let network_events: Vec<_> = network_service_events
        .into_network_and_events()
        .into_iter()
        .map(|(network_id, events)| events.map(move |event| (network_id, event)))
        .collect();
    let mut events = select_all(network_events).fuse();
    let mut scheduled_broadcasts = FuturesUnordered::new();
    let mut update_peers_interval =
        tokio::time::interval(Duration::from_millis(peer_update_interval_ms));

    // Spawn a dedicated task to handle commit notifications from state sync
    spawn_commit_notification_handler(&smp, mempool_listener);

    // Use a BoundedExecutor to restrict only `workers_available` concurrent
    // worker tasks that can process incoming transactions.
    let workers_available = smp.config.shared_mempool_max_concurrent_inbound_syncs;
    let bounded_executor = BoundedExecutor::new(workers_available, executor.clone());

    let initial_reconfig = mempool_reconfig_events
        .next()
        .await
        .expect("Reconfig sender dropped, unable to start mempool");
    handle_mempool_reconfig_event(
        &mut smp,
        &bounded_executor,
        initial_reconfig.on_chain_configs,
    )
    .await;

    loop {
        let _timer = counters::MAIN_LOOP.start_timer();
        ::futures::select! {
            msg = client_events.select_next_some() => {
                handle_client_request(&mut smp, &bounded_executor, msg).await;
            },
            msg = quorum_store_requests.select_next_some() => {
                tasks::process_quorum_store_request(&smp, msg);
            },
            reconfig_notification = mempool_reconfig_events.select_next_some() => {
                handle_mempool_reconfig_event(&mut smp, &bounded_executor, reconfig_notification.on_chain_configs).await;
            },
            (peer, backoff) = scheduled_broadcasts.select_next_some() => {
                tasks::execute_broadcast(peer, backoff, &mut smp, &mut scheduled_broadcasts, executor.clone()).await;
            },
            (network_id, event) = events.select_next_some() => {
                handle_network_event(&bounded_executor, &mut smp, network_id, event).await;
            },
            _ = update_peers_interval.tick().fuse() => {
                handle_update_peers(peers_and_metadata.clone(), &mut smp, &mut scheduled_broadcasts, executor.clone()).await;
            },
            complete => break,
        }
    }
    error!(LogSchema::event_log(
        LogEntry::CoordinatorRuntime,
        LogEvent::Terminated
    ));
}

/// Spawn a task to handle commit notifications from state sync
fn spawn_commit_notification_handler<NetworkClient, TransactionValidator>(
    smp: &SharedMempool<NetworkClient, TransactionValidator>,
    mut mempool_listener: MempoolNotificationListener,
) where
    NetworkClient: NetworkClientInterface<MempoolSyncMsg> + 'static,
    TransactionValidator: TransactionValidation + 'static,
{
    let mempool = smp.mempool.clone();
    let mempool_validator = smp.validator.clone();
    let use_case_history = smp.use_case_history.clone();
    let num_committed_txns_received_since_peers_updated = smp
        .network_interface
        .num_committed_txns_received_since_peers_updated
        .clone();

    tokio::spawn(async move {
        while let Some(commit_notification) = mempool_listener.next().await {
            handle_commit_notification(
                &mempool,
                &mempool_validator,
                &use_case_history,
                commit_notification,
                &num_committed_txns_received_since_peers_updated,
            );
        }
    });
}

/// Spawn a task for processing `MempoolClientRequest`s from a client such as API service
async fn handle_client_request<NetworkClient, TransactionValidator>(
    smp: &mut SharedMempool<NetworkClient, TransactionValidator>,
    bounded_executor: &BoundedExecutor,
    request: MempoolClientRequest,
) where
    NetworkClient: NetworkClientInterface<MempoolSyncMsg> + 'static,
    TransactionValidator: TransactionValidation + 'static,
{
    match request {
        MempoolClientRequest::SubmitTransaction(txn, callback) => {
            // This timer measures how long it took for the bounded executor to *schedule* the
            // task.
            let _timer = counters::task_spawn_latency_timer(
                counters::CLIENT_EVENT_LABEL,
                counters::SPAWN_LABEL,
            );
            // This timer measures how long it took for the task to go from scheduled to started.
            let task_start_timer = counters::task_spawn_latency_timer(
                counters::CLIENT_EVENT_LABEL,
                counters::START_LABEL,
            );
            smp.network_interface
                .num_mempool_txns_received_since_peers_updated += 1;
            bounded_executor
                .spawn(tasks::process_client_transaction_submission(
                    smp.clone(),
                    txn,
                    callback,
                    task_start_timer,
                ))
                .await;
        },
        MempoolClientRequest::GetTransactionByHash(hash, callback) => {
            // This timer measures how long it took for the bounded executor to *schedule* the
            // task.
            let _timer = counters::task_spawn_latency_timer(
                counters::CLIENT_EVENT_GET_TXN_LABEL,
                counters::SPAWN_LABEL,
            );
            // This timer measures how long it took for the task to go from scheduled to started.
            let task_start_timer = counters::task_spawn_latency_timer(
                counters::CLIENT_EVENT_GET_TXN_LABEL,
                counters::START_LABEL,
            );
            bounded_executor
                .spawn(tasks::process_client_get_transaction(
                    smp.clone(),
                    hash,
                    callback,
                    task_start_timer,
                ))
                .await;
        },
        MempoolClientRequest::GetAddressesFromParkingLot(callback) => {
            bounded_executor
                .spawn(tasks::process_parking_lot_addresses(smp.clone(), callback))
                .await;
        },
    }
}

/// Handle removing committed transactions from local mempool immediately.  This should be done
/// immediately to ensure broadcasts of committed transactions stop as soon as possible.
fn handle_commit_notification<TransactionValidator>(
    mempool: &Arc<Mutex<CoreMempool>>,
    mempool_validator: &Arc<RwLock<TransactionValidator>>,
    use_case_history: &Arc<Mutex<UseCaseHistory>>,
    msg: MempoolCommitNotification,
    num_committed_txns_received_since_peers_updated: &Arc<AtomicU64>,
) where
    TransactionValidator: TransactionValidation,
{
    debug!(
        block_timestamp_usecs = msg.block_timestamp_usecs,
        num_committed_txns = msg.transactions.len(),
        LogSchema::event_log(LogEntry::StateSyncCommit, LogEvent::Received),
    );

    // Process and time committed user transactions.
    let start_time = Instant::now();
    counters::mempool_service_transactions(
        counters::COMMIT_STATE_SYNC_LABEL,
        msg.transactions.len(),
    );
    num_committed_txns_received_since_peers_updated
        .fetch_add(msg.transactions.len() as u64, Ordering::Relaxed);
    process_committed_transactions(
        mempool,
        use_case_history,
        msg.transactions,
        msg.block_timestamp_usecs,
    );
    mempool_validator.write().notify_commit();
    let latency = start_time.elapsed();
    counters::mempool_service_latency(
        counters::COMMIT_STATE_SYNC_LABEL,
        counters::REQUEST_SUCCESS_LABEL,
        latency,
    );
}

/// Spawn a task to restart the transaction validator with the new reconfig data.
async fn handle_mempool_reconfig_event<NetworkClient, TransactionValidator, ConfigProvider>(
    smp: &mut SharedMempool<NetworkClient, TransactionValidator>,
    bounded_executor: &BoundedExecutor,
    config_update: OnChainConfigPayload<ConfigProvider>,
) where
    NetworkClient: NetworkClientInterface<MempoolSyncMsg> + 'static,
    TransactionValidator: TransactionValidation + 'static,
    ConfigProvider: OnChainConfigProvider,
{
    info!(LogSchema::event_log(
        LogEntry::ReconfigUpdate,
        LogEvent::Received
    ));
    let _timer =
        counters::task_spawn_latency_timer(counters::RECONFIG_EVENT_LABEL, counters::SPAWN_LABEL);

    bounded_executor
        .spawn(tasks::process_config_update(
            config_update,
            smp.validator.clone(),
            smp.broadcast_within_validator_network.clone(),
        ))
        .await;
}

async fn process_received_txns<NetworkClient, TransactionValidator>(
    bounded_executor: &BoundedExecutor,
    smp: &mut SharedMempool<NetworkClient, TransactionValidator>,
    network_id: NetworkId,
    message_id: MempoolMessageId,
    transactions: Vec<(
        SignedTransaction,
        Option<u64>,
        Option<BroadcastPeerPriority>,
    )>,
    peer_id: PeerId,
) where
    NetworkClient: NetworkClientInterface<MempoolSyncMsg> + 'static,
    TransactionValidator: TransactionValidation + 'static,
{
    smp.network_interface
        .num_mempool_txns_received_since_peers_updated += transactions.len() as u64;
    let smp_clone = smp.clone();
    let peer = PeerNetworkId::new(network_id, peer_id);
    let ineligible_for_broadcast = (smp.network_interface.is_validator()
        && !smp.broadcast_within_validator_network())
        || smp.network_interface.is_upstream_peer(&peer, None);
    let timeline_state = if ineligible_for_broadcast {
        TimelineState::NonQualified
    } else {
        TimelineState::NotReady
    };
    // This timer measures how long it took for the bounded executor to
    // *schedule* the task.
    let _timer = counters::task_spawn_latency_timer(
        counters::PEER_BROADCAST_EVENT_LABEL,
        counters::SPAWN_LABEL,
    );
    // This timer measures how long it took for the task to go from scheduled
    // to started.
    let task_start_timer = counters::task_spawn_latency_timer(
        counters::PEER_BROADCAST_EVENT_LABEL,
        counters::START_LABEL,
    );
    bounded_executor
        .spawn(tasks::process_transaction_broadcast(
            smp_clone,
            transactions,
            message_id,
            timeline_state,
            peer,
            task_start_timer,
        ))
        .await;
}

/// Handles all network messages.
/// - Network messages follow a simple Request/Response framework to accept new transactions
/// TODO: Move to RPC off of DirectSend
async fn handle_network_event<NetworkClient, TransactionValidator>(
    bounded_executor: &BoundedExecutor,
    smp: &mut SharedMempool<NetworkClient, TransactionValidator>,
    network_id: NetworkId,
    event: Event<MempoolSyncMsg>,
) where
    NetworkClient: NetworkClientInterface<MempoolSyncMsg> + 'static,
    TransactionValidator: TransactionValidation + 'static,
{
    match event {
        Event::Message(peer_id, msg) => {
            counters::shared_mempool_event_inc("message");
            match msg {
                MempoolSyncMsg::BroadcastTransactionsRequest {
                    message_id,
                    transactions,
                } => {
                    process_received_txns(
                        bounded_executor,
                        smp,
                        network_id,
                        message_id,
                        transactions.into_iter().map(|t| (t, None, None)).collect(),
                        peer_id,
                    )
                    .await;
                },
                MempoolSyncMsg::BroadcastTransactionsRequestWithReadyTime {
                    message_id,
                    transactions,
                } => {
                    process_received_txns(
                        bounded_executor,
                        smp,
                        network_id,
                        message_id,
                        transactions
                            .into_iter()
                            .map(|t| (t.0, Some(t.1), Some(t.2)))
                            .collect(),
                        peer_id,
                    )
                    .await;
                },
                MempoolSyncMsg::BroadcastTransactionsResponse {
                    message_id,
                    retry,
                    backoff,
                } => {
                    let ack_timestamp = SystemTime::now();
                    smp.network_interface.process_broadcast_ack(
                        PeerNetworkId::new(network_id, peer_id),
                        message_id,
                        retry,
                        backoff,
                        ack_timestamp,
                    );
                },
            }
        },
        Event::RpcRequest(peer_id, _msg, _, _res_tx) => {
            counters::unexpected_msg_count_inc(&network_id);
            sample!(
                SampleRate::Duration(Duration::from_secs(60)),
                warn!(LogSchema::new(LogEntry::UnexpectedNetworkMsg)
                    .peer(&PeerNetworkId::new(network_id, peer_id)))
            );
        },
    }
}

async fn handle_update_peers<NetworkClient, TransactionValidator>(
    peers_and_metadata: Arc<PeersAndMetadata>,
    smp: &mut SharedMempool<NetworkClient, TransactionValidator>,
    scheduled_broadcasts: &mut FuturesUnordered<ScheduledBroadcast>,
    executor: Handle,
) where
    NetworkClient: NetworkClientInterface<MempoolSyncMsg> + 'static,
    TransactionValidator: TransactionValidation + 'static,
{
    if let Ok(connected_peers) = peers_and_metadata.get_connected_peers_and_metadata() {
        let (newly_added_upstream, disabled) = smp.network_interface.update_peers(&connected_peers);
        if !newly_added_upstream.is_empty() || !disabled.is_empty() {
            counters::shared_mempool_event_inc("peer_update");
            notify_subscribers(SharedMempoolNotification::PeerStateChange, &smp.subscribers);
        }
        for peer in &newly_added_upstream {
            debug!(LogSchema::new(LogEntry::NewPeer).peer(peer));
            tasks::execute_broadcast(*peer, false, smp, scheduled_broadcasts, executor.clone())
                .await;
        }
        for peer in &disabled {
            debug!(LogSchema::new(LogEntry::LostPeer).peer(peer));
        }
    }
}

/// Garbage collect all expired transactions by SystemTTL.
pub(crate) async fn gc_coordinator(mempool: Arc<Mutex<CoreMempool>>, gc_interval_ms: u64) {
    debug!(LogSchema::event_log(LogEntry::GCRuntime, LogEvent::Start));
    let mut interval = IntervalStream::new(interval(Duration::from_millis(gc_interval_ms)));
    while let Some(_interval) = interval.next().await {
        sample!(
            SampleRate::Duration(Duration::from_secs(60)),
            debug!(LogSchema::event_log(LogEntry::GCRuntime, LogEvent::Live))
        );
        mempool.lock().gc();
    }

    error!(LogSchema::event_log(
        LogEntry::GCRuntime,
        LogEvent::Terminated
    ));
}

/// Periodically logs a snapshot of transactions in core mempool.
/// In the future we may want an interactive way to directly query mempool's internal state.
/// For now, we will rely on this periodic snapshot to observe the internal state.
pub(crate) async fn snapshot_job(mempool: Arc<Mutex<CoreMempool>>, snapshot_interval_secs: u64) {
    let mut interval = IntervalStream::new(interval(Duration::from_secs(snapshot_interval_secs)));
    while let Some(_interval) = interval.next().await {
        let snapshot = mempool.lock().gen_snapshot();
        trace!(LogSchema::new(LogEntry::MempoolSnapshot).txns(snapshot));
    }
}
