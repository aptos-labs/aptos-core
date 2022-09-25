// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Tasks that are executed by coordinators (short-lived compared to coordinators)
use crate::{
    core_mempool::{CoreMempool, TimelineState, TxnPointer},
    counters,
    logging::{LogEntry, LogEvent, LogSchema},
    network::{BroadcastError, MempoolSyncMsg},
    shared_mempool::types::{
        notify_subscribers, BatchId, ScheduledBroadcast, SharedMempool, SharedMempoolNotification,
        SubmissionStatusBundle,
    },
    thread_pool::IO_POOL,
    QuorumStoreRequest, QuorumStoreResponse, SubmissionStatus,
};
use anyhow::Result;
use aptos_config::network_id::PeerNetworkId;
use aptos_crypto::HashValue;
use aptos_infallible::{Mutex, RwLock};
use aptos_logger::prelude::*;
use aptos_metrics_core::HistogramTimer;
use aptos_types::{
    mempool_status::{MempoolStatus, MempoolStatusCode},
    on_chain_config::OnChainConfigPayload,
    transaction::SignedTransaction,
    vm_status::DiscardedVMStatus,
};
use consensus_types::common::TransactionSummary;
use futures::{channel::oneshot, stream::FuturesUnordered};
use network::application::interface::NetworkInterface;
use rayon::prelude::*;
use std::{
    cmp,
    collections::HashSet,
    sync::Arc,
    time::{Duration, Instant},
};
use storage_interface::state_view::LatestDbStateCheckpointView;
use tokio::runtime::Handle;
use vm_validator::vm_validator::{get_account_sequence_number, TransactionValidation};

// ============================== //
//  broadcast_coordinator tasks  //
// ============================== //

/// Attempts broadcast to `peer` and schedules the next broadcast.
pub(crate) async fn execute_broadcast<V>(
    peer: PeerNetworkId,
    backoff: bool,
    smp: &mut SharedMempool<V>,
    scheduled_broadcasts: &mut FuturesUnordered<ScheduledBroadcast>,
    executor: Handle,
) where
    V: TransactionValidation,
{
    let network_interface = &smp.network_interface.clone();
    // If there's no connection, don't bother to broadcast
    if network_interface.app_data().read(&peer).is_some() {
        if let Err(err) = network_interface
            .execute_broadcast(peer, backoff, smp)
            .await
        {
            match err {
                BroadcastError::NetworkError(peer, error) => error!(LogSchema::event_log(
                    LogEntry::BroadcastTransaction,
                    LogEvent::NetworkSendFail
                )
                .peer(&peer)
                .error(&error)),
                _ => {
                    trace!("{:?}", err)
                }
            }
        }
    } else {
        // Drop the scheduled broadcast, we're not connected anymore
        return;
    }
    let schedule_backoff = network_interface.is_backoff_mode(&peer);

    let interval_ms = if schedule_backoff {
        smp.config.shared_mempool_backoff_interval_ms
    } else {
        smp.config.shared_mempool_tick_interval_ms
    };

    scheduled_broadcasts.push(ScheduledBroadcast::new(
        Instant::now() + Duration::from_millis(interval_ms),
        peer,
        schedule_backoff,
        executor,
    ))
}

// =============================== //
// Tasks processing txn submission //
// =============================== //

/// Processes transactions directly submitted by client.
pub(crate) async fn process_client_transaction_submission<V>(
    smp: SharedMempool<V>,
    transaction: SignedTransaction,
    callback: oneshot::Sender<Result<SubmissionStatus>>,
    timer: HistogramTimer,
) where
    V: TransactionValidation,
{
    timer.stop_and_record();
    let _timer = counters::process_txn_submit_latency_timer_client();
    let ineligible_for_broadcast =
        !smp.broadcast_within_validator_network() && smp.network_interface.is_validator();
    let timeline_state = if ineligible_for_broadcast {
        TimelineState::NonQualified
    } else {
        TimelineState::NotReady
    };
    let statuses = process_incoming_transactions(&smp, vec![transaction], timeline_state);
    log_txn_process_results(&statuses, None);

    if let Some(status) = statuses.first() {
        if callback.send(Ok(status.1.clone())).is_err() {
            error!(LogSchema::event_log(
                LogEntry::JsonRpc,
                LogEvent::CallbackFail
            ));
            counters::CLIENT_CALLBACK_FAIL.inc();
        }
    }
}

