// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

//! Processes that are directly spawned by shared mempool runtime initialization
use crate::{
    core_mempool::{CoreMempool, TimelineState},
    counters,
    logging::{LogEntry, LogEvent, LogSchema},
    network::{MempoolNetworkEvents, MempoolSyncMsg},
    shared_mempool::{
        tasks,
        tasks::process_committed_transactions,
        types::{notify_subscribers, ScheduledBroadcast, SharedMempool, SharedMempoolNotification},
    },
    ConsensusRequest, MempoolEventsReceiver, TransactionSummary,
};
use ::network::protocols::network::Event;
use bounded_executor::BoundedExecutor;
use diem_config::network_id::{NetworkId, PeerNetworkId};
use diem_infallible::Mutex;
use diem_logger::prelude::*;
use diem_types::on_chain_config::OnChainConfigPayload;
use event_notifications::ReconfigNotificationListener;
use futures::{
    channel::mpsc,
    stream::{select_all, FuturesUnordered},
    StreamExt,
};
use mempool_notifications::{MempoolCommitNotification, MempoolNotificationListener};
use std::{
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};
use tokio::{runtime::Handle, time::interval};
use tokio_stream::wrappers::IntervalStream;
use vm_validator::vm_validator::TransactionValidation;

use super::types::MempoolClientRequest;

/// Coordinator that handles inbound network events and outbound txn broadcasts.
pub(crate) async fn coordinator<V>(
    mut smp: SharedMempool<V>,
    executor: Handle,
    network_events: Vec<(NetworkId, MempoolNetworkEvents)>,
    mut client_events: MempoolEventsReceiver,
    mut consensus_requests: mpsc::Receiver<ConsensusRequest>,
    mut mempool_listener: MempoolNotificationListener,
    mut mempool_reconfig_events: ReconfigNotificationListener,
) where
    V: TransactionValidation,
{
    info!(LogSchema::event_log(
        LogEntry::CoordinatorRuntime,
        LogEvent::Start
    ));
    // Combine `NetworkEvents` for each `NetworkId` into one stream
    let smp_events: Vec<_> = network_events
        .into_iter()
        .map(|(network_id, events)| events.map(move |e| (network_id, e)))
        .collect();
    let mut events = select_all(smp_events).fuse();
    let mut scheduled_broadcasts = FuturesUnordered::new();

    // Use a BoundedExecutor to restrict only `workers_available` concurrent
    // worker tasks that can process incoming transactions.
    let workers_available = smp.config.shared_mempool_max_concurrent_inbound_syncs;
    let bounded_executor = BoundedExecutor::new(workers_available, executor.clone());

    loop {
        let _timer = counters::MAIN_LOOP.start_timer();
        ::futures::select! {
            msg = client_events.select_next_some() => {
                handle_client_request(&mut smp, &bounded_executor, msg).await;
            },
            msg = consensus_requests.select_next_some() => {
                tasks::process_consensus_request(&smp.mempool, msg);
            },
            msg = mempool_listener.select_next_some() => {
                handle_commit_notification(&mut smp, msg, &mut mempool_listener);
            },
            reconfig_notification = mempool_reconfig_events.select_next_some() => {
                handle_mempool_reconfig_event(&mut smp, &bounded_executor, reconfig_notification.on_chain_configs).await;
            },
            (peer, backoff) = scheduled_broadcasts.select_next_some() => {
                tasks::execute_broadcast(peer, backoff, &mut smp, &mut scheduled_broadcasts, executor.clone());
            },
            (network_id, event) = events.select_next_some() => {
                handle_network_event(&executor, &bounded_executor, &mut scheduled_broadcasts, &mut smp, network_id, event).await;
            },
            complete => break,
        }
    }
    error!(LogSchema::event_log(
        LogEntry::CoordinatorRuntime,
        LogEvent::Terminated
    ));
}

/// Spawn a task for processing `MempoolClientRequest`
async fn handle_client_request<V>(
    smp: &mut SharedMempool<V>,
    bounded_executor: &BoundedExecutor,
    request: MempoolClientRequest,
) where
    V: TransactionValidation,
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
            bounded_executor
                .spawn(tasks::process_client_transaction_submission(
                    smp.clone(),
                    txn,
                    callback,
                    task_start_timer,
                ))
                .await;
        }
        MempoolClientRequest::GetTransaction(hash, callback) => {
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
        }
    }
}

/// Handle removing committed transactions from local mempool immediately.  This should be done
/// immediately to ensure broadcasts of committed transactions stop as soon as possible.
fn handle_commit_notification<V>(
    smp: &mut SharedMempool<V>,
    msg: MempoolCommitNotification,
    mempool_listener: &mut MempoolNotificationListener,
) where
    V: TransactionValidation,
{
    debug!(
        LogSchema::event_log(LogEntry::StateSyncCommit, LogEvent::Received).state_sync_msg(&msg)
    );

    // Process and time committed user transactions.
    let start_time = Instant::now();
    counters::mempool_service_transactions(
        counters::COMMIT_STATE_SYNC_LABEL,
        msg.transactions.len(),
    );
    process_committed_transactions(
        &smp.mempool.clone(),
        msg.transactions
            .iter()
            .map(|txn| TransactionSummary {
                sender: txn.sender,
                sequence_number: txn.sequence_number,
            })
            .collect(),
        msg.block_timestamp_usecs,
        false,
    );
    let counter_result = if mempool_listener.ack_commit_notification(msg).is_err() {
        error!(LogSchema::event_log(
            LogEntry::StateSyncCommit,
            LogEvent::CallbackFail
        ));
        counters::REQUEST_FAIL_LABEL
    } else {
        counters::REQUEST_SUCCESS_LABEL
    };
    let latency = start_time.elapsed();
    counters::mempool_service_latency(counters::COMMIT_STATE_SYNC_LABEL, counter_result, latency);
}

