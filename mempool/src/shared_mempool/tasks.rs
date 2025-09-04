// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Tasks that are executed by coordinators (short-lived compared to coordinators)
use super::types::MempoolMessageId;
use crate::{
    core_mempool::{CoreMempool, TimelineState},
    counters,
    logging::{LogEntry, LogEvent, LogSchema},
    network::{BroadcastError, BroadcastPeerPriority, MempoolSyncMsg},
    shared_mempool::{
        types::{
            notify_subscribers, ScheduledBroadcast, SharedMempool, SharedMempoolNotification,
            SubmissionStatusBundle,
        },
        use_case_history::UseCaseHistory,
    },
    thread_pool::{IO_POOL, VALIDATION_POOL},
    QuorumStoreRequest, QuorumStoreResponse, SubmissionStatus,
};
use anyhow::Result;
use velor_config::{config::TransactionFilterConfig, network_id::PeerNetworkId};
use velor_consensus_types::common::RejectedTransactionSummary;
use velor_crypto::HashValue;
use velor_infallible::{Mutex, RwLock};
use velor_logger::prelude::*;
use velor_mempool_notifications::CommittedTransaction;
use velor_metrics_core::HistogramTimer;
use velor_network::application::interface::NetworkClientInterface;
use velor_storage_interface::state_store::state_view::db_state_view::LatestDbStateCheckpointView;
use velor_types::{
    account_address::AccountAddress,
    mempool_status::{MempoolStatus, MempoolStatusCode},
    on_chain_config::{OnChainConfigPayload, OnChainConfigProvider, OnChainConsensusConfig},
    transaction::{ReplayProtector, SignedTransaction},
    vm_status::{DiscardedVMStatus, StatusCode},
};
use velor_vm_validator::vm_validator::{get_account_sequence_number, TransactionValidation};
use futures::{channel::oneshot, stream::FuturesUnordered};
use rayon::prelude::*;
use std::{
    cmp,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::runtime::Handle;
// ============================== //
//  broadcast_coordinator tasks  //
// ============================== //

// The sample rate for broadcast events and errors
const BROADCAST_ERROR_LOG_SAMPLE_SECS: u64 = 1;
const BROADCAST_EVENT_LOG_SAMPLE_SECS: u64 = 5;

/// Attempts broadcast to `peer` and schedules the next broadcast.
pub(crate) async fn execute_broadcast<NetworkClient, TransactionValidator>(
    peer: PeerNetworkId,
    backoff: bool,
    smp: &mut SharedMempool<NetworkClient, TransactionValidator>,
    scheduled_broadcasts: &mut FuturesUnordered<ScheduledBroadcast>,
    executor: Handle,
) where
    NetworkClient: NetworkClientInterface<MempoolSyncMsg>,
    TransactionValidator: TransactionValidation,
{
    let network_interface = &smp.network_interface.clone();
    counters::shared_mempool_broadcast_event_inc(counters::RUNNING_LABEL, peer.network_id());

    // If there's no connection, don't bother to broadcast
    if network_interface.sync_states_exists(&peer) {
        if let Err(err) = network_interface
            .execute_broadcast(peer, backoff, smp)
            .await
        {
            counters::shared_mempool_broadcast_event_inc(err.get_label(), peer.network_id());
            match err {
                BroadcastError::NoTransactions(_) => {
                    sample!(
                        SampleRate::Duration(Duration::from_secs(BROADCAST_EVENT_LOG_SAMPLE_SECS)),
                        debug!("No transactions to broadcast: {:?}", err)
                    );
                },
                BroadcastError::PeerNotPrioritized(_, _) => {
                    sample!(
                        SampleRate::Duration(Duration::from_secs(BROADCAST_EVENT_LOG_SAMPLE_SECS)),
                        debug!(
                            "Peer {} not prioritized. Skipping broadcast: {:?}",
                            peer, err
                        )
                    );
                },
                _ => {
                    sample!(
                        SampleRate::Duration(Duration::from_secs(BROADCAST_ERROR_LOG_SAMPLE_SECS)),
                        warn!("Execute broadcast for peer {} failed: {:?}", peer, err)
                    );
                },
            }
        }
    } else {
        // Drop the scheduled broadcast, we're not connected anymore
        counters::shared_mempool_broadcast_event_inc(
            counters::DROP_BROADCAST_LABEL,
            peer.network_id(),
        );
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
pub(crate) async fn process_client_transaction_submission<NetworkClient, TransactionValidator>(
    smp: SharedMempool<NetworkClient, TransactionValidator>,
    transaction: SignedTransaction,
    callback: oneshot::Sender<Result<SubmissionStatus>>,
    timer: HistogramTimer,
) where
    NetworkClient: NetworkClientInterface<MempoolSyncMsg>,
    TransactionValidator: TransactionValidation + 'static,
{
    timer.stop_and_record();
    let _timer = counters::process_txn_submit_latency_timer_client();
    let ineligible_for_broadcast =
        smp.network_interface.is_validator() && !smp.broadcast_within_validator_network();
    let timeline_state = if ineligible_for_broadcast {
        TimelineState::NonQualified
    } else {
        TimelineState::NotReady
    };
    let statuses: Vec<(SignedTransaction, (MempoolStatus, Option<StatusCode>))> =
        process_incoming_transactions(
            &smp,
            vec![(transaction, None, Some(BroadcastPeerPriority::Primary))],
            timeline_state,
            true,
        );
    log_txn_process_results(&statuses, None);

    if let Some(status) = statuses.first() {
        if callback.send(Ok(status.1.clone())).is_err() {
            warn!(LogSchema::event_log(
                LogEntry::JsonRpc,
                LogEvent::CallbackFail
            ));
            counters::CLIENT_CALLBACK_FAIL.inc();
        }
    }
}

/// Processes request for all addresses in parking lot
pub(crate) async fn process_parking_lot_addresses<NetworkClient, TransactionValidator>(
    smp: SharedMempool<NetworkClient, TransactionValidator>,
    callback: oneshot::Sender<Vec<(AccountAddress, u64)>>,
) where
    NetworkClient: NetworkClientInterface<MempoolSyncMsg>,
    TransactionValidator: TransactionValidation + 'static,
{
    let addresses = smp.mempool.lock().get_parking_lot_addresses();

    if callback.send(addresses).is_err() {
        warn!(LogSchema::event_log(
            LogEntry::JsonRpc,
            LogEvent::CallbackFail
        ));
        counters::CLIENT_CALLBACK_FAIL.inc();
    }
}

/// Processes get transaction by hash request by client.
pub(crate) async fn process_client_get_transaction<NetworkClient, TransactionValidator>(
    smp: SharedMempool<NetworkClient, TransactionValidator>,
    hash: HashValue,
    callback: oneshot::Sender<Option<SignedTransaction>>,
    timer: HistogramTimer,
) where
    NetworkClient: NetworkClientInterface<MempoolSyncMsg>,
    TransactionValidator: TransactionValidation,
{
    timer.stop_and_record();
    let _timer = counters::process_get_txn_latency_timer_client();
    let txn = smp.mempool.lock().get_by_hash(hash);

    if callback.send(txn).is_err() {
        warn!(LogSchema::event_log(
            LogEntry::GetTransaction,
            LogEvent::CallbackFail
        ));
        counters::CLIENT_CALLBACK_FAIL.inc();
    }
}

/// Processes transactions from other nodes.
pub(crate) async fn process_transaction_broadcast<NetworkClient, TransactionValidator>(
    smp: SharedMempool<NetworkClient, TransactionValidator>,
    // The sender of the transactions can send the time at which the transactions were inserted
    // in the sender's mempool. The sender can also send the priority of this node for the sender
    // of the transactions.
    transactions: Vec<(
        SignedTransaction,
        Option<u64>,
        Option<BroadcastPeerPriority>,
    )>,
    message_id: MempoolMessageId,
    timeline_state: TimelineState,
    peer: PeerNetworkId,
    timer: HistogramTimer,
) where
    NetworkClient: NetworkClientInterface<MempoolSyncMsg>,
    TransactionValidator: TransactionValidation,
{
    timer.stop_and_record();
    let _timer = counters::process_txn_submit_latency_timer(peer.network_id());
    let results = process_incoming_transactions(&smp, transactions, timeline_state, false);
    log_txn_process_results(&results, Some(peer));

    let ack_response = gen_ack_response(message_id, results, &peer);

    // Respond to the peer with an ack. Note: ack response messages should be
    // small enough that they always fit within the maximum network message
    // size, so there's no need to check them here.
    if let Err(e) = smp
        .network_interface
        .send_message_to_peer(peer, ack_response)
    {
        counters::network_send_fail_inc(counters::ACK_TXNS);
        warn!(
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
    message_id: MempoolMessageId,
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
        message_id,
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
/// and returns a vector containing [SubmissionStatusBundle].
pub(crate) fn process_incoming_transactions<NetworkClient, TransactionValidator>(
    smp: &SharedMempool<NetworkClient, TransactionValidator>,
    transactions: Vec<(
        SignedTransaction,
        Option<u64>,
        Option<BroadcastPeerPriority>,
    )>,
    timeline_state: TimelineState,
    client_submitted: bool,
) -> Vec<SubmissionStatusBundle>
where
    NetworkClient: NetworkClientInterface<MempoolSyncMsg>,
    TransactionValidator: TransactionValidation,
{
    // Filter out any disallowed transactions
    let mut statuses = vec![];
    let transactions =
        filter_transactions(&smp.transaction_filter_config, transactions, &mut statuses);

    // If there are no transactions left after filtering, return early
    if transactions.is_empty() {
        return statuses;
    }

    let start_storage_read = Instant::now();
    let state_view = smp
        .db
        .latest_state_checkpoint_view()
        .expect("Failed to get latest state checkpoint view.");

    // Track latency: fetching seq number
    let account_seq_numbers = IO_POOL.install(|| {
        transactions
            .par_iter()
            .map(|(t, _, _)| match t.replay_protector() {
                ReplayProtector::Nonce(_) => Ok(None),
                ReplayProtector::SequenceNumber(_) => {
                    get_account_sequence_number(&state_view, t.sender())
                        .map(Some)
                        .inspect_err(|e| {
                            error!(LogSchema::new(LogEntry::DBError).error(e));
                            counters::DB_ERROR.inc();
                        })
                },
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
        .filter_map(|(idx, (t, ready_time_at_sender, priority))| {
            if let Ok(account_sequence_num) = account_seq_numbers[idx] {
                match account_sequence_num {
                    Some(sequence_num) => {
                        if t.sequence_number() >= sequence_num {
                            return Some((t, Some(sequence_num), ready_time_at_sender, priority));
                        } else {
                            statuses.push((
                                t,
                                (
                                    MempoolStatus::new(MempoolStatusCode::VmError),
                                    Some(DiscardedVMStatus::SEQUENCE_NUMBER_TOO_OLD),
                                ),
                            ));
                        }
                    },
                    None => {
                        return Some((t, None, ready_time_at_sender, priority));
                    },
                }
            } else {
                // Failed to get account's onchain sequence number
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

    validate_and_add_transactions(
        transactions,
        smp,
        timeline_state,
        &mut statuses,
        client_submitted,
    );
    notify_subscribers(SharedMempoolNotification::NewTransactions, &smp.subscribers);
    statuses
}

/// Filters transactions based on the transaction filter configuration. Any
/// transactions that are filtered out will have their statuses marked accordingly.
fn filter_transactions(
    transaction_filter_config: &TransactionFilterConfig,
    transactions: Vec<(
        SignedTransaction,
        Option<u64>,
        Option<BroadcastPeerPriority>,
    )>,
    statuses: &mut Vec<(SignedTransaction, (MempoolStatus, Option<StatusCode>))>,
) -> Vec<(
    SignedTransaction,
    Option<u64>,
    Option<BroadcastPeerPriority>,
)> {
    // If the filter is not enabled, return early
    if !transaction_filter_config.is_enabled() {
        return transactions;
    }

    // Start the filter processing timer
    let transaction_filter_timer = counters::PROCESS_TXN_BREAKDOWN_LATENCY
        .with_label_values(&[counters::FILTER_TRANSACTIONS_LABEL])
        .start_timer();

    // Filter the transactions and update the statuses accordingly
    let transactions = transactions
        .into_iter()
        .filter_map(|(transaction, account_sequence_number, priority)| {
            if transaction_filter_config
                .transaction_filter()
                .allows_transaction(&transaction)
            {
                Some((transaction, account_sequence_number, priority))
            } else {
                info!(LogSchema::event_log(
                    LogEntry::TransactionFilter,
                    LogEvent::TransactionRejected
                )
                .message(&format!(
                    "Transaction {} rejected by filter",
                    transaction.committed_hash()
                )));

                statuses.push((
                    transaction.clone(),
                    (
                        MempoolStatus::new(MempoolStatusCode::RejectedByFilter),
                        None,
                    ),
                ));
                None
            }
        })
        .collect();

    // Update the filter processing latency metrics
    transaction_filter_timer.stop_and_record();

    transactions
}

/// Perfoms VM validation on the transactions and inserts those that passes
/// validation into the mempool.
#[cfg(not(feature = "consensus-only-perf-test"))]
fn validate_and_add_transactions<NetworkClient, TransactionValidator>(
    transactions: Vec<(
        SignedTransaction,
        Option<u64>,
        Option<u64>,
        Option<BroadcastPeerPriority>,
    )>,
    smp: &SharedMempool<NetworkClient, TransactionValidator>,
    timeline_state: TimelineState,
    statuses: &mut Vec<(SignedTransaction, (MempoolStatus, Option<StatusCode>))>,
    client_submitted: bool,
) where
    NetworkClient: NetworkClientInterface<MempoolSyncMsg>,
    TransactionValidator: TransactionValidation,
{
    // Track latency: VM validation
    let vm_validation_timer = counters::PROCESS_TXN_BREAKDOWN_LATENCY
        .with_label_values(&[counters::VM_VALIDATION_LABEL])
        .start_timer();
    let validation_results = VALIDATION_POOL.install(|| {
        transactions
            .par_iter()
            .map(|t| {
                let result = smp.validator.read().validate_transaction(t.0.clone());
                // Pre-compute the hash and length if the transaction is valid, before locking mempool
                if result.is_ok() {
                    t.0.committed_hash();
                    t.0.txn_bytes_len();
                }
                result
            })
            .collect::<Vec<_>>()
    });
    vm_validation_timer.stop_and_record();
    {
        let mut mempool = smp.mempool.lock();
        for (idx, (transaction, account_sequence_number, ready_time_at_sender, priority)) in
            transactions.into_iter().enumerate()
        {
            if let Ok(validation_result) = &validation_results[idx] {
                match validation_result.status() {
                    None => {
                        let ranking_score = validation_result.score();
                        let mempool_status = mempool.add_txn(
                            transaction.clone(),
                            ranking_score,
                            account_sequence_number,
                            timeline_state,
                            client_submitted,
                            ready_time_at_sender,
                            priority.clone(),
                        );
                        statuses.push((transaction, (mempool_status, None)));
                    },
                    Some(validation_status) => {
                        statuses.push((
                            transaction.clone(),
                            (
                                MempoolStatus::new(MempoolStatusCode::VmError),
                                Some(validation_status),
                            ),
                        ));
                    },
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
}

/// In consensus-only mode, insert transactions into the mempool directly
/// without any VM validation.
///
/// We want to populate transactions as fast as and
/// as much as possible into the mempool, and the VM validator would interfere with
/// this because validation has some overhead and the validator bounds the number of
/// outstanding sequence numbers.
#[cfg(feature = "consensus-only-perf-test")]
fn validate_and_add_transactions<NetworkClient, TransactionValidator>(
    transactions: Vec<(SignedTransaction, Option<u64>, Option<u64>)>,
    smp: &SharedMempool<NetworkClient, TransactionValidator>,
    timeline_state: TimelineState,
    statuses: &mut Vec<(
        SignedTransaction,
        (
            MempoolStatus,
            Option<StatusCode>,
            Option<BroadcastPeerPriority>,
        ),
    )>,
    client_submitted: bool,
) where
    NetworkClient: NetworkClientInterface<MempoolSyncMsg>,
    TransactionValidator: TransactionValidation,
{
    use super::priority;

    let mut mempool = smp.mempool.lock();
    for (transaction, account_sequence_number, ready_time_at_sender, priority) in
        transactions.into_iter()
    {
        let mempool_status = mempool.add_txn(
            transaction.clone(),
            0,
            account_sequence_number,
            timeline_state,
            client_submitted,
            read_time_at_sender,
            priority,
        );
        statuses.push((transaction, (mempool_status, None)));
    }
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
pub(crate) fn process_quorum_store_request<NetworkClient, TransactionValidator>(
    smp: &SharedMempool<NetworkClient, TransactionValidator>,
    req: QuorumStoreRequest,
) where
    NetworkClient: NetworkClientInterface<MempoolSyncMsg>,
    TransactionValidator: TransactionValidation,
{
    // Start latency timer
    let start_time = Instant::now();

    let (resp, callback, counter_label) = match req {
        QuorumStoreRequest::GetBatchRequest(
            max_txns,
            max_bytes,
            return_non_full,
            exclude_transactions,
            callback,
        ) => {
            let txns;
            {
                let lock_timer = counters::mempool_service_start_latency_timer(
                    counters::GET_BLOCK_LOCK_LABEL,
                    counters::REQUEST_SUCCESS_LABEL,
                );
                let mut mempool = smp.mempool.lock();
                lock_timer.observe_duration();

                {
                    let _gc_timer = counters::mempool_service_start_latency_timer(
                        counters::GET_BLOCK_GC_LABEL,
                        counters::REQUEST_SUCCESS_LABEL,
                    );
                    // gc before pulling block as extra protection against txns that may expire in consensus
                    // Note: this gc operation relies on the fact that consensus uses the system time to determine block timestamp
                    let curr_time = velor_infallible::duration_since_epoch();
                    mempool.gc_by_expiration_time(curr_time);
                }

                let max_txns = cmp::max(max_txns, 1);
                let _get_batch_timer = counters::mempool_service_start_latency_timer(
                    counters::GET_BLOCK_GET_BATCH_LABEL,
                    counters::REQUEST_SUCCESS_LABEL,
                );
                txns =
                    mempool.get_batch(max_txns, max_bytes, return_non_full, exclude_transactions);
            }

            // mempool_service_transactions is logged inside get_batch

            (
                QuorumStoreResponse::GetBatchResponse(txns),
                callback,
                counters::GET_BLOCK_LABEL,
            )
        },
        QuorumStoreRequest::RejectNotification(transactions, callback) => {
            counters::mempool_service_transactions(
                counters::COMMIT_CONSENSUS_LABEL,
                transactions.len(),
            );
            process_rejected_transactions(&smp.mempool, transactions);
            (
                QuorumStoreResponse::CommitResponse(),
                callback,
                counters::COMMIT_CONSENSUS_LABEL,
            )
        },
    };
    // Send back to callback
    let result = if callback.send(Ok(resp)).is_err() {
        debug!(LogSchema::event_log(
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
    use_case_history: &Mutex<UseCaseHistory>,
    transactions: Vec<CommittedTransaction>,
    block_timestamp_usecs: u64,
) {
    let mut pool = mempool.lock();
    let block_timestamp = Duration::from_micros(block_timestamp_usecs);

    let tracking_usecases = {
        let mut history = use_case_history.lock();
        history.update_usecases(&transactions);
        history.compute_tracking_set()
    };

    for transaction in transactions {
        pool.log_commit_transaction(
            &transaction.sender,
            transaction.replay_protector,
            tracking_usecases
                .get(&transaction.use_case)
                .map(|name| (transaction.use_case.clone(), name)),
            block_timestamp,
        );
        pool.commit_transaction(&transaction.sender, transaction.replay_protector);
    }

    if block_timestamp_usecs > 0 {
        pool.gc_by_expiration_time(block_timestamp);
    }
}

pub(crate) fn process_rejected_transactions(
    mempool: &Mutex<CoreMempool>,
    transactions: Vec<RejectedTransactionSummary>,
) {
    let mut pool = mempool.lock();

    for transaction in transactions {
        pool.reject_transaction(
            &transaction.sender,
            transaction.replay_protector,
            &transaction.hash,
            &transaction.reason,
        );
    }
}

/// Processes on-chain reconfiguration notifications.  Restarts validator with the new info.
pub(crate) async fn process_config_update<V, P>(
    config_update: OnChainConfigPayload<P>,
    validator: Arc<RwLock<V>>,
    broadcast_within_validator_network: Arc<RwLock<bool>>,
) where
    V: TransactionValidation,
    P: OnChainConfigProvider,
{
    info!(LogSchema::event_log(
        LogEntry::ReconfigUpdate,
        LogEvent::Process
    ));

    if let Err(e) = validator.write().restart() {
        counters::VM_RECONFIG_UPDATE_FAIL_COUNT.inc();
        error!(LogSchema::event_log(LogEntry::ReconfigUpdate, LogEvent::VMUpdateFail).error(&e));
    }

    let consensus_config: anyhow::Result<OnChainConsensusConfig> = config_update.get();
    match consensus_config {
        Ok(consensus_config) => {
            *broadcast_within_validator_network.write() =
                !consensus_config.quorum_store_enabled() && !consensus_config.is_dag_enabled()
        },
        Err(e) => {
            error!(
                "Failed to read on-chain consensus config, keeping value broadcast_within_validator_network={}: {}",
                *broadcast_within_validator_network.read(),
                e
            );
        },
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use velor_crypto::{ed25519::Ed25519PrivateKey, PrivateKey, SigningKey, Uniform};
    use velor_transaction_filters::transaction_filter::TransactionFilter;
    use velor_types::{
        chain_id::ChainId,
        transaction::{RawTransaction, Script, TransactionPayload},
    };

    #[test]
    fn test_filter_transactions() {
        // Create test transactions
        let mut transactions = vec![];
        for _ in 0..10 {
            let transaction = create_signed_transaction();
            transactions.push((transaction, None, Some(BroadcastPeerPriority::Primary)));
        }

        // Create a config with filtering enabled (the first and last transactions will be rejected)
        let transaction_filter = TransactionFilter::empty()
            .add_sender_filter(false, transactions[0].0.sender())
            .add_sender_filter(false, transactions[9].0.sender());
        let transaction_filter_config = TransactionFilterConfig::new(true, transaction_filter);

        // Filter the transactions
        let mut statuses = vec![];
        let filtered_transactions = filter_transactions(
            &transaction_filter_config,
            transactions.clone(),
            &mut statuses,
        );

        // Verify that the first and last transactions are filtered out
        assert_eq!(filtered_transactions.len(), 8);
        assert!(!filtered_transactions.contains(&transactions[0]));
        assert!(!filtered_transactions.contains(&transactions[9]));

        // Verify the filtered transaction statuses
        assert_eq!(statuses.len(), 2);
        verify_rejected_status(statuses[0].clone(), transactions[0].0.clone());
        verify_rejected_status(statuses[1].clone(), transactions[9].0.clone());
    }

    #[test]
    fn test_filter_transactions_disabled() {
        // Create test transactions
        let num_transactions = 10;
        let mut transactions = vec![];
        for _ in 0..num_transactions {
            let transaction = create_signed_transaction();
            transactions.push((transaction, None, Some(BroadcastPeerPriority::Primary)));
        }

        // Create a config with filtering disabled
        let transaction_filter = TransactionFilter::empty().add_all_filter(false); // Reject all transactions
        let transaction_filter_config = TransactionFilterConfig::new(false, transaction_filter);

        // Filter the transactions
        let mut statuses = vec![];
        let filtered_transactions = filter_transactions(
            &transaction_filter_config,
            transactions.clone(),
            &mut statuses,
        );

        // Verify that all transactions are retained
        assert_eq!(filtered_transactions.len(), num_transactions);
        assert!(statuses.is_empty());
        for transaction in transactions {
            assert!(filtered_transactions.contains(&transaction));
        }
    }

    #[test]
    fn test_filter_transactions_empty() {
        // Create test transactions
        let num_transactions = 10;
        let mut transactions = vec![];
        for _ in 0..num_transactions {
            let transaction = create_signed_transaction();
            transactions.push((transaction, None, Some(BroadcastPeerPriority::Primary)));
        }

        // Create a config with filtering enabled (the filter is empty, so no transactions will be rejected)
        let transaction_filter = TransactionFilter::empty(); // Allow all transactions
        let transaction_filter_config = TransactionFilterConfig::new(true, transaction_filter);

        // Filter the transactions
        let mut statuses = vec![];
        let filtered_transactions = filter_transactions(
            &transaction_filter_config,
            transactions.clone(),
            &mut statuses,
        );

        // Verify that all transactions are retained
        assert_eq!(filtered_transactions.len(), num_transactions);
        assert!(statuses.is_empty());
        for transaction in transactions {
            assert!(filtered_transactions.contains(&transaction));
        }
    }

    fn create_raw_transaction() -> RawTransaction {
        RawTransaction::new(
            AccountAddress::random(),
            0,
            TransactionPayload::Script(Script::new(vec![], vec![], vec![])),
            0,
            0,
            0,
            ChainId::new(10),
        )
    }

    fn create_signed_transaction() -> SignedTransaction {
        let raw_transaction = create_raw_transaction();
        let private_key_1 = Ed25519PrivateKey::generate_for_testing();
        let signature = private_key_1.sign(&raw_transaction).unwrap();

        SignedTransaction::new(
            raw_transaction.clone(),
            private_key_1.public_key(),
            signature.clone(),
        )
    }

    fn verify_rejected_status(
        status: (SignedTransaction, (MempoolStatus, Option<StatusCode>)),
        transaction: SignedTransaction,
    ) {
        let rejected_status = MempoolStatus::new(MempoolStatusCode::RejectedByFilter);
        assert_eq!(status, (transaction, (rejected_status, None)));
    }
}