/// Processes get transaction by hash request by client.
pub(crate) async fn process_client_get_transaction<V>(
    smp: SharedMempool<V>,
    hash: HashValue,
    callback: oneshot::Sender<Option<SignedTransaction>>,
    timer: HistogramTimer,
) where
    V: TransactionValidation,
{
    timer.stop_and_record();
    let _timer = counters::process_get_txn_latency_timer_client();
    let txn = smp.mempool.lock().get_by_hash(hash);

    if callback.send(txn).is_err() {
        error!(LogSchema::event_log(
            LogEntry::GetTransaction,
            LogEvent::CallbackFail
        ));
        counters::CLIENT_CALLBACK_FAIL.inc();
    }
}

/// Processes transactions from other nodes.
pub(crate) async fn process_transaction_broadcast<V>(
    smp: SharedMempool<V>,
    transactions: Vec<SignedTransaction>,
    request_id: BatchId,
    timeline_state: TimelineState,
    peer: PeerNetworkId,
    timer: HistogramTimer,
) where
    V: TransactionValidation,
{
    timer.stop_and_record();
    let _timer = counters::process_txn_submit_latency_timer(peer.network_id());
    let results = process_incoming_transactions(&smp, transactions, timeline_state);
    log_txn_process_results(&results, Some(peer));

    let ack_response = gen_ack_response(request_id, results, &peer);
    let network_sender = smp.network_interface.sender();

    // Respond to the peer with an ack. Note: ack response messages should be
    // small enough that they always fit within the maximum network message
    // size, so there's no need to check them here.
    if let Err(e) = network_sender.send_to(peer, ack_response) {
        counters::network_send_fail_inc(counters::ACK_TXNS);
        error!(
            LogSchema::event_log(LogEntry::BroadcastACK, LogEvent::NetworkSendFail)
                .peer(&peer)
                .error(&e.into())
        );
        return;
    }
    notify_subscribers(SharedMempoolNotification::ACK, &smp.subscribers);
}

/// If `MempoolIsFull` on any of the transactions, provide backpressure to the downstream peer.
fn gen_ack_response(
    request_id: BatchId,
    results: Vec<SubmissionStatusBundle>,
    peer: &PeerNetworkId,
) -> MempoolSyncMsg {
    let mut backoff_and_retry = false;
    for (_, (mempool_status, _)) in results.into_iter() {
        if mempool_status.code == MempoolStatusCode::MempoolIsFull {
            backoff_and_retry = true;
            break;
        }
    }

    update_ack_counter(
        peer,
        counters::SENT_LABEL,
        backoff_and_retry,
        backoff_and_retry,
    );
    MempoolSyncMsg::BroadcastTransactionsResponse {
        request_id,
        retry: backoff_and_retry,
        backoff: backoff_and_retry,
    }
}

pub(crate) fn update_ack_counter(
    peer: &PeerNetworkId,
    direction_label: &str,
    retry: bool,
    backoff: bool,
) {
    if retry {
        counters::shared_mempool_ack_inc(
            peer.network_id(),
            direction_label,
            counters::RETRY_BROADCAST_LABEL,
        );
    }
    if backoff {
        counters::shared_mempool_ack_inc(
            peer.network_id(),
            direction_label,
            counters::BACKPRESSURE_BROADCAST_LABEL,
        );
    }
}

