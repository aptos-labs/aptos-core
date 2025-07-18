// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::Error,
    logging::{LogEntry, LogSchema},
    metadata_storage::MetadataStorageInterface,
    metrics,
    notification_handlers::{
        CommitNotification, CommittedTransactions, ErrorNotification, MempoolNotificationHandler,
        StorageServiceNotificationHandler,
    },
    utils,
};
use aptos_config::config::StateSyncDriverConfig;
use aptos_data_streaming_service::data_notification::NotificationId;
use aptos_event_notifications::EventSubscriptionService;
use aptos_executor_types::{ChunkCommitNotification, ChunkExecutorTrait};
use aptos_infallible::Mutex;
use aptos_logger::prelude::*;
use aptos_mempool_notifications::MempoolNotificationSender;
use aptos_metrics_core::HistogramTimer;
use aptos_storage_interface::{DbReader, DbReaderWriter, StateSnapshotReceiver};
use aptos_storage_service_notifications::StorageServiceNotificationSender;
use aptos_types::{
    ledger_info::LedgerInfoWithSignatures,
    state_store::{
        state_key::StateKey,
        state_value::{StateValue, StateValueChunkWithProof},
    },
    transaction::{
        Transaction, TransactionListWithProofV2, TransactionOutput,
        TransactionOutputListWithProofV2, Version,
    },
};
use async_trait::async_trait;
use futures::{channel::mpsc, SinkExt, StreamExt};
use std::{
    future::Future,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Instant,
};
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
        notification_metadata: NotificationMetadata,
        output_list_with_proof: TransactionOutputListWithProofV2,
        target_ledger_info: LedgerInfoWithSignatures,
        end_of_epoch_ledger_info: Option<LedgerInfoWithSignatures>,
    ) -> Result<(), Error>;

    /// Executes a batch of transactions.
    ///
    /// Note: this assumes that the ledger infos have already been verified.
    async fn execute_transactions(
        &mut self,
        notification_metadata: NotificationMetadata,
        transaction_list_with_proof: TransactionListWithProofV2,
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
        target_output_with_proof: TransactionOutputListWithProofV2,
    ) -> Result<JoinHandle<()>, Error>;

    /// Returns true iff there is storage data that is still waiting
    /// to be executed/applied or committed.
    fn pending_storage_data(&self) -> bool;

    /// Saves the given state values to storage.
    ///
    /// Note: this requires that `initialize_state_synchronizer` has been
    /// called.
    async fn save_state_values(
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

/// A simple struct that holds metadata related to data notifications
#[derive(Copy, Clone, Debug)]
pub struct NotificationMetadata {
    pub creation_time: Instant,
    pub notification_id: NotificationId,
}

impl NotificationMetadata {
    pub fn new(creation_time: Instant, notification_id: NotificationId) -> Self {
        Self {
            creation_time,
            notification_id,
        }
    }

    #[cfg(test)]
    /// Returns a new metadata struct for test purposes
    pub fn new_for_test(notification_id: NotificationId) -> Self {
        Self::new(Instant::now(), notification_id)
    }
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
    pub fn new<
        MempoolNotifier: MempoolNotificationSender,
        StorageServiceNotifier: StorageServiceNotificationSender,
    >(
        driver_config: StateSyncDriverConfig,
        chunk_executor: Arc<ChunkExecutor>,
        commit_notification_sender: mpsc::UnboundedSender<CommitNotification>,
        error_notification_sender: mpsc::UnboundedSender<ErrorNotification>,
        event_subscription_service: Arc<Mutex<EventSubscriptionService>>,
        mempool_notification_handler: MempoolNotificationHandler<MempoolNotifier>,
        storage_service_notification_handler: StorageServiceNotificationHandler<
            StorageServiceNotifier,
        >,
        metadata_storage: MetadataStorage,
        storage: DbReaderWriter,
        runtime: Option<&Runtime>,
    ) -> (Self, StorageSynchronizerHandles) {
        // Create a channel to notify the executor when data chunks are ready
        let max_pending_data_chunks = driver_config.max_pending_data_chunks as usize;
        let (executor_notifier, executor_listener) = mpsc::channel(max_pending_data_chunks);

        // Create a channel to notify the ledger updater when executed chunks are ready
        let (ledger_updater_notifier, ledger_updater_listener) =
            mpsc::channel(max_pending_data_chunks);

        // Create a channel to notify the committer when the ledger has been updated
        let (committer_notifier, committer_listener) = mpsc::channel(max_pending_data_chunks);

        // Create a channel to notify the commit post-processor when a chunk has been committed
        let (commit_post_processor_notifier, commit_post_processor_listener) =
            mpsc::channel(max_pending_data_chunks);

        // Create a shared pending data chunk counter
        let pending_data_chunks = Arc::new(AtomicU64::new(0));

        // Spawn the executor that executes/applies storage data chunks
        let runtime = runtime.map(|runtime| runtime.handle().clone());
        let executor_handle = spawn_executor(
            chunk_executor.clone(),
            error_notification_sender.clone(),
            executor_listener,
            ledger_updater_notifier,
            pending_data_chunks.clone(),
            runtime.clone(),
        );

        // Spawn the ledger updater that updates the ledger in storage
        let ledger_updater_handle = spawn_ledger_updater(
            chunk_executor.clone(),
            error_notification_sender.clone(),
            ledger_updater_listener,
            committer_notifier,
            pending_data_chunks.clone(),
            runtime.clone(),
        );

        // Spawn the committer that commits executed (but pending) chunks
        let committer_handle = spawn_committer(
            chunk_executor.clone(),
            error_notification_sender.clone(),
            committer_listener,
            commit_post_processor_notifier,
            pending_data_chunks.clone(),
            runtime.clone(),
            storage.reader.clone(),
        );

        // Spawn the commit post-processor that handles commit notifications
        let commit_post_processor_handle = spawn_commit_post_processor(
            commit_post_processor_listener,
            event_subscription_service,
            mempool_notification_handler,
            storage_service_notification_handler,
            pending_data_chunks.clone(),
            runtime.clone(),
            storage.reader.clone(),
        );

        // Initialize the metric gauges
        utils::initialize_sync_gauges(storage.reader.clone())
            .expect("Failed to initialize the metric gauges!");

        // Create the storage synchronizer
        let storage_synchronizer = Self {
            chunk_executor,
            commit_notification_sender,
            driver_config,
            error_notification_sender,
            executor_notifier,
            pending_data_chunks,
            metadata_storage,
            runtime,
            state_snapshot_notifier: None,
            storage,
        };

        // Create the storage synchronizer handles
        let storage_synchronizer_handles = StorageSynchronizerHandles {
            executor: executor_handle,
            ledger_updater: ledger_updater_handle,
            committer: committer_handle,
            commit_post_processor: commit_post_processor_handle,
        };

        (storage_synchronizer, storage_synchronizer_handles)
    }

    /// Notifies the executor of new data chunks
    async fn notify_executor(&mut self, storage_data_chunk: StorageDataChunk) -> Result<(), Error> {
        if let Err(error) = send_and_monitor_backpressure(
            &mut self.executor_notifier,
            metrics::STORAGE_SYNCHRONIZER_EXECUTOR,
            storage_data_chunk,
        )
        .await
        {
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
        notification_metadata: NotificationMetadata,
        output_list_with_proof: TransactionOutputListWithProofV2,
        target_ledger_info: LedgerInfoWithSignatures,
        end_of_epoch_ledger_info: Option<LedgerInfoWithSignatures>,
    ) -> Result<(), Error> {
        // Update the metrics for the data notification apply latency
        metrics::observe_duration(
            &metrics::DATA_NOTIFICATION_LATENCIES,
            metrics::NOTIFICATION_CREATE_TO_APPLY,
            notification_metadata.creation_time,
        );

        // Notify the executor of the new transaction output chunk
        let storage_data_chunk = StorageDataChunk::TransactionOutputs(
            notification_metadata,
            output_list_with_proof,
            target_ledger_info,
            end_of_epoch_ledger_info,
        );
        self.notify_executor(storage_data_chunk).await
    }

    async fn execute_transactions(
        &mut self,
        notification_metadata: NotificationMetadata,
        transaction_list_with_proof: TransactionListWithProofV2,
        target_ledger_info: LedgerInfoWithSignatures,
        end_of_epoch_ledger_info: Option<LedgerInfoWithSignatures>,
    ) -> Result<(), Error> {
        // Update the metrics for the data notification execute latency
        metrics::observe_duration(
            &metrics::DATA_NOTIFICATION_LATENCIES,
            metrics::NOTIFICATION_CREATE_TO_EXECUTE,
            notification_metadata.creation_time,
        );

        // Notify the executor of the new transaction chunk
        let storage_data_chunk = StorageDataChunk::Transactions(
            notification_metadata,
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
        target_output_with_proof: TransactionOutputListWithProofV2,
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

    async fn save_state_values(
        &mut self,
        notification_id: NotificationId,
        state_value_chunk_with_proof: StateValueChunkWithProof,
    ) -> Result<(), Error> {
        // Get the snapshot notifier and create the storage data chunk
        let state_snapshot_notifier = self.state_snapshot_notifier.as_mut().ok_or_else(|| {
            Error::UnexpectedError("The state snapshot receiver has not been initialized!".into())
        })?;
        let storage_data_chunk =
            StorageDataChunk::States(notification_id, state_value_chunk_with_proof);

        // Notify the snapshot receiver of the storage data chunk
        if let Err(error) = send_and_monitor_backpressure(
            state_snapshot_notifier,
            metrics::STORAGE_SYNCHRONIZER_STATE_SNAPSHOT_RECEIVER,
            storage_data_chunk,
        )
        .await
        {
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

/// A simple container that holds the handles to the spawned storage synchronizer threads
#[allow(dead_code)]
pub struct StorageSynchronizerHandles {
    pub executor: JoinHandle<()>,
    pub ledger_updater: JoinHandle<()>,
    pub committer: JoinHandle<()>,
    pub commit_post_processor: JoinHandle<()>,
}

/// A chunk of data to be executed and/or committed to storage (i.e., states,
/// transactions or outputs).
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug)]
enum StorageDataChunk {
    States(NotificationId, StateValueChunkWithProof),
    Transactions(
        NotificationMetadata,
        TransactionListWithProofV2,
        LedgerInfoWithSignatures,
        Option<LedgerInfoWithSignatures>,
    ),
    TransactionOutputs(
        NotificationMetadata,
        TransactionOutputListWithProofV2,
        LedgerInfoWithSignatures,
        Option<LedgerInfoWithSignatures>,
    ),
}

/// Spawns a dedicated executor that executes/applies storage data chunks
fn spawn_executor<ChunkExecutor: ChunkExecutorTrait + 'static>(
    chunk_executor: Arc<ChunkExecutor>,
    error_notification_sender: mpsc::UnboundedSender<ErrorNotification>,
    mut executor_listener: mpsc::Receiver<StorageDataChunk>,
    mut ledger_updater_notifier: mpsc::Sender<NotificationMetadata>,
    pending_data_chunks: Arc<AtomicU64>,
    runtime: Option<Handle>,
) -> JoinHandle<()> {
    // Create an executor
    let executor = async move {
        while let Some(storage_data_chunk) = executor_listener.next().await {
            // Start the execute/apply timer
            let _timer = start_execute_apply_timer(&storage_data_chunk);

            // Execute/apply the storage data chunk
            let (notification_metadata, result, executed_chunk) = match storage_data_chunk {
                StorageDataChunk::Transactions(
                    notification_metadata,
                    transactions_with_proof,
                    target_ledger_info,
                    end_of_epoch_ledger_info,
                ) => {
                    // Execute the storage data chunk
                    let result = execute_transaction_chunk(
                        chunk_executor.clone(),
                        transactions_with_proof,
                        target_ledger_info,
                        end_of_epoch_ledger_info,
                    )
                    .await;
                    (notification_metadata, result, true)
                },
                StorageDataChunk::TransactionOutputs(
                    notification_metadata,
                    outputs_with_proof,
                    target_ledger_info,
                    end_of_epoch_ledger_info,
                ) => {
                    // Apply the storage data chunk
                    let result = apply_output_chunk(
                        chunk_executor.clone(),
                        outputs_with_proof,
                        target_ledger_info,
                        end_of_epoch_ledger_info,
                    )
                    .await;
                    (notification_metadata, result, false)
                },
                storage_data_chunk => {
                    unreachable!(
                        "Invalid data chunk sent to executor! This shouldn't happen: {:?}",
                        storage_data_chunk
                    );
                },
            };

            // Notify the ledger updater of the new executed/applied chunks
            match result {
                Ok(()) => {
                    // Update the metrics for the data notification ledger update latency
                    metrics::observe_duration(
                        &metrics::DATA_NOTIFICATION_LATENCIES,
                        metrics::NOTIFICATION_CREATE_TO_UPDATE_LEDGER,
                        notification_metadata.creation_time,
                    );

                    // Notify the ledger updater
                    if let Err(error) = send_and_monitor_backpressure(
                        &mut ledger_updater_notifier,
                        metrics::STORAGE_SYNCHRONIZER_LEDGER_UPDATER,
                        notification_metadata,
                    )
                    .await
                    {
                        // Send an error notification to the driver (we failed to notify the ledger updater)
                        let error =
                            format!("Failed to notify the ledger updater! Error: {:?}", error);
                        handle_storage_synchronizer_error(
                            notification_metadata,
                            error,
                            &error_notification_sender,
                            &pending_data_chunks,
                        )
                        .await;
                    }
                },
                Err(error) => {
                    // Send an error notification to the driver (we failed to execute/apply the chunk)
                    let error = if executed_chunk {
                        format!("Failed to execute the data chunk! Error: {:?}", error)
                    } else {
                        format!("Failed to apply the data chunk! Error: {:?}", error)
                    };
                    handle_storage_synchronizer_error(
                        notification_metadata,
                        error,
                        &error_notification_sender,
                        &pending_data_chunks,
                    )
                    .await;
                },
            }
        }
    };

    // Spawn the executor
    spawn(runtime, executor)
}

/// Starts the timer for the execute/apply phase of the storage synchronizer
fn start_execute_apply_timer(storage_data_chunk: &StorageDataChunk) -> HistogramTimer {
    // Get the timer label
    let label = match storage_data_chunk {
        StorageDataChunk::Transactions(_, _, _, _) => metrics::STORAGE_SYNCHRONIZER_EXECUTE_CHUNK,
        StorageDataChunk::TransactionOutputs(_, _, _, _) => {
            metrics::STORAGE_SYNCHRONIZER_APPLY_CHUNK
        },
        storage_data_chunk => unreachable!(
            "Invalid storage data chunk sent to executor! This shouldn't happen: {:?}",
            storage_data_chunk
        ),
    };

    // Start and return the timer
    metrics::start_timer(&metrics::STORAGE_SYNCHRONIZER_LATENCIES, label)
}

/// Spawns a dedicated updater that updates the ledger after chunk execution/application
fn spawn_ledger_updater<ChunkExecutor: ChunkExecutorTrait + 'static>(
    chunk_executor: Arc<ChunkExecutor>,
    error_notification_sender: mpsc::UnboundedSender<ErrorNotification>,
    mut ledger_updater_listener: mpsc::Receiver<NotificationMetadata>,
    mut committer_notifier: mpsc::Sender<NotificationMetadata>,
    pending_data_chunks: Arc<AtomicU64>,
    runtime: Option<Handle>,
) -> JoinHandle<()> {
    // Create a ledger updater
    let ledger_updater = async move {
        while let Some(notification_metadata) = ledger_updater_listener.next().await {
            // Start the update ledger timer
            let _timer = metrics::start_timer(
                &metrics::STORAGE_SYNCHRONIZER_LATENCIES,
                metrics::STORAGE_SYNCHRONIZER_UPDATE_LEDGER,
            );

            // Update the storage ledger
            let result = update_ledger(chunk_executor.clone()).await;

            // Notify the committer of the updated ledger
            match result {
                Ok(()) => {
                    // Log the successful ledger update
                    debug!(
                        LogSchema::new(LogEntry::StorageSynchronizer).message(&format!(
                            "Updated the ledger for notification ID {:?}!",
                            notification_metadata.notification_id,
                        ))
                    );

                    // Update the metrics for the data notification commit latency
                    metrics::observe_duration(
                        &metrics::DATA_NOTIFICATION_LATENCIES,
                        metrics::NOTIFICATION_CREATE_TO_COMMIT,
                        notification_metadata.creation_time,
                    );

                    // Notify the committer of the update
                    if let Err(error) = send_and_monitor_backpressure(
                        &mut committer_notifier,
                        metrics::STORAGE_SYNCHRONIZER_COMMITTER,
                        notification_metadata,
                    )
                    .await
                    {
                        // Send an error notification to the driver (we failed to notify the committer)
                        let error = format!("Failed to notify the committer! Error: {:?}", error);
                        handle_storage_synchronizer_error(
                            notification_metadata,
                            error,
                            &error_notification_sender,
                            &pending_data_chunks,
                        )
                        .await;
                    }
                },
                Err(error) => {
                    // Send an error notification to the driver (we failed to update the ledger)
                    let error = format!("Failed to update the ledger! Error: {:?}", error);
                    handle_storage_synchronizer_error(
                        notification_metadata,
                        error,
                        &error_notification_sender,
                        &pending_data_chunks,
                    )
                    .await;
                },
            };
        }
    };

    // Spawn the ledger updater
    spawn(runtime, ledger_updater)
}

/// Spawns a dedicated committer that commits executed (but pending) chunks
fn spawn_committer<ChunkExecutor: ChunkExecutorTrait + 'static>(
    chunk_executor: Arc<ChunkExecutor>,
    error_notification_sender: mpsc::UnboundedSender<ErrorNotification>,
    mut committer_listener: mpsc::Receiver<NotificationMetadata>,
    mut commit_post_processor_notifier: mpsc::Sender<ChunkCommitNotification>,
    pending_data_chunks: Arc<AtomicU64>,
    runtime: Option<Handle>,
    storage: Arc<dyn DbReader>,
) -> JoinHandle<()> {
    // Create a committer
    let committer = async move {
        while let Some(notification_metadata) = committer_listener.next().await {
            // Start the commit timer
            let _timer = metrics::start_timer(
                &metrics::STORAGE_SYNCHRONIZER_LATENCIES,
                metrics::STORAGE_SYNCHRONIZER_COMMIT_CHUNK,
            );

            // Commit the executed chunk
            let result = commit_chunk(chunk_executor.clone()).await;

            // Notify the commit post-processor of the committed chunk
            match result {
                Ok(notification) => {
                    // Log the successful commit
                    info!(
                        LogSchema::new(LogEntry::StorageSynchronizer).message(&format!(
                            "Committed a new transaction chunk! \
                                    Transaction total: {:?}, event total: {:?}",
                            notification.committed_transactions.len(),
                            notification.subscribable_events.len()
                        ))
                    );

                    // Update the synced version metrics
                    utils::update_new_synced_metrics(
                        storage.clone(),
                        notification.committed_transactions.len(),
                    );

                    // Update the synced epoch metrics
                    let reconfiguration_occurred = notification.reconfiguration_occurred;
                    utils::update_new_epoch_metrics(storage.clone(), reconfiguration_occurred);

                    // Update the metrics for the data notification commit post-process latency
                    metrics::observe_duration(
                        &metrics::DATA_NOTIFICATION_LATENCIES,
                        metrics::NOTIFICATION_CREATE_TO_COMMIT_POST_PROCESS,
                        notification_metadata.creation_time,
                    );

                    // Notify the commit post-processor of the committed chunk
                    if let Err(error) = send_and_monitor_backpressure(
                        &mut commit_post_processor_notifier,
                        metrics::STORAGE_SYNCHRONIZER_COMMIT_POST_PROCESSOR,
                        notification,
                    )
                    .await
                    {
                        // Send an error notification to the driver (we failed to notify the commit post-processor)
                        let error = format!(
                            "Failed to notify the commit post-processor! Error: {:?}",
                            error
                        );
                        handle_storage_synchronizer_error(
                            notification_metadata,
                            error,
                            &error_notification_sender,
                            &pending_data_chunks,
                        )
                        .await;
                    }
                },
                Err(error) => {
                    // Send an error notification to the driver (we failed to commit the chunk)
                    let error = format!("Failed to commit executed chunk! Error: {:?}", error);
                    handle_storage_synchronizer_error(
                        notification_metadata,
                        error,
                        &error_notification_sender,
                        &pending_data_chunks,
                    )
                    .await;
                },
            };
        }
    };

    // Spawn the committer
    spawn(runtime, committer)
}

/// Spawns a dedicated commit post-processor that handles commit notifications
fn spawn_commit_post_processor<
    MempoolNotifier: MempoolNotificationSender,
    StorageServiceNotifier: StorageServiceNotificationSender,
>(
    mut commit_post_processor_listener: mpsc::Receiver<ChunkCommitNotification>,
    event_subscription_service: Arc<Mutex<EventSubscriptionService>>,
    mempool_notification_handler: MempoolNotificationHandler<MempoolNotifier>,
    storage_service_notification_handler: StorageServiceNotificationHandler<StorageServiceNotifier>,
    pending_data_chunks: Arc<AtomicU64>,
    runtime: Option<Handle>,
    storage: Arc<dyn DbReader>,
) -> JoinHandle<()> {
    // Create a commit post-processor
    let commit_post_processor = async move {
        while let Some(notification) = commit_post_processor_listener.next().await {
            // Start the commit post-process timer
            let _timer = metrics::start_timer(
                &metrics::STORAGE_SYNCHRONIZER_LATENCIES,
                metrics::STORAGE_SYNCHRONIZER_COMMIT_POST_PROCESS,
            );

            // Handle the committed transaction notification (e.g., notify mempool)
            let committed_transactions = CommittedTransactions {
                events: notification.subscribable_events,
                transactions: notification.committed_transactions,
            };
            utils::handle_committed_transactions(
                committed_transactions,
                storage.clone(),
                mempool_notification_handler.clone(),
                event_subscription_service.clone(),
                storage_service_notification_handler.clone(),
            )
            .await;
            decrement_pending_data_chunks(pending_data_chunks.clone());
        }
    };

    // Spawn the commit post-processor
    spawn(runtime, commit_post_processor)
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
    pending_data_chunks: Arc<AtomicU64>,
    metadata_storage: MetadataStorage,
    storage: DbReaderWriter,
    epoch_change_proofs: Vec<LedgerInfoWithSignatures>,
    target_ledger_info: LedgerInfoWithSignatures,
    target_output_with_proof: TransactionOutputListWithProofV2,
    runtime: Option<Handle>,
) -> JoinHandle<()> {
    // Create a state snapshot receiver
    let receiver = async move {
        // Get the target version and expected root hash
        let version = target_ledger_info.ledger_info().version();
        let expected_root_hash = target_output_with_proof
            .get_output_list_with_proof()
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
        while let Some(storage_data_chunk) = state_snapshot_listener.next().await {
            // Start the snapshot timer for the state value chunk
            let _timer = metrics::start_timer(
                &metrics::STORAGE_SYNCHRONIZER_LATENCIES,
                metrics::STORAGE_SYNCHRONIZER_STATE_VALUE_CHUNK,
            );

            // Commit the state value chunk
            match storage_data_chunk {
                StorageDataChunk::States(notification_id, states_with_proof) => {
                    // Commit the state value chunk
                    let all_states_synced = states_with_proof.is_last_chunk();
                    let last_committed_state_index = states_with_proof.last_index;
                    let num_state_values = states_with_proof.raw_values.len();

                    let result = state_snapshot_receiver.add_chunk(
                        states_with_proof.raw_values,
                        states_with_proof.proof.clone(),
                    );

                    // Handle the commit result
                    match result {
                        Ok(()) => {
                            // Update the logs and metrics
                            info!(
                                LogSchema::new(LogEntry::StorageSynchronizer).message(&format!(
                                    "Committed a new state value chunk! Chunk size: {:?}, last persisted index: {:?}",
                                    num_state_values,
                                    last_committed_state_index
                                ))
                            );

                            // Update the chunk metrics
                            let operation_label =
                                metrics::StorageSynchronizerOperations::SyncedStates.get_label();
                            metrics::set_gauge(
                                &metrics::STORAGE_SYNCHRONIZER_OPERATIONS,
                                operation_label,
                                last_committed_state_index,
                            );
                            metrics::observe_value(
                                &metrics::STORAGE_SYNCHRONIZER_CHUNK_SIZES,
                                operation_label,
                                num_state_values as u64,
                            );

                            if !all_states_synced {
                                // Update the metadata storage with the last committed state index
                                if let Err(error) = metadata_storage
                                    .clone()
                                    .update_last_persisted_state_value_index(
                                        &target_ledger_info,
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
                                decrement_pending_data_chunks(pending_data_chunks.clone());
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
                                &target_ledger_info,
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
                            decrement_pending_data_chunks(pending_data_chunks.clone());
                            return; // There's nothing left to do!
                        },
                        Err(error) => {
                            let error =
                                format!("Failed to commit state value chunk! Error: {:?}", error);
                            send_storage_synchronizer_error(
                                error_notification_sender.clone(),
                                notification_id,
                                error,
                            )
                            .await;
                        },
                    }
                },
                storage_data_chunk => {
                    unimplemented!(
                        "Invalid storage data chunk sent to state snapshot receiver! This shouldn't happen: {:?}",
                        storage_data_chunk
                    );
                },
            }
            decrement_pending_data_chunks(pending_data_chunks.clone());
        }
    };

    // Spawn the receiver
    spawn(runtime, receiver)
}

/// Spawns a dedicated task that applies the given output chunk. We use
/// `spawn_blocking` so that the heavy synchronous function doesn't
/// block the async thread.
async fn apply_output_chunk<ChunkExecutor: ChunkExecutorTrait + 'static>(
    chunk_executor: Arc<ChunkExecutor>,
    outputs_with_proof: TransactionOutputListWithProofV2,
    target_ledger_info: LedgerInfoWithSignatures,
    end_of_epoch_ledger_info: Option<LedgerInfoWithSignatures>,
) -> anyhow::Result<()> {
    // Apply the output chunk
    let num_outputs = outputs_with_proof.get_num_outputs();
    let result = tokio::task::spawn_blocking(move || {
        chunk_executor.enqueue_chunk_by_transaction_outputs(
            outputs_with_proof,
            &target_ledger_info,
            end_of_epoch_ledger_info.as_ref(),
        )
    })
    .await
    .expect("Spawn_blocking(apply_output_chunk) failed!");

    // Update the logs and metrics if the chunk was applied successfully
    if result.is_ok() {
        // Log the application event
        info!(
            LogSchema::new(LogEntry::StorageSynchronizer).message(&format!(
                "Applied a new transaction output chunk! Transaction total: {:?}.",
                num_outputs
            ))
        );

        // Update the chunk metrics
        let operation_label =
            metrics::StorageSynchronizerOperations::AppliedTransactionOutputs.get_label();
        update_synchronizer_chunk_metrics(num_outputs, operation_label);
    }

    result
}

/// Spawns a dedicated task that executes the given transaction chunk.
/// We use `spawn_blocking` so that the heavy synchronous function
/// doesn't block the async thread.
async fn execute_transaction_chunk<ChunkExecutor: ChunkExecutorTrait + 'static>(
    chunk_executor: Arc<ChunkExecutor>,
    transactions_with_proof: TransactionListWithProofV2,
    target_ledger_info: LedgerInfoWithSignatures,
    end_of_epoch_ledger_info: Option<LedgerInfoWithSignatures>,
) -> anyhow::Result<()> {
    // Execute the transaction chunk
    let num_transactions = transactions_with_proof
        .get_transaction_list_with_proof()
        .transactions
        .len();
    let result = tokio::task::spawn_blocking(move || {
        chunk_executor.enqueue_chunk_by_execution(
            transactions_with_proof,
            &target_ledger_info,
            end_of_epoch_ledger_info.as_ref(),
        )
    })
    .await
    .expect("Spawn_blocking(execute_transaction_chunk) failed!");

    // Update the logs and metrics if the chunk was executed successfully
    if result.is_ok() {
        // Log the execution event
        info!(
            LogSchema::new(LogEntry::StorageSynchronizer).message(&format!(
                "Executed a new transaction chunk! Transaction total: {:?}.",
                num_transactions
            ))
        );

        // Update the chunk metrics
        let operation_label =
            metrics::StorageSynchronizerOperations::ExecutedTransactions.get_label();
        update_synchronizer_chunk_metrics(num_transactions, operation_label);
    }

    result
}

/// Updates the storage synchronizer chunk metrics
fn update_synchronizer_chunk_metrics(num_items: usize, operation_label: &str) {
    metrics::increment_gauge(
        &metrics::STORAGE_SYNCHRONIZER_OPERATIONS,
        operation_label,
        num_items as u64,
    );
    metrics::observe_value(
        &metrics::STORAGE_SYNCHRONIZER_CHUNK_SIZES,
        operation_label,
        num_items as u64,
    );
}

/// Spawns a dedicated task that updates the ledger in storage. We use
/// `spawn_blocking` so that the heavy synchronous function doesn't
/// block the async thread.
async fn update_ledger<ChunkExecutor: ChunkExecutorTrait + 'static>(
    chunk_executor: Arc<ChunkExecutor>,
) -> anyhow::Result<()> {
    tokio::task::spawn_blocking(move || chunk_executor.update_ledger())
        .await
        .expect("Spawn_blocking(update_ledger) failed!")
}

/// Spawns a dedicated task that commits a data chunk. We use
/// `spawn_blocking` so that the heavy synchronous function doesn't
/// block the async thread.
async fn commit_chunk<ChunkExecutor: ChunkExecutorTrait + 'static>(
    chunk_executor: Arc<ChunkExecutor>,
) -> anyhow::Result<ChunkCommitNotification> {
    tokio::task::spawn_blocking(move || chunk_executor.commit_chunk())
        .await
        .expect("Spawn_blocking(commit_chunk) failed!")
}

