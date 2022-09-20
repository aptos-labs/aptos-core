// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::metrics;
use crate::{
    error::Error,
    logging::{LogEntry, LogSchema},
    metadata_storage::MetadataStorageInterface,
    notification_handlers::{
        CommitNotification, CommittedTransactions, ErrorNotification, MempoolNotificationHandler,
    },
    utils,
};
use aptos_config::config::StateSyncDriverConfig;
use aptos_infallible::Mutex;
use aptos_logger::prelude::*;
use aptos_types::state_store::state_key::StateKey;
use aptos_types::state_store::state_value::StateValue;
use aptos_types::transaction::Version;
use aptos_types::{
    ledger_info::LedgerInfoWithSignatures,
    state_store::state_value::StateValueChunkWithProof,
    transaction::{
        Transaction, TransactionListWithProof, TransactionOutput, TransactionOutputListWithProof,
    },
};
use async_trait::async_trait;
use data_streaming_service::data_notification::NotificationId;
use event_notifications::EventSubscriptionService;
use executor_types::ChunkExecutorTrait;
use futures::channel::mpsc::UnboundedSender;
use futures::{channel::mpsc, SinkExt, StreamExt};
use mempool_notifications::MempoolNotificationSender;
use std::{
    future::Future,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use storage_interface::{DbReader, DbReaderWriter, StateSnapshotReceiver};
use tokio::{
    runtime::{Handle, Runtime},
    task::JoinHandle,
};

/// Synchronizes the storage of the node by verifying and storing new data
/// (e.g., transactions and outputs).
#[async_trait]
pub trait StorageSynchronizerInterface {
    /// Applies a batch of transaction outputs.
    ///
    /// Note: this assumes that the ledger infos have already been verified.
    async fn apply_transaction_outputs(
        &mut self,
        notification_id: NotificationId,
        output_list_with_proof: TransactionOutputListWithProof,
        target_ledger_info: LedgerInfoWithSignatures,
        end_of_epoch_ledger_info: Option<LedgerInfoWithSignatures>,
    ) -> Result<(), Error>;

    /// Executes a batch of transactions.
    ///
    /// Note: this assumes that the ledger infos have already been verified.
    async fn execute_transactions(
        &mut self,
        notification_id: NotificationId,
        transaction_list_with_proof: TransactionListWithProof,
        target_ledger_info: LedgerInfoWithSignatures,
        end_of_epoch_ledger_info: Option<LedgerInfoWithSignatures>,
    ) -> Result<(), Error>;

    /// Initializes a state synchronizer with the specified
    /// `target_ledger_info` and `target_output_with_proof` at the target
    /// syncing version. Returns a join handle to the state synchronizer.
    ///
    /// Note: this assumes that `epoch_change_proofs`, `target_ledger_info`,
    /// and `target_output_with_proof` have already been verified.
    fn initialize_state_synchronizer(
        &mut self,
        epoch_change_proofs: Vec<LedgerInfoWithSignatures>,
        target_ledger_info: LedgerInfoWithSignatures,
        target_output_with_proof: TransactionOutputListWithProof,
    ) -> Result<JoinHandle<()>, Error>;

    /// Returns true iff there is storage data that is still waiting
    /// to be executed/applied or committed.
    fn pending_storage_data(&self) -> bool;

    /// Saves the given state values to storage.
    ///
    /// Note: this requires that `initialize_state_synchronizer` has been
    /// called.
    fn save_state_values(
        &mut self,
        notification_id: NotificationId,
        state_value_chunk_with_proof: StateValueChunkWithProof,
    ) -> Result<(), Error>;

    /// Resets the chunk executor. This is required to support continuous
    /// interaction between consensus and state sync.
    fn reset_chunk_executor(&self) -> Result<(), Error>;

    /// Finish the chunk executor at this round of state sync by releasing
    /// any in-memory resources to prevent memory leak.
    fn finish_chunk_executor(&self);
}

/// The implementation of the `StorageSynchronizerInterface` used by state sync
pub struct StorageSynchronizer<ChunkExecutor, MetadataStorage> {
    // The executor for transaction and transaction output chunks
    chunk_executor: Arc<ChunkExecutor>,

    // A channel through which to notify the driver of committed data
    commit_notification_sender: mpsc::UnboundedSender<CommitNotification>,

    // The configuration of the state sync driver
    driver_config: StateSyncDriverConfig,

    // A channel through which to notify the driver of storage errors
    error_notification_sender: mpsc::UnboundedSender<ErrorNotification>,

    // A channel through which to notify the executor of new data chunks
    executor_notifier: mpsc::Sender<StorageDataChunk>,

    // The storage to write metadata about the syncing progress
    metadata_storage: MetadataStorage,

    // The number of storage data chunks pending execute/apply, or commit
    pending_data_chunks: Arc<AtomicU64>,

    // An optional runtime on which to spawn the storage synchronizer threads
    runtime: Option<Handle>,

    // The channel through which to notify the state snapshot receiver of new data chunks
    state_snapshot_notifier: Option<mpsc::Sender<StorageDataChunk>>,

    // The reader and writer for storage (required for state syncing)
    storage: DbReaderWriter,
}

// TODO(joshlind): this cannot currently be derived because of limitations around
// how deriving `Clone` works. See: https://github.com/rust-lang/rust/issues/26925.
impl<
        ChunkExecutor: ChunkExecutorTrait + 'static,
        MetadataStorage: MetadataStorageInterface + Clone,
    > Clone for StorageSynchronizer<ChunkExecutor, MetadataStorage>
{
    fn clone(&self) -> Self {
        Self {
            chunk_executor: self.chunk_executor.clone(),
            commit_notification_sender: self.commit_notification_sender.clone(),
            driver_config: self.driver_config,
            error_notification_sender: self.error_notification_sender.clone(),
            executor_notifier: self.executor_notifier.clone(),
            pending_data_chunks: self.pending_data_chunks.clone(),
            metadata_storage: self.metadata_storage.clone(),
            runtime: self.runtime.clone(),
            state_snapshot_notifier: self.state_snapshot_notifier.clone(),
            storage: self.storage.clone(),
        }
    }
}

impl<
        ChunkExecutor: ChunkExecutorTrait + 'static,
        MetadataStorage: MetadataStorageInterface + Clone,
    > StorageSynchronizer<ChunkExecutor, MetadataStorage>
{
    /// Returns a new storage synchronizer alongside the executor and committer handles
    pub fn new<MempoolNotifier: MempoolNotificationSender>(
        driver_config: StateSyncDriverConfig,
        chunk_executor: Arc<ChunkExecutor>,
        commit_notification_sender: mpsc::UnboundedSender<CommitNotification>,
        error_notification_sender: mpsc::UnboundedSender<ErrorNotification>,
        event_subscription_service: Arc<Mutex<EventSubscriptionService>>,
        mempool_notification_handler: MempoolNotificationHandler<MempoolNotifier>,
        metadata_storage: MetadataStorage,
        storage: DbReaderWriter,
        runtime: Option<&Runtime>,
    ) -> (Self, JoinHandle<()>, JoinHandle<()>) {
        // Create a channel to notify the executor when data chunks are ready
        let max_pending_data_chunks = driver_config.max_pending_data_chunks as usize;
        let (executor_notifier, executor_listener) = mpsc::channel(max_pending_data_chunks);

        // Create a channel to notify the committer when executed chunks are ready
        let (committer_notifier, committer_listener) = mpsc::channel(max_pending_data_chunks);

        // Create a shared pending data chunk counter
        let pending_transaction_chunks = Arc::new(AtomicU64::new(0));

        // Spawn the executor that executes/applies storage data chunks
        let runtime = runtime.map(|runtime| runtime.handle().clone());
        let executor_handle = spawn_executor(
            chunk_executor.clone(),
            error_notification_sender.clone(),
            executor_listener,
            committer_notifier,
            pending_transaction_chunks.clone(),
            runtime.clone(),
        );

        // Spawn the committer that commits executed (but pending) chunks
        let committer_handle = spawn_committer(
            chunk_executor.clone(),
            committer_listener,
            error_notification_sender.clone(),
            event_subscription_service,
            mempool_notification_handler,
            pending_transaction_chunks.clone(),
            runtime.clone(),
            storage.reader.clone(),
        );

        // Initialize the metric gauges
        utils::initialize_sync_gauges(storage.reader.clone())
            .expect("Failed to initialize the metric gauges!");

        let storage_synchronizer = Self {
            chunk_executor,
            commit_notification_sender,
            driver_config,
            error_notification_sender,
            executor_notifier,
            pending_data_chunks: pending_transaction_chunks,
            metadata_storage,
            runtime,
            state_snapshot_notifier: None,
            storage,
        };

        (storage_synchronizer, executor_handle, committer_handle)
    }

    /// Notifies the executor of new data chunks
    async fn notify_executor(&mut self, storage_data_chunk: StorageDataChunk) -> Result<(), Error> {
        if let Err(error) = self.executor_notifier.send(storage_data_chunk).await {
            Err(Error::UnexpectedError(format!(
                "Failed to send storage data chunk to executor: {:?}",
                error
            )))
        } else {
            increment_pending_data_chunks(self.pending_data_chunks.clone());
            Ok(())
        }
    }
}

#[async_trait]
impl<
        ChunkExecutor: ChunkExecutorTrait + 'static,
        MetadataStorage: MetadataStorageInterface + Clone + Send + Sync + 'static,
    > StorageSynchronizerInterface for StorageSynchronizer<ChunkExecutor, MetadataStorage>
{
    async fn apply_transaction_outputs(
        &mut self,
        notification_id: NotificationId,
        output_list_with_proof: TransactionOutputListWithProof,
        target_ledger_info: LedgerInfoWithSignatures,
        end_of_epoch_ledger_info: Option<LedgerInfoWithSignatures>,
    ) -> Result<(), Error> {
        let storage_data_chunk = StorageDataChunk::TransactionOutputs(
            notification_id,
            output_list_with_proof,
            target_ledger_info,
            end_of_epoch_ledger_info,
        );
        self.notify_executor(storage_data_chunk).await
    }

    async fn execute_transactions(
        &mut self,
        notification_id: NotificationId,
        transaction_list_with_proof: TransactionListWithProof,
        target_ledger_info: LedgerInfoWithSignatures,
        end_of_epoch_ledger_info: Option<LedgerInfoWithSignatures>,
    ) -> Result<(), Error> {
        let storage_data_chunk = StorageDataChunk::Transactions(
            notification_id,
            transaction_list_with_proof,
            target_ledger_info,
            end_of_epoch_ledger_info,
        );
        self.notify_executor(storage_data_chunk).await
    }

    fn initialize_state_synchronizer(
        &mut self,
        epoch_change_proofs: Vec<LedgerInfoWithSignatures>,
        target_ledger_info: LedgerInfoWithSignatures,
        target_output_with_proof: TransactionOutputListWithProof,
    ) -> Result<JoinHandle<()>, Error> {
        // Create a channel to notify the state snapshot receiver when data chunks are ready
        let max_pending_data_chunks = self.driver_config.max_pending_data_chunks as usize;
        let (state_snapshot_notifier, state_snapshot_listener) =
            mpsc::channel(max_pending_data_chunks);

        // Spawn the state snapshot receiver that commits state values
        let receiver_handle = spawn_state_snapshot_receiver(
            self.chunk_executor.clone(),
            state_snapshot_listener,
            self.commit_notification_sender.clone(),
            self.error_notification_sender.clone(),
            self.pending_data_chunks.clone(),
            self.metadata_storage.clone(),
            self.storage.clone(),
            epoch_change_proofs,
            target_ledger_info,
            target_output_with_proof,
            self.runtime.clone(),
        );
        self.state_snapshot_notifier = Some(state_snapshot_notifier);

        Ok(receiver_handle)
    }

    fn pending_storage_data(&self) -> bool {
        load_pending_data_chunks(self.pending_data_chunks.clone()) > 0
    }

    fn save_state_values(
        &mut self,
        notification_id: NotificationId,
        state_value_chunk_with_proof: StateValueChunkWithProof,
    ) -> Result<(), Error> {
        let state_snapshot_notifier = &mut self
            .state_snapshot_notifier
            .as_mut()
            .expect("The state snapshot receiver has not been initialized!");
        let storage_data_chunk =
            StorageDataChunk::States(notification_id, state_value_chunk_with_proof);
        if let Err(error) = state_snapshot_notifier.try_send(storage_data_chunk) {
            Err(Error::UnexpectedError(format!(
                "Failed to send storage data chunk to state snapshot listener: {:?}",
                error
            )))
        } else {
            increment_pending_data_chunks(self.pending_data_chunks.clone());
            Ok(())
        }
    }

    fn reset_chunk_executor(&self) -> Result<(), Error> {
        self.chunk_executor.reset().map_err(|error| {
            Error::UnexpectedError(format!(
                "Failed to reset the chunk executor! Error: {:?}",
                error
            ))
        })
    }

    fn finish_chunk_executor(&self) {
        self.chunk_executor.finish()
    }
}

/// A chunk of data to be executed and/or committed to storage (i.e., states,
/// transactions or outputs).
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
enum StorageDataChunk {
    States(NotificationId, StateValueChunkWithProof),
    Transactions(
        NotificationId,
        TransactionListWithProof,
        LedgerInfoWithSignatures,
        Option<LedgerInfoWithSignatures>,
    ),
    TransactionOutputs(
        NotificationId,
        TransactionOutputListWithProof,
        LedgerInfoWithSignatures,
        Option<LedgerInfoWithSignatures>,
    ),
}

/// Spawns a dedicated executor that executes/applies storage data chunks
fn spawn_executor<ChunkExecutor: ChunkExecutorTrait + 'static>(
    chunk_executor: Arc<ChunkExecutor>,
    error_notification_sender: mpsc::UnboundedSender<ErrorNotification>,
    mut executor_listener: mpsc::Receiver<StorageDataChunk>,
    mut committer_notifier: mpsc::Sender<NotificationId>,
    pending_transaction_chunks: Arc<AtomicU64>,
    runtime: Option<Handle>,
) -> JoinHandle<()> {
    // Create an executor
    let executor = async move {
        while let Some(storage_data_chunk) = executor_listener.next().await {
            // Execute/apply the storage data chunk
            let (notification_id, result) = match storage_data_chunk {
                StorageDataChunk::Transactions(
                    notification_id,
                    transactions_with_proof,
                    target_ledger_info,
                    end_of_epoch_ledger_info,
                ) => {
                    let timer = metrics::start_timer(
                        &metrics::STORAGE_SYNCHRONIZER_LATENCIES,
                        metrics::STORAGE_SYNCHRONIZER_EXECUTE_CHUNK,
                    );
                    let num_transactions = transactions_with_proof.transactions.len();
                    let result = chunk_executor.execute_chunk(
                        transactions_with_proof,
                        &target_ledger_info,
                        end_of_epoch_ledger_info.as_ref(),
                    );
                    if result.is_ok() {
                        info!(
                            LogSchema::new(LogEntry::StorageSynchronizer).message(&format!(
                                "Executed a new transaction chunk! Transaction total: {:?}.",
                                num_transactions
                            ))
                        );
                        metrics::increment_gauge(
                            &metrics::STORAGE_SYNCHRONIZER_OPERATIONS,
                            metrics::StorageSynchronizerOperations::ExecutedTransactions
                                .get_label(),
                            num_transactions as u64,
                        );
                    }
                    drop(timer);
                    (notification_id, result)
                }
                StorageDataChunk::TransactionOutputs(
                    notification_id,
                    outputs_with_proof,
                    target_ledger_info,
                    end_of_epoch_ledger_info,
                ) => {
                    let timer = metrics::start_timer(
                        &metrics::STORAGE_SYNCHRONIZER_LATENCIES,
                        metrics::STORAGE_SYNCHRONIZER_APPLY_CHUNK,
                    );
                    let num_outputs = outputs_with_proof.transactions_and_outputs.len();
                    let chunk_executor_clone = chunk_executor.clone();
                    // `spawn_blocking` so that the heavy synchronous function call doesn't
                    // block the async thread.
                    let result = tokio::task::spawn_blocking(move || {
                        chunk_executor_clone.apply_chunk(
                            outputs_with_proof,
                            &target_ledger_info,
                            end_of_epoch_ledger_info.as_ref(),
                        )
                    })
                    .await
                    .expect("spawn_blocking(apply_chunk) failed.");
                    if result.is_ok() {
                        info!(
                            LogSchema::new(LogEntry::StorageSynchronizer).message(&format!(
                                "Applied a new transaction output chunk! Transaction total: {:?}.",
                                num_outputs
                            ))
                        );
                        metrics::increment_gauge(
                            &metrics::STORAGE_SYNCHRONIZER_OPERATIONS,
                            metrics::StorageSynchronizerOperations::AppliedTransactionOutputs
                                .get_label(),
                            num_outputs as u64,
                        );
                    }
                    drop(timer);
                    (notification_id, result)
                }
                storage_data_chunk => {
                    panic!(
                        "Invalid storage data chunk sent to executor: {:?}",
                        storage_data_chunk
                    );
                }
            };

            // Notify the committer of new executed chunks
            match result {
                Ok(()) => {
                    if let Err(error) = committer_notifier.send(notification_id).await {
                        let error = format!("Failed to notify the committer! Error: {:?}", error);
                        send_storage_synchronizer_error(
                            error_notification_sender.clone(),
                            notification_id,
                            error,
                        )
                        .await;
                        decrement_pending_data_chunks(pending_transaction_chunks.clone());
                    }
                }
                Err(error) => {
                    let error = format!(
                        "Failed to execute/apply the storage data chunk! Error: {:?}",
                        error
                    );
                    send_storage_synchronizer_error(
                        error_notification_sender.clone(),
                        notification_id,
                        error,
                    )
                    .await;
                    decrement_pending_data_chunks(pending_transaction_chunks.clone());
                }
            }
        }
    };

    // Spawn the executor
    spawn(runtime, executor)
}