/// Submits a list of SignedTransaction to the local mempool
/// and returns a vector containing AdmissionControlStatus.
pub(crate) fn process_incoming_transactions<V>(
    smp: &SharedMempool<V>,
    transactions: Vec<SignedTransaction>,
    timeline_state: TimelineState,
) -> Vec<SubmissionStatusBundle>
where
    V: TransactionValidation,
{
    let mut statuses = vec![];

    let start_storage_read = Instant::now();
    let state_view = smp
        .db
        .latest_state_checkpoint_view()
        .expect("Failed to get latest state checkpoint view.");

    // Track latency: fetching seq number
    let seq_numbers = IO_POOL.install(|| {
        transactions
            .par_iter()
            .map(|t| {
                get_account_sequence_number(&state_view, t.sender()).map_err(|e| {
                    error!(LogSchema::new(LogEntry::DBError).error(&e));
                    counters::DB_ERROR.inc();
                    e
                })
            })
            .collect::<Vec<_>>()
    });
    // Track latency for storage read fetching sequence number
    let storage_read_latency = start_storage_read.elapsed();
    counters::PROCESS_TXN_BREAKDOWN_LATENCY
        .with_label_values(&[counters::FETCH_SEQ_NUM_LABEL])
        .observe(storage_read_latency.as_secs_f64() / transactions.len() as f64);

    let transactions: Vec<_> = transactions
        .into_iter()
        .enumerate()
        .filter_map(|(idx, t)| {
            if let Ok(sequence_info) = seq_numbers[idx] {
                if t.sequence_number() >= sequence_info.min_seq() {
                    return Some((t, sequence_info));
                } else {
                    statuses.push((
                        t,
                        (
                            MempoolStatus::new(MempoolStatusCode::VmError),
                            Some(DiscardedVMStatus::SEQUENCE_NUMBER_TOO_OLD),
                        ),
                    ));
                }
            } else {
                // Failed to get transaction
                statuses.push((
                    t,
                    (
                        MempoolStatus::new(MempoolStatusCode::VmError),
                        Some(DiscardedVMStatus::RESOURCE_DOES_NOT_EXIST),
                    ),
                ));
            }
            None
        })
        .collect();

    // Track latency: VM validation
    let vm_validation_timer = counters::PROCESS_TXN_BREAKDOWN_LATENCY
        .with_label_values(&[counters::VM_VALIDATION_LABEL])
        .start_timer();
    let validation_results = transactions
        .iter()
        .map(|t| smp.validator.read().validate_transaction(t.0.clone()))
        .collect::<Vec<_>>();
    vm_validation_timer.stop_and_record();
    {
        let mut mempool = smp.mempool.lock();
        for (idx, (transaction, sequence_info)) in transactions.into_iter().enumerate() {
            if let Ok(validation_result) = &validation_results[idx] {
                match validation_result.status() {
                    None => {
                        let ranking_score = validation_result.score();
                        let mempool_status = mempool.add_txn(
                            transaction.clone(),
                            ranking_score,
                            sequence_info,
                            timeline_state,
                        );
                        statuses.push((transaction, (mempool_status, None)));
                    }
                    Some(validation_status) => {
                        statuses.push((
                            transaction.clone(),
                            (
                                MempoolStatus::new(MempoolStatusCode::VmError),
                                Some(validation_status),
                            ),
                        ));
                    }
                }
            } else {
                statuses.push((
                    transaction.clone(),
                    (
                        MempoolStatus::new(MempoolStatusCode::VmError),
                        Some(DiscardedVMStatus::UNKNOWN_STATUS),
                    ),
                ));
            }
        }
    }
    notify_subscribers(SharedMempoolNotification::NewTransactions, &smp.subscribers);
    statuses
}

fn log_txn_process_results(results: &[SubmissionStatusBundle], sender: Option<PeerNetworkId>) {
    let network = match sender {
        Some(peer) => peer.network_id().to_string(),
        None => counters::CLIENT_LABEL.to_string(),
    };
    for (txn, (mempool_status, maybe_vm_status)) in results.iter() {
        if let Some(vm_status) = maybe_vm_status {
            trace!(
                SecurityEvent::InvalidTransactionMempool,
                failed_transaction = txn,
                vm_status = vm_status,
                sender = sender,
            );
            counters::shared_mempool_transactions_processed_inc(
                counters::VM_VALIDATION_LABEL,
                &network,
            );
            continue;
        }
        match mempool_status.code {
            MempoolStatusCode::Accepted => counters::shared_mempool_transactions_processed_inc(
                counters::SUCCESS_LABEL,
                &network,
            ),
            _ => counters::shared_mempool_transactions_processed_inc(
                &mempool_status.code.to_string(),
                &network,
            ),
        }
    }
}