/// Spawn a task to restart the transaction validator with the new reconfig data.
async fn handle_mempool_reconfig_event<V>(
    smp: &mut SharedMempool<V>,
    bounded_executor: &BoundedExecutor,
    config_update: OnChainConfigPayload,
) where
    V: TransactionValidation,
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
        ))
        .await;
}

/// Handles all NewPeer, LostPeer, and network messages.
/// - NewPeer events start new automatic broadcasts if the peer is upstream. If the peer is not upstream, we ignore it.
/// - LostPeer events disable the upstream peer, which will cancel ongoing broadcasts.
/// - Network messages follow a simple Request/Response framework to accept new transactions
/// TODO: Move to RPC off of DirectSend
async fn handle_network_event<V>(
    executor: &Handle,
    bounded_executor: &BoundedExecutor,
    scheduled_broadcasts: &mut FuturesUnordered<ScheduledBroadcast>,
    smp: &mut SharedMempool<V>,
    network_id: NetworkId,
    event: Event<MempoolSyncMsg>,
) where
    V: TransactionValidation,
{
    match event {
        Event::NewPeer(metadata) => {
            counters::shared_mempool_event_inc("new_peer");
            let peer = PeerNetworkId::new(network_id, metadata.remote_peer_id);
            let is_new_peer = smp.network_interface.add_peer(peer, metadata.clone());
            let is_upstream_peer = smp
                .network_interface
                .is_upstream_peer(&peer, Some(&metadata));
            debug!(LogSchema::new(LogEntry::NewPeer)
                .peer(&peer)
                .is_upstream_peer(is_upstream_peer));
            notify_subscribers(SharedMempoolNotification::PeerStateChange, &smp.subscribers);
            if is_new_peer && is_upstream_peer {
                tasks::execute_broadcast(peer, false, smp, scheduled_broadcasts, executor.clone());
            }
        }
        Event::LostPeer(metadata) => {
            counters::shared_mempool_event_inc("lost_peer");
            let peer = PeerNetworkId::new(network_id, metadata.remote_peer_id);
            debug!(LogSchema::new(LogEntry::LostPeer)
                .peer(&peer)
                .is_upstream_peer(
                    smp.network_interface
                        .is_upstream_peer(&peer, Some(&metadata))
                ));
            smp.network_interface.disable_peer(peer);
            notify_subscribers(SharedMempoolNotification::PeerStateChange, &smp.subscribers);
        }
        Event::Message(peer_id, msg) => {
            counters::shared_mempool_event_inc("message");
            match msg {
                MempoolSyncMsg::BroadcastTransactionsRequest {
                    request_id,
                    transactions,
                } => {
                    let smp_clone = smp.clone();
                    let peer = PeerNetworkId::new(network_id, peer_id);
                    let timeline_state = match smp.network_interface.is_upstream_peer(&peer, None) {
                        true => TimelineState::NonQualified,
                        false => TimelineState::NotReady,
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
                            request_id,
                            timeline_state,
                            peer,
                            task_start_timer,
                        ))
                        .await;
                }
                MempoolSyncMsg::BroadcastTransactionsResponse {
                    request_id,
                    retry,
                    backoff,
                } => {
                    let ack_timestamp = SystemTime::now();
                    smp.network_interface.process_broadcast_ack(
                        PeerNetworkId::new(network_id, peer_id),
                        request_id,
                        retry,
                        backoff,
                        ack_timestamp,
                    );
                }
            }
        }
        Event::RpcRequest(peer_id, _msg, _, _res_tx) => {
            counters::unexpected_msg_count_inc(&network_id, &peer_id);
            sample!(
                SampleRate::Duration(Duration::from_secs(60)),
                warn!(LogSchema::new(LogEntry::UnexpectedNetworkMsg)
                    .peer(&PeerNetworkId::new(network_id, peer_id)))
            );
        }
    }
}

/// Garbage collect all expired transactions by SystemTTL.
pub(crate) async fn gc_coordinator(mempool: Arc<Mutex<CoreMempool>>, gc_interval_ms: u64) {
    info!(LogSchema::event_log(LogEntry::GCRuntime, LogEvent::Start));
    let mut interval = IntervalStream::new(interval(Duration::from_millis(gc_interval_ms)));
    while let Some(_interval) = interval.next().await {
        sample!(
            SampleRate::Duration(Duration::from_secs(60)),
            info!(LogSchema::event_log(LogEntry::GCRuntime, LogEvent::Live))
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
        debug!(LogSchema::new(LogEntry::MempoolSnapshot).txns(snapshot));
    }
}