/// Spawns a dedicated committer that commits executed (but pending) chunks
fn spawn_committer<
    ChunkExecutor: ChunkExecutorTrait + 'static,
    MempoolNotifier: MempoolNotificationSender,
>(
    chunk_executor: Arc<ChunkExecutor>,
    mut committer_listener: mpsc::Receiver<NotificationId>,
    error_notification_sender: mpsc::UnboundedSender<ErrorNotification>,
    event_subscription_service: Arc<Mutex<EventSubscriptionService>>,
    mempool_notification_handler: MempoolNotificationHandler<MempoolNotifier>,
    pending_transaction_chunks: Arc<AtomicU64>,
    runtime: Option<Handle>,
    storage: Arc<dyn DbReader>,
) -> JoinHandle<()> {
    // Create a committer
    let committer = async move {
        while let Some(notification_id) = committer_listener.next().await {
            // Commit the executed chunk
            let timer = metrics::start_timer(
                &metrics::STORAGE_SYNCHRONIZER_LATENCIES,
                metrics::STORAGE_SYNCHRONIZER_COMMIT_CHUNK,
            );
            let chunk_executor_clone = chunk_executor.clone();
            let commit_fut = move || chunk_executor_clone.commit_chunk();
            // `spawn_blocking` so that the heavy synchronous function call doesn't
            // block the async thread.
            let result = tokio::task::spawn_blocking(commit_fut)
                .await
                .expect("spawn_blocking(commit_chunk) failed.");
            match result {
                Ok(notification) => {
                    // Log the event and update the metrics
                    info!(
                        LogSchema::new(LogEntry::StorageSynchronizer).message(&format!(
                            "Committed a new transaction chunk! \
                                    Transaction total: {:?}, event total: {:?}",
                            notification.committed_transactions.len(),
                            notification.committed_events.len()
                        ))
                    );
                    metrics::increment_gauge(
                        &metrics::STORAGE_SYNCHRONIZER_OPERATIONS,
                        metrics::StorageSynchronizerOperations::Synced.get_label(),
                        notification.committed_transactions.len() as u64,
                    );
                    if notification.reconfiguration_occurred {
                        utils::update_new_epoch_metrics();
                    }

                    // Handle the committed transaction notification (e.g., notify mempool).
                    // We do this here due to synchronization issues with mempool and
                    // storage. See: https://github.com/aptos-labs/aptos-core/issues/553
                    let committed_transactions = CommittedTransactions {
                        events: notification.committed_events,
                        transactions: notification.committed_transactions,
                    };
                    utils::handle_committed_transactions(
                        committed_transactions,
                        storage.clone(),
                        mempool_notification_handler.clone(),
                        event_subscription_service.clone(),
                    )
                    .await;
                }
                Err(error) => {
                    let error = format!("Failed to commit executed chunk! Error: {:?}", error);
                    send_storage_synchronizer_error(
                        error_notification_sender.clone(),
                        notification_id,
                        error,
                    )
                    .await;
                }
            };
            drop(timer);
            decrement_pending_data_chunks(pending_transaction_chunks.clone());
        }
    };

    // Spawn the committer
    spawn(runtime, committer)
}