/// Finalizes storage once all state values have been committed
/// and sends a commit notification to the driver.
async fn finalize_storage_and_send_commit<
    'receiver_lifetime, // Required because of https://github.com/rust-lang/rust/issues/63033
    ChunkExecutor: ChunkExecutorTrait + 'static,
    MetadataStorage: MetadataStorageInterface + Clone + Send + Sync + 'static,
>(
    chunk_executor: Arc<ChunkExecutor>,
    commit_notification_sender: &mut mpsc::UnboundedSender<CommitNotification>,
    metadata_storage: MetadataStorage,
    state_snapshot_receiver: Box<
        dyn StateSnapshotReceiver<StateKey, StateValue> + 'receiver_lifetime,
    >,
    storage: DbReaderWriter,
    epoch_change_proofs: &[LedgerInfoWithSignatures],
    target_output_with_proof: TransactionOutputListWithProofV2,
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

    info!("All states have synced, version: {}", version);

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
        target_output_with_proof,
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
    target_output_with_proof: TransactionOutputListWithProofV2,
    last_committed_state_index: u64,
    version: u64,
) -> CommitNotification {
    let (transactions, outputs): (Vec<Transaction>, Vec<TransactionOutput>) =
        target_output_with_proof
            .consume_output_list_with_proof()
            .transactions_and_outputs
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

/// Handles a storage synchronizer error by sending a notification to the driver
/// and decrementing the number of pending data chunks in the pipeline.
async fn handle_storage_synchronizer_error(
    notification_metadata: NotificationMetadata,
    error: String,
    error_notification_sender: &mpsc::UnboundedSender<ErrorNotification>,
    pending_data_chunks: &Arc<AtomicU64>,
) {
    // Send an error notification to the driver
    send_storage_synchronizer_error(
        error_notification_sender.clone(),
        notification_metadata.notification_id,
        error,
    )
    .await;

    // Decrement the number of pending data chunks
    decrement_pending_data_chunks(pending_data_chunks.clone());
}

/// Sends the given message along the specified channel, and monitors
/// if the channel hits backpressure (i.e., the channel is full).
async fn send_and_monitor_backpressure<T: Clone>(
    channel: &mut mpsc::Sender<T>,
    channel_label: &str,
    message: T,
) -> Result<(), Error> {
    match channel.try_send(message.clone()) {
        Ok(_) => Ok(()), // The message was sent successfully
        Err(error) => {
            // Otherwise, try_send failed. Handle the error.
            if error.is_full() {
                // The channel is full, log the backpressure and update the metrics.
                info!(
                    LogSchema::new(LogEntry::StorageSynchronizer).message(&format!(
                        "The {:?} channel is full! Backpressure will kick in!",
                        channel_label
                    ))
                );
                metrics::set_gauge(
                    &metrics::STORAGE_SYNCHRONIZER_PIPELINE_CHANNEL_BACKPRESSURE,
                    channel_label,
                    1, // We hit backpressure
                );

                // Call the blocking send (we still need to send the data chunk with backpressure)
                let result = channel.send(message).await.map_err(|error| {
                    Error::UnexpectedError(format!(
                        "Failed to send storage data chunk to: {:?}. Error: {:?}",
                        channel_label, error
                    ))
                });

                // Reset the gauge for the pipeline channel to inactive (we're done sending the message)
                metrics::set_gauge(
                    &metrics::STORAGE_SYNCHRONIZER_PIPELINE_CHANNEL_BACKPRESSURE,
                    channel_label,
                    0, // Backpressure is no longer active
                );

                result
            } else {
                // Otherwise, return the error (there's nothing else we can do)
                Err(Error::UnexpectedError(format!(
                    "Failed to try_send storage data chunk to {:?}. Error: {:?}",
                    channel_label, error
                )))
            }
        },
    }
}

/// Sends an error notification to the driver
async fn send_storage_synchronizer_error(
    mut error_notification_sender: mpsc::UnboundedSender<ErrorNotification>,
    notification_id: NotificationId,
    error_message: String,
) {
    // Log the storage synchronizer error
    let error_message = format!("Storage synchronizer error: {:?}", error_message);
    error!(LogSchema::new(LogEntry::StorageSynchronizer).message(&error_message));

    // Update the storage synchronizer error metrics
    let error = Error::UnexpectedError(error_message);
    metrics::increment_counter(&metrics::STORAGE_SYNCHRONIZER_ERRORS, error.get_label());

    // Send an error notification to the driver
    let error_notification = ErrorNotification {
        error: error.clone(),
        notification_id,
    };
    if let Err(error) = error_notification_sender.send(error_notification).await {
        error!(
            LogSchema::new(LogEntry::StorageSynchronizer).message(&format!(
                "Failed to send error notification! Error: {:?}",
                error
            ))
        );
    }
}
