// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::Error,
    logging::{LogEntry, LogSchema},
    metrics,
    notification_handlers::{
        CommitNotification, CommittedTransactions, ErrorNotification, MempoolNotificationHandler,
    },
    utils,
};
use aptos_config::config::StateSyncDriverConfig;
use aptos_infallible::Mutex;
use aptos_logger::prelude::*;
use aptos_types::{
    ledger_info::LedgerInfoWithSignatures,
    state_store::state_value::StateValueChunkWithProof,
    transaction::{
        Transaction, TransactionListWithProof, TransactionOutput, TransactionOutputListWithProof,
    },
};
use data_streaming_service::data_notification::NotificationId;
use event_notifications::EventSubscriptionService;
use executor_types::ChunkExecutorTrait;
use futures::{channel::mpsc, SinkExt, StreamExt};
use mempool_notifications::MempoolNotificationSender;
use std::{
    future::Future,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use storage_interface::{DbReader, DbReaderWriter};
use tokio::{
    runtime::{Handle, Runtime},
    task::{yield_now, JoinHandle},
};

/// Synchronizes the storage of the node by verifying and storing new data
/// (e.g., transactions and outputs).
pub trait StorageSynchronizerInterface {
    /// Applies a batch of transaction outputs.
    ///
    /// Note: this assumes that the ledger infos have already been verified.
    fn apply_transaction_outputs(
        &mut self,
        notification_id: NotificationId,
        output_list_with_proof: TransactionOutputListWithProof,
        target_ledger_info: LedgerInfoWithSignatures,
        end_of_epoch_ledger_info: Option<LedgerInfoWithSignatures>,
    ) -> Result<(), Error>;

    /// Executes a batch of transactions.
    ///
    /// Note: this assumes that the ledger infos have already been verified.
    fn execute_transactions(
        &mut self,
        notification_id: NotificationId,
        transaction_list_with_proof: TransactionListWithProof,
        target_ledger_info: LedgerInfoWithSignatures,
        end_of_epoch_ledger_info: Option<LedgerInfoWithSignatures>,
    ) -> Result<(), Error>;

    /// Initializes an account synchronizer with the specified
    /// `target_ledger_info` and `target_output_with_proof` at the target
    /// syncing version. Returns a join handle to the account synchronizer.
    ///
    /// Note: this assumes that `epoch_change_proofs`, `target_ledger_info`,
    /// and `target_output_with_proof` have already been verified.
    fn initialize_account_synchronizer(
        &mut self,
        epoch_change_proofs: Vec<LedgerInfoWithSignatures>,
        target_ledger_info: LedgerInfoWithSignatures,
        target_output_with_proof: TransactionOutputListWithProof,
    ) -> Result<JoinHandle<()>, Error>;

    /// Returns true iff there is storage data that is still waiting
    /// to be executed/applied or committed.
    fn pending_storage_data(&self) -> bool;

    /// Saves the given account states to storage.
    ///
    /// Note: this requires that `initialize_account_synchronizer` has been
    /// called.
    fn save_account_states(
        &mut self,
        notification_id: NotificationId,
        account_states_with_proof: StateValueChunkWithProof,
    ) -> Result<(), Error>;
}

/// The implementation of the `StorageSynchronizerInterface` used by state sync
pub struct StorageSynchronizer<ChunkExecutor> {
    // The executor for transaction and transaction output chunks
    chunk_executor: Arc<ChunkExecutor>,

    // A channel through which to notify the driver of committed account data
    commit_notification_sender: mpsc::UnboundedSender<CommitNotification>,

    // The configuration of the state sync driver
    driver_config: StateSyncDriverConfig,

    // A channel through which to notify the driver of storage errors
    error_notification_sender: mpsc::UnboundedSender<ErrorNotification>,

    // A channel through which to notify the executor of new data chunks
    executor_notifier: mpsc::Sender<StorageDataChunk>,

    // The number of storage data chunks pending execute/apply, or commit
    pending_data_chunks: Arc<AtomicU64>,

    // An optional runtime on which to spawn the storage synchronizer threads
    runtime: Option<Handle>,

    // The channel through which to notify the state snapshot receiver of new data chunks
    state_snapshot_notifier: Option<mpsc::Sender<StorageDataChunk>>,

    // The reader and writer for storage (required for account syncing)
    storage: DbReaderWriter,
}

// TODO(joshlind): this cannot currently be derived because of limitations around
// how deriving `Clone` works. See: https://github.com/rust-lang/rust/issues/26925.
impl<ChunkExecutor: ChunkExecutorTrait + 'static> Clone for StorageSynchronizer<ChunkExecutor> {
    fn clone(&self) -> Self {
        Self {
            chunk_executor: self.chunk_executor.clone(),
            commit_notification_sender: self.commit_notification_sender.clone(),
            driver_config: self.driver_config,
            error_notification_sender: self.error_notification_sender.clone(),
            executor_notifier: self.executor_notifier.clone(),
            pending_data_chunks: self.pending_data_chunks.clone(),
            runtime: self.runtime.clone(),
            state_snapshot_notifier: self.state_snapshot_notifier.clone(),
            storage: self.storage.clone(),
        }
    }
}