/// Spawns a dedicated receiver that commits state values from a state snapshot
fn spawn_state_snapshot_receiver<
    ChunkExecutor: ChunkExecutorTrait + 'static,
    MetadataStorage: MetadataStorageInterface + Clone + Send + Sync + 'static,
>(
    chunk_executor: Arc<ChunkExecutor>,
    mut state_snapshot_listener: mpsc::Receiver<StorageDataChunk>,
    mut commit_notification_sender: mpsc::UnboundedSender<CommitNotification>,
    error_notification_sender: mpsc::UnboundedSender<ErrorNotification>,
    pending_transaction_chunks: Arc<AtomicU64>,
    metadata_storage: MetadataStorage,
    storage: DbReaderWriter,
    epoch_change_proofs: Vec<LedgerInfoWithSignatures>,
    target_ledger_info: LedgerInfoWithSignatures,
    target_output_with_proof: TransactionOutputListWithProof,
    runtime: Option<Handle>,
) -> JoinHandle<()> {
    // Create a state snapshot receiver
    let receiver = async move {
        // Get the target version and expected root hash
        let version = target_ledger_info.ledger_info().version();
        let expected_root_hash = target_output_with_proof
            .proof
            .transaction_infos
            .first()
            .expect("Target transaction info should exist!")
            .ensure_state_checkpoint_hash()
            .expect("Must be at state checkpoint.");

        // Create the snapshot receiver
        let mut state_snapshot_receiver = storage
            .writer
            .get_state_snapshot_receiver(version, expected_root_hash)
            .expect("Failed to initialize the state snapshot receiver!");

        // Handle state value chunks
        let target_ledger_info = &target_ledger_info;
        while let Some(storage_data_chunk) = state_snapshot_listener.next().await {
            // Process the chunk
            match storage_data_chunk {
                StorageDataChunk::States(notification_id, states_with_proof) => {
                    let all_states_synced = states_with_proof.is_last_chunk();
                    let last_committed_state_index = states_with_proof.last_index;

                    // Attempt to commit the chunk
                    let num_state_values = states_with_proof.raw_values.len();
                    let commit_result = state_snapshot_receiver.add_chunk(
                        states_with_proof.raw_values,
                        states_with_proof.proof.clone(),
                    );
                    match commit_result {
                        Ok(()) => {
                            // Update the logs and metrics
                            info!(
                                LogSchema::new(LogEntry::StorageSynchronizer).message(&format!(
                                    "Committed a new state value chunk! Chunk size: {:?}, last persisted index: {:?}",
                                    num_state_values,
                                    last_committed_state_index
                                ))
                            );
                            metrics::set_gauge(
                                &metrics::STORAGE_SYNCHRONIZER_OPERATIONS,
                                metrics::StorageSynchronizerOperations::SyncedStates.get_label(),
                                last_committed_state_index as u64,
                            );

                            if !all_states_synced {
                                // Update the metadata storage with the last committed state index
                                if let Err(error) = metadata_storage
                                    .clone()
                                    .update_last_persisted_state_value_index(
                                        target_ledger_info,
                                        last_committed_state_index,
                                        all_states_synced,
                                    )
                                {
                                    let error = format!("Failed to update the last persisted state index at version: {:?}! Error: {:?}", version, error);
                                    send_storage_synchronizer_error(
                                        error_notification_sender.clone(),
                                        notification_id,
                                        error,
                                    )
                                    .await;
                                }
                                decrement_pending_data_chunks(pending_transaction_chunks.clone());
                                continue; // Wait for the next chunk
                            }

                            // Finalize storage and send a commit notification
                            if let Err(error) = finalize_storage_and_send_commit(
                                chunk_executor,
                                &mut commit_notification_sender,
                                metadata_storage,
                                state_snapshot_receiver,
                                storage,
                                &epoch_change_proofs,
                                target_output_with_proof,
                                version,
                                target_ledger_info,
                                last_committed_state_index,
                            )
                            .await
                            {
                                send_storage_synchronizer_error(
                                    error_notification_sender.clone(),
                                    notification_id,
                                    error,
                                )
                                .await;
                            }
                            decrement_pending_data_chunks(pending_transaction_chunks.clone());
                            return; // There's nothing left to do!
                        }
                        Err(error) => {
                            let error =
                                format!("Failed to commit state value chunk! Error: {:?}", error);
                            send_storage_synchronizer_error(
                                error_notification_sender.clone(),
                                notification_id,
                                error,
                            )
                            .await;
                        }
                    }
                }
                storage_data_chunk => {
                    panic!(
                        "Invalid storage data chunk sent to state snapshot receiver: {:?}",
                        storage_data_chunk
                    );
                }
            }
            decrement_pending_data_chunks(pending_transaction_chunks.clone());
        }
    };

    // Spawn the receiver
    spawn(runtime, receiver)
}