// ================================= //
// intra-node communication handlers //
// ================================= //

/// Only applies to Validators. Either provides transactions to consensus [`GetBlockRequest`] or
/// handles rejecting transactions [`RejectNotification`]
pub(crate) fn process_quorum_store_request<V: TransactionValidation>(
    smp: &SharedMempool<V>,
    req: QuorumStoreRequest,
) {
    // Start latency timer
    let start_time = Instant::now();
    debug!(LogSchema::event_log(LogEntry::QuorumStore, LogEvent::Received).quorum_store_msg(&req));

    let (resp, callback, counter_label) = match req {
        QuorumStoreRequest::GetBatchRequest(max_txns, max_bytes, transactions, callback) => {
            let exclude_transactions: HashSet<TxnPointer> = transactions
                .iter()
                .map(|txn| (txn.sender, txn.sequence_number))
                .collect();
            let txns;
            {
                let mut mempool = smp.mempool.lock();
                // gc before pulling block as extra protection against txns that may expire in consensus
                // Note: this gc operation relies on the fact that consensus uses the system time to determine block timestamp
                let curr_time = aptos_infallible::duration_since_epoch();
                mempool.gc_by_expiration_time(curr_time);
                let max_txns = cmp::max(max_txns, 1);
                txns = mempool.get_batch(max_txns, max_bytes, exclude_transactions);
            }

            // mempool_service_transactions is logged inside get_batch

            (
                QuorumStoreResponse::GetBatchResponse(txns),
                callback,
                counters::GET_BLOCK_LABEL,
            )
        }
        QuorumStoreRequest::RejectNotification(transactions, callback) => {
            counters::mempool_service_transactions(
                counters::COMMIT_CONSENSUS_LABEL,
                transactions.len(),
            );
            process_committed_transactions(&smp.mempool, transactions, 0, true);
            (
                QuorumStoreResponse::CommitResponse(),
                callback,
                counters::COMMIT_CONSENSUS_LABEL,
            )
        }
    };
    // Send back to callback
    let result = if callback.send(Ok(resp)).is_err() {
        error!(LogSchema::event_log(
            LogEntry::QuorumStore,
            LogEvent::CallbackFail
        ));
        counters::REQUEST_FAIL_LABEL
    } else {
        counters::REQUEST_SUCCESS_LABEL
    };
    let latency = start_time.elapsed();
    counters::mempool_service_latency(counter_label, result, latency);
}

/// Remove transactions that are committed (or rejected) so that we can stop broadcasting them.
pub(crate) fn process_committed_transactions(
    mempool: &Mutex<CoreMempool>,
    transactions: Vec<TransactionSummary>,
    block_timestamp_usecs: u64,
    is_rejected: bool,
) {
    let mut pool = mempool.lock();

    for transaction in transactions {
        pool.remove_transaction(
            &transaction.sender,
            transaction.sequence_number,
            is_rejected,
        );
    }

    if block_timestamp_usecs > 0 {
        pool.gc_by_expiration_time(Duration::from_micros(block_timestamp_usecs));
    }
}

/// Processes on-chain reconfiguration notifications.  Restarts validator with the new info.
pub(crate) async fn process_config_update<V>(
    config_update: OnChainConfigPayload,
    validator: Arc<RwLock<V>>,
) where
    V: TransactionValidation,
{
    info!(
        LogSchema::event_log(LogEntry::ReconfigUpdate, LogEvent::Process)
            .reconfig_update(config_update.clone())
    );

    if let Err(e) = validator.write().restart(config_update) {
        counters::VM_RECONFIG_UPDATE_FAIL_COUNT.inc();
        error!(LogSchema::event_log(LogEntry::ReconfigUpdate, LogEvent::VMUpdateFail).error(&e));
    }
}