impl<ChunkExecutor: ChunkExecutorTrait + 'static> StorageSynchronizer<ChunkExecutor> {
    /// Returns a new storage synchronizer alongside the executor and committer handles
    pub fn new<MempoolNotifier: MempoolNotificationSender>(
        driver_config: StateSyncDriverConfig,
        chunk_executor: Arc<ChunkExecutor>,
        commit_notification_sender: mpsc::UnboundedSender<CommitNotification>,
        error_notification_sender: mpsc::UnboundedSender<ErrorNotification>,
        event_subscription_service: Arc<Mutex<EventSubscriptionService>>,
        mempool_notification_handler: MempoolNotificationHandler<MempoolNotifier>,
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
        utils::initialize_sync_version_gauges(storage.reader.clone())
            .expect("Failed to initialize the metric gauges!");

        let storage_synchronizer = Self {
            chunk_executor,
            commit_notification_sender,
            driver_config,
            error_notification_sender,
            executor_notifier,
            pending_data_chunks: pending_transaction_chunks,
            runtime,
            state_snapshot_notifier: None,
            storage,
        };

        (storage_synchronizer, executor_handle, committer_handle)
    }

    /// Notifies the executor of new data chunks
    fn notify_executor(&mut self, storage_data_chunk: StorageDataChunk) -> Result<(), Error> {
        if let Err(error) = self.executor_notifier.try_send(storage_data_chunk) {
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

impl<ChunkExecutor: ChunkExecutorTrait + 'static> StorageSynchronizerInterface
    for StorageSynchronizer<ChunkExecutor>
{
    fn apply_transaction_outputs(
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
        self.notify_executor(storage_data_chunk)
    }

    fn execute_transactions(
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
        self.notify_executor(storage_data_chunk)
    }

    fn initialize_account_synchronizer(
        &mut self,
        epoch_change_proofs: Vec<LedgerInfoWithSignatures>,
        target_ledger_info: LedgerInfoWithSignatures,
        target_output_with_proof: TransactionOutputListWithProof,
    ) -> Result<JoinHandle<()>, Error> {
        // Create a channel to notify the state snapshot receiver when data chunks are ready
        let max_pending_data_chunks = self.driver_config.max_pending_data_chunks as usize;
        let (state_snapshot_notifier, state_snapshot_listener) =
            mpsc::channel(max_pending_data_chunks);

        // Spawn the state snapshot receiver that commits account states
        let receiver_handle = spawn_state_snapshot_receiver(
            self.chunk_executor.clone(),
            state_snapshot_listener,
            self.commit_notification_sender.clone(),
            self.error_notification_sender.clone(),
            self.pending_data_chunks.clone(),
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

    fn save_account_states(
        &mut self,
        notification_id: NotificationId,
        account_states_with_proof: StateValueChunkWithProof,
    ) -> Result<(), Error> {
        let state_snapshot_notifier = &mut self
            .state_snapshot_notifier
            .as_mut()
            .expect("The state snapshot receiver has not been initialized!");
        let storage_data_chunk =
            StorageDataChunk::Accounts(notification_id, account_states_with_proof);
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
}

/// A chunk of data to be executed and/or committed to storage (i.e., accounts,
/// transactions or outputs).
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
enum StorageDataChunk {
    Accounts(NotificationId, StateValueChunkWithProof),
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
        loop {
            ::futures::select! {
                storage_data_chunk = executor_listener.select_next_some() => {
                    // Execute/apply the storage data chunk
                    let (notification_id, result) = match storage_data_chunk {
                        StorageDataChunk::Transactions(notification_id, transactions_with_proof, target_ledger_info, end_of_epoch_ledger_info) => {
                            let num_transactions = transactions_with_proof.transactions.len();
                            let result = chunk_executor
                               .execute_chunk(
                                    transactions_with_proof,
                                    &target_ledger_info,
                                    end_of_epoch_ledger_info.as_ref(),
                                );
                            if result.is_ok() {
                                metrics::increment_gauge(
                                    &metrics::STORAGE_SYNCHRONIZER_OPERATIONS,
                                    metrics::StorageSynchronizerOperations::ExecutedTransactions
                                        .get_label(),
                                    num_transactions as u64,
                                );
                            }
                            (notification_id, result)
                        },
                        StorageDataChunk::TransactionOutputs(notification_id, outputs_with_proof, target_ledger_info, end_of_epoch_ledger_info) => {
                            let num_outputs = outputs_with_proof.transactions_and_outputs.len();
                            let result = chunk_executor
                                .apply_chunk(
                                    outputs_with_proof,
                                    &target_ledger_info,
                                    end_of_epoch_ledger_info.as_ref(),
                                );
                            if result.is_ok() {
                                metrics::increment_gauge(
                                    &metrics::STORAGE_SYNCHRONIZER_OPERATIONS,
                                    metrics::StorageSynchronizerOperations::AppliedTransactionOutputs
                                        .get_label(),
                                    num_outputs as u64,
                                );
                            }
                            (notification_id, result)
                        }
                        storage_data_chunk => {
                            panic!("Invalid storage data chunk sent to executor: {:?}", storage_data_chunk);
                        }
                    };

                    // Notify the committer of new executed chunks
                    match result {
                        Ok(()) => {
                            if let Err(error) = committer_notifier.try_send(notification_id) {
                                let error = format!("Failed to notify the committer! Error: {:?}", error);
                                send_storage_synchronizer_error(error_notification_sender.clone(), notification_id, error).await;
                                decrement_pending_data_chunks(pending_transaction_chunks.clone());
                            }
                        },
                        Err(error) => {
                            let error = format!("Failed to execute/apply the storage data chunk! Error: {:?}", error);
                            send_storage_synchronizer_error(error_notification_sender.clone(), notification_id, error).await;
                            decrement_pending_data_chunks(pending_transaction_chunks.clone());
                        }
                    }
                    yield_thread().await;
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
        loop {
            ::futures::select! {
                notification_id = committer_listener.select_next_some() => {
                    // Commit the executed chunk
                    match chunk_executor.commit_chunk() {
                        Ok((events, transactions)) => {
                             // Log the event and update the metrics
                             debug!(
                                LogSchema::new(LogEntry::StorageSynchronizer).message(&format!(
                                    "Committed a new transaction chunk! \
                                    Transaction total: {:?}, event total: {:?}",
                                   transactions.len(),
                                   events.len()
                                ))
                            );
                            metrics::increment_gauge(
                                &metrics::STORAGE_SYNCHRONIZER_OPERATIONS,
                                metrics::StorageSynchronizerOperations::Synced
                                    .get_label(),
                                transactions.len() as u64,
                            );

                            // Handle the committed transaction notification (e.g., notify mempool).
                            // We do this here due to synchronization issues with mempool and
                            // storage. See: https://github.com/aptos-labs/aptos-core/issues/553
                            let committed_transactions = CommittedTransactions {
                                events,
                                transactions
                            };
                            utils::handle_committed_transactions(committed_transactions,
                                storage.clone(),
                                mempool_notification_handler.clone(),
                                event_subscription_service.clone(),
                            ).await;
                        }
                        Err(error) => {
                            let error = format!("Failed to commit executed chunk! Error: {:?}", error);
                            send_storage_synchronizer_error(error_notification_sender.clone(), notification_id, error).await;
                        }
                    };
                    decrement_pending_data_chunks(pending_transaction_chunks.clone());
                    yield_thread().await;
                }
            }
        }
    };

    // Spawn the committer
    spawn(runtime, committer)
}

/// Spawns a dedicated receiver that commits accounts from a state snapshot
fn spawn_state_snapshot_receiver<ChunkExecutor: ChunkExecutorTrait + 'static>(
    chunk_executor: Arc<ChunkExecutor>,
    mut state_snapshot_listener: mpsc::Receiver<StorageDataChunk>,
    mut commit_notification_sender: mpsc::UnboundedSender<CommitNotification>,
    error_notification_sender: mpsc::UnboundedSender<ErrorNotification>,
    pending_transaction_chunks: Arc<AtomicU64>,
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

        // Handle account state chunks
        loop {
            ::futures::select! {
                storage_data_chunk = state_snapshot_listener.select_next_some() => {
                    // Process the chunk
                    match storage_data_chunk {
                        StorageDataChunk::Accounts(notification_id, account_states_with_proof) => {
                            let all_accounts_synced = account_states_with_proof.is_last_chunk();
                            let last_committed_account_index = account_states_with_proof.last_index;

                            // Attempt to commit the chunk
                            let commit_result = state_snapshot_receiver.add_chunk(
                                account_states_with_proof.raw_values,
                                account_states_with_proof.proof.clone(),
                            );
                            match commit_result {
                                Ok(()) => {
                                    // Update the metrics
                                    metrics::set_gauge(
                                        &metrics::STORAGE_SYNCHRONIZER_OPERATIONS,
                                        metrics::StorageSynchronizerOperations::SyncedAccounts
                                            .get_label(),
                                        last_committed_account_index as u64,
                                    );

                                    if !all_accounts_synced {
                                        // Send a commit notification to the listener
                                        let commit_notification = CommitNotification::new_committed_accounts(all_accounts_synced, last_committed_account_index, None);
                                        if let Err(error) = commit_notification_sender.send(commit_notification).await {
                                            let error = format!("Failed to send account commit notification! Error: {:?}", error);
                                            send_storage_synchronizer_error(error_notification_sender.clone(), notification_id, error).await;
                                        }

                                        decrement_pending_data_chunks(pending_transaction_chunks.clone());
                                        yield_thread().await;
                                        continue; // Wait for the next chunk
                                    }

                                    // All accounts have been synced! Create a new commit notification
                                    let commit_notification = create_final_commit_notification(&target_output_with_proof, last_committed_account_index);

                                    // Finalize storage, reset the executor and send a commit
                                    // notification to the listener.
                                    let finalized_result = if let Err(error) = state_snapshot_receiver.finish_box() {
                                        Err(format!("Failed to finish the account states synchronization! Error: {:?}", error))
                                    } else if let Err(error) = storage.writer.finalize_state_snapshot(version, target_output_with_proof) {
                                        Err(format!("Failed to finalize the state snapshot! Error: {:?}", error))
                                    } else if let Err(error) = storage.writer.save_ledger_infos(&epoch_change_proofs) {
                                        Err(format!("Failed to save all epoch ending ledger infos! Error: {:?}", error))
                                    } else if let Err(error) = storage.writer.delete_genesis() {
                                        Err(format!("Failed to delete the genesis transaction! Error: {:?}", error))
                                    } else if let Err(error) = chunk_executor.reset() {
                                        Err(format!("Failed to reset the chunk executor after account states synchronization! Error: {:?}", error))
                                    } else if let Err(error) = commit_notification_sender.send(commit_notification).await {
                                       Err(format!("Failed to send the final account commit notification! Error: {:?}", error))
                                    } else if let Err(error) = utils::initialize_sync_version_gauges(storage.reader) {
                                       Err(format!("Failed to initialize the state sync version gauges! Error: {:?}", error))
                                    } else {
                                        Ok(())
                                    };

                                    // Notify the state sync driver of any errors
                                    if let Err(error) = finalized_result {
                                      send_storage_synchronizer_error(error_notification_sender.clone(), notification_id, error).await;
                                    }
                                    decrement_pending_data_chunks(pending_transaction_chunks.clone());
                                    return; // There's nothing left to do!
                                },
                                Err(error) => {
                                    let error = format!("Failed to commit account states chunk! Error: {:?}", error);
                                    send_storage_synchronizer_error(error_notification_sender.clone(), notification_id, error).await;
                                }
                            }
                        },
                        storage_data_chunk => {
                            panic!("Invalid storage data chunk sent to state snapshot receiver: {:?}", storage_data_chunk);
                        }
                    }
                    decrement_pending_data_chunks(pending_transaction_chunks.clone());
                }
            }
        }
    };

    // Spawn the receiver
    spawn(runtime, receiver)
}

/// Creates a final commit notification for the last account states chunk
fn create_final_commit_notification(
    target_output_with_proof: &TransactionOutputListWithProof,
    last_committed_account_index: u64,
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
    let committed_transaction = CommittedTransactions {
        events,
        transactions,
    };
    CommitNotification::new_committed_accounts(
        true,
        last_committed_account_index,
        Some(committed_transaction),
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

/// This yields the currently executing thread. This is required
/// to avoid starvation of other threads when the system is under
/// heavy load (see: https://github.com/aptos-labs/aptos-core/issues/623).
///
/// TODO(joshlind): identify a better solution. It likely requires
/// using spawn_blocking() at a lower level, or merging runtimes.
async fn yield_thread() {
    // We have a 50% chance of yielding here.
    sample!(SampleRate::Frequency(2), yield_now().await;);
}