/// Finalizes storage once all state values have been committed
/// and sends a commit notification to the driver.
async fn finalize_storage_and_send_commit<
    'receiver_lifetime, // Required because of https://github.com/rust-lang/rust/issues/63033
    ChunkExecutor: ChunkExecutorTrait + 'static,
    MetadataStorage: MetadataStorageInterface + Clone + Send + Sync + 'static,
>(
    chunk_executor: Arc<ChunkExecutor>,
    commit_notification_sender: &mut UnboundedSender<CommitNotification>,
    metadata_storage: MetadataStorage,
    state_snapshot_receiver: Box<
        dyn StateSnapshotReceiver<StateKey, StateValue> + 'receiver_lifetime,
    >,
    storage: DbReaderWriter,
    epoch_change_proofs: &[LedgerInfoWithSignatures],
    target_output_with_proof: TransactionOutputListWithProof,
    version: Version,
    target_ledger_info: &LedgerInfoWithSignatures,
    last_committed_state_index: u64,
) -> Result<(), String> {
    // Finalize the state snapshot
    state_snapshot_receiver.finish_box().map_err(|error| {
        format!(
            "Failed to finish the state value synchronization! Error: {:?}",
            error
        )
    })?;
    storage
        .writer
        .finalize_state_snapshot(
            version,
            target_output_with_proof.clone(),
            epoch_change_proofs,
        )
        .map_err(|error| format!("Failed to finalize the state snapshot! Error: {:?}", error))?;

    // Update the metadata storage
    metadata_storage.update_last_persisted_state_value_index(
            target_ledger_info,
            last_committed_state_index,
            true,
        ).map_err(|error| {
        format!("All states have synced, but failed to update the metadata storage at version {:?}! Error: {:?}", version, error)
    })?;

    // Reset the chunk executor
    chunk_executor.reset().map_err(|error| {
        format!(
            "Failed to reset the chunk executor after state snapshot synchronization! Error: {:?}",
            error
        )
    })?;

    // Create and send the commit notification
    let commit_notification = create_commit_notification(
        &target_output_with_proof,
        last_committed_state_index,
        version,
    );
    commit_notification_sender
        .send(commit_notification)
        .await
        .map_err(|error| {
            format!(
                "Failed to send the final state commit notification! Error: {:?}",
                error
            )
        })?;

    // Update the counters
    utils::initialize_sync_gauges(storage.reader).map_err(|error| {
        format!(
            "Failed to initialize the state sync version gauges! Error: {:?}",
            error
        )
    })?;

    Ok(())
}

/// Creates a commit notification for the new committed state snapshot
fn create_commit_notification(
    target_output_with_proof: &TransactionOutputListWithProof,
    last_committed_state_index: u64,
    version: u64,
) -> CommitNotification {
    let (transactions, outputs): (Vec<Transaction>, Vec<TransactionOutput>) =
        target_output_with_proof
            .transactions_and_outputs
            .clone()
            .into_iter()
            .unzip();
    let events = outputs
        .into_iter()
        .flat_map(|output| output.events().to_vec())
        .collect::<Vec<_>>();
    CommitNotification::new_committed_state_snapshot(
        events,
        transactions,
        last_committed_state_index,
        version,
    )
}

/// Spawns a future on a specified runtime. If no runtime is specified, uses
/// the current runtime.
fn spawn(
    runtime: Option<Handle>,
    future: impl Future<Output = ()> + Send + 'static,
) -> JoinHandle<()> {
    if let Some(runtime) = runtime {
        runtime.spawn(future)
    } else {
        tokio::spawn(future)
    }
}

/// Returns the value currently held by the pending chunk counter
fn load_pending_data_chunks(pending_data_chunks: Arc<AtomicU64>) -> u64 {
    pending_data_chunks.load(Ordering::Relaxed)
}

/// Increments the pending data chunks
fn increment_pending_data_chunks(pending_data_chunks: Arc<AtomicU64>) {
    let delta = 1;
    pending_data_chunks.fetch_add(delta, Ordering::Relaxed);
    metrics::increment_gauge(
        &metrics::STORAGE_SYNCHRONIZER_GAUGES,
        metrics::STORAGE_SYNCHRONIZER_PENDING_DATA,
        delta,
    );
}

/// Decrements the pending data chunks
fn decrement_pending_data_chunks(atomic_u64: Arc<AtomicU64>) {
    let delta = 1;
    atomic_u64.fetch_sub(delta, Ordering::Relaxed);
    metrics::decrement_gauge(
        &metrics::STORAGE_SYNCHRONIZER_GAUGES,
        metrics::STORAGE_SYNCHRONIZER_PENDING_DATA,
        delta,
    );
}

/// Sends an error notification to the notification listener
async fn send_storage_synchronizer_error(
    mut error_notification_sender: mpsc::UnboundedSender<ErrorNotification>,
    notification_id: NotificationId,
    error_message: String,
) {
    let error_message = format!("Storage synchronizer error: {:?}", error_message);
    error!(LogSchema::new(LogEntry::StorageSynchronizer).message(&error_message));

    // Send an error notification
    let error = Error::UnexpectedError(error_message);
    let error_notification = ErrorNotification {
        error: error.clone(),
        notification_id,
    };
    if let Err(error) = error_notification_sender.send(error_notification).await {
        panic!("Failed to send error notification! Error: {:?}", error);
    }

    // Update the metrics
    metrics::increment_counter(&metrics::STORAGE_SYNCHRONIZER_ERRORS, error.get_label());
}
