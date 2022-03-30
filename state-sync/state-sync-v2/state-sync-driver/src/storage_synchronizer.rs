// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::Error,
    notification_handlers::{CommitNotification, ErrorNotification},
};
use aptos_config::config::StateSyncDriverConfig;
use aptos_logger::prelude::*;
use aptos_types::{
    ledger_info::LedgerInfoWithSignatures,
    state_store::state_value::StateValueChunkWithProof,
    transaction::{TransactionListWithProof, TransactionOutputListWithProof},
};
use data_streaming_service::data_notification::NotificationId;
use executor_types::ChunkExecutorTrait;
use futures::{channel::mpsc, SinkExt, StreamExt};
use std::{
    future::Future,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use storage_interface::DbWriter;
use tokio::runtime::{Handle, Runtime};

// TODO(joshlind): add structured logging support!

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
    /// syncing version. Also, writes all `epoch_change_proofs` to storage.
    ///
    /// Note: this assumes that `epoch_change_proofs`, `target_ledger_info`,
    /// and `target_output_with_proof` have already been verified.
    fn initialize_account_synchronizer(
        &mut self,
        epoch_change_proofs: Vec<LedgerInfoWithSignatures>,
        target_ledger_info: LedgerInfoWithSignatures,
        target_output_with_proof: TransactionOutputListWithProof,
    ) -> Result<(), Error>;

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

    // A channel through which to notify the driver of committed data
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

    // The writer to storage (required for account state syncing)
    storage: Arc<dyn DbWriter>,
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
    pub fn new(
        driver_config: StateSyncDriverConfig,
        chunk_executor: Arc<ChunkExecutor>,
        commit_notification_sender: mpsc::UnboundedSender<CommitNotification>,
        error_notification_sender: mpsc::UnboundedSender<ErrorNotification>,
        storage: Arc<dyn DbWriter>,
        runtime: Option<&Runtime>,
    ) -> Self {
        // Create a channel to notify the executor when data chunks are ready
        let max_pending_data_chunks = driver_config.max_pending_data_chunks as usize;
        let (executor_notifier, executor_listener) = mpsc::channel(max_pending_data_chunks);

        // Create a channel to notify the committer when executed chunks are ready
        let (committer_notifier, committer_listener) = mpsc::channel(max_pending_data_chunks);

        // Create a shared pending data chunk counter
        let pending_transaction_chunks = Arc::new(AtomicU64::new(0));

        // Spawn the executor that executes/applies storage data chunks
        let runtime = runtime.map(|runtime| runtime.handle().clone());
        spawn_executor(
            chunk_executor.clone(),
            error_notification_sender.clone(),
            executor_listener,
            committer_notifier,
            pending_transaction_chunks.clone(),
            runtime.clone(),
        );

        // Spawn the committer that commits executed (but pending) chunks
        spawn_committer(
            chunk_executor.clone(),
            committer_listener,
            commit_notification_sender.clone(),
            error_notification_sender.clone(),
            pending_transaction_chunks.clone(),
            runtime.clone(),
        );

        Self {
            chunk_executor,
            commit_notification_sender,
            driver_config,
            error_notification_sender,
            executor_notifier,
            pending_data_chunks: pending_transaction_chunks,
            runtime,
            state_snapshot_notifier: None,
            storage,
        }
    }

    /// Notifies the executor of new data chunks
    fn notify_executor(&mut self, storage_data_chunk: StorageDataChunk) -> Result<(), Error> {
        if let Err(error) = self.executor_notifier.try_send(storage_data_chunk) {
            Err(Error::UnexpectedError(format!(
                "Failed to send storage data chunk to executor: {:?}",
                error
            )))
        } else {
            increment_atomic(self.pending_data_chunks.clone());
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
    ) -> Result<(), Error> {
        // Create a channel to notify the state snapshot receiver when data chunks are ready
        let max_pending_data_chunks = self.driver_config.max_pending_data_chunks as usize;
        let (state_snapshot_notifier, state_snapshot_listener) =
            mpsc::channel(max_pending_data_chunks);

        // Spawn the state snapshot receiver that commits account states
        spawn_state_snapshot_receiver(
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

        Ok(())
    }

    fn pending_storage_data(&self) -> bool {
        self.pending_data_chunks.load(Ordering::Relaxed) > 0
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
            increment_atomic(self.pending_data_chunks.clone());
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
) {
    // Create an executor
    let executor = async move {
        loop {
            ::futures::select! {
                storage_data_chunk = executor_listener.select_next_some() => {
                    // Execute/apply the storage data chunk
                    let (notification_id, result) = match storage_data_chunk {
                        StorageDataChunk::Transactions(notification_id, transactions_with_proof, target_ledger_info, end_of_epoch_ledger_info) => {
                            let result = chunk_executor
                               .execute_chunk(
                                    transactions_with_proof,
                                    &target_ledger_info,
                                    end_of_epoch_ledger_info.as_ref(),
                                );
                            (notification_id, result)
                        },
                        StorageDataChunk::TransactionOutputs(notification_id, outputs_with_proof, target_ledger_info, end_of_epoch_ledger_info) => {
                            let result = chunk_executor
                                .apply_chunk(
                                    outputs_with_proof,
                                    &target_ledger_info,
                                    end_of_epoch_ledger_info.as_ref(),
                                );
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
                                decrement_atomic(pending_transaction_chunks.clone());
                            }
                        },
                        Err(error) => {
                            let error = format!("Failed to execute/apply the storage data chunk! Error: {:?}", error);
                            send_storage_synchronizer_error(error_notification_sender.clone(), notification_id, error).await;
                            decrement_atomic(pending_transaction_chunks.clone());
                        }
                    }
                }
            }
        }
    };

    // Spawn the executor
    spawn(runtime, executor);
}

/// Spawns a dedicated committer that commits executed (but pending) chunks
fn spawn_committer<ChunkExecutor: ChunkExecutorTrait + 'static>(
    chunk_executor: Arc<ChunkExecutor>,
    mut committer_listener: mpsc::Receiver<NotificationId>,
    mut commit_notification_sender: mpsc::UnboundedSender<CommitNotification>,
    error_notification_sender: mpsc::UnboundedSender<ErrorNotification>,
    pending_transaction_chunks: Arc<AtomicU64>,
    runtime: Option<Handle>,
) {
    // Create a committer
    let committer = async move {
        loop {
            ::futures::select! {
                notification_id = committer_listener.select_next_some() => {
                    // Commit the executed chunk
                    match chunk_executor.commit_chunk() {
                        Ok((events, transactions)) => {
                            // Send a commit notification to the commit listener
                            let commit_notification = CommitNotification::new_committed_transactions(events, transactions);
                            if let Err(error) = commit_notification_sender.send(commit_notification).await {
                                let error = format!("Failed to send transaction commit notification! Error: {:?}", error);
                                send_storage_synchronizer_error(error_notification_sender.clone(), notification_id, error).await;
                            }
                        }
                        Err(error) => {
                            let error = format!("Failed to commit executed chunk! Error: {:?}", error);
                            send_storage_synchronizer_error(error_notification_sender.clone(), notification_id, error).await;
                        }
                    };
                    decrement_atomic(pending_transaction_chunks.clone());
                }
            }
        }
    };

    // Spawn the committer
    spawn(runtime, committer);
}

/// Spawns a dedicated receiver that commits accounts from a state snapshot
fn spawn_state_snapshot_receiver<ChunkExecutor: ChunkExecutorTrait + 'static>(
    chunk_executor: Arc<ChunkExecutor>,
    mut state_snapshot_listener: mpsc::Receiver<StorageDataChunk>,
    mut commit_notification_sender: mpsc::UnboundedSender<CommitNotification>,
    error_notification_sender: mpsc::UnboundedSender<ErrorNotification>,
    pending_transaction_chunks: Arc<AtomicU64>,
    storage: Arc<dyn DbWriter>,
    epoch_change_proofs: Vec<LedgerInfoWithSignatures>,
    target_ledger_info: LedgerInfoWithSignatures,
    target_output_with_proof: TransactionOutputListWithProof,
    runtime: Option<Handle>,
) {
    // Create a state snapshot receiver
    let receiver = async move {
        // Get the target version and expected root hash
        let version = target_ledger_info.ledger_info().version();
        let expected_root_hash = target_output_with_proof
            .proof
            .transaction_infos
            .first()
            .expect("Target transaction info should exist!")
            .state_change_hash();

        // Create the snapshot receiver
        let mut state_snapshot_receiver = storage
            .get_state_snapshot_receiver(version, expected_root_hash)
            .expect("Failed to initialize the state snapshot receiver!");

        // Handle account state chunks
        loop {
            ::futures::select! {
                storage_data_chunk = state_snapshot_listener.select_next_some() => {
                    // Commit the account states chunk
                    match storage_data_chunk {
                        StorageDataChunk::Accounts(notification_id, account_states_with_proof) => {
                            let all_accounts_synced = account_states_with_proof.proof.right_siblings().is_empty();
                            let last_committed_account_index = account_states_with_proof.last_index;

                            // Attempt to the commit the chunk
                            let commit_result = state_snapshot_receiver.add_chunk(
                                account_states_with_proof.raw_values,
                                account_states_with_proof.proof.clone(),
                            );
                            match commit_result {
                                Ok(()) => {
                                    // Send a commit notification to the commit listener
                                    let commit_notification = CommitNotification::new_committed_accounts(all_accounts_synced, last_committed_account_index);
                                    if let Err(error) = commit_notification_sender.send(commit_notification).await {
                                        let error = format!("Failed to send account commit notification! Error: {:?}", error);
                                        send_storage_synchronizer_error(error_notification_sender.clone(), notification_id, error).await;
                                    } else if all_accounts_synced {
                                        // We're done synchronizing account states. Finalize storage and reset the executor.
                                        let finalized_result = if let Err(error) = state_snapshot_receiver.finish_box() {
                                            Err(format!("Failed to finish the account states synchronization! Error: {:?}", error))
                                        } else if let Err(error) = storage.finalize_state_snapshot(version, target_output_with_proof) {
                                            Err(format!("Failed to finalize the state snapshot! Error: {:?}", error))
                                        } else if let Err(error) = storage.save_ledger_infos(&epoch_change_proofs) {
                                            Err(format!("Failed to save all epoch ending ledger infos! Error: {:?}", error))
                                        } else if let Err(error) = chunk_executor.reset() { // Reset the chunk executor (to read the latest db state)
                                            Err(format!("Failed to reset the chunk executor after account states synchronization! Error: {:?}", error))
                                        } else {
                                            Ok(())
                                        };

                                        // Notify the state sync driver of any errors
                                        if let Err(error) = finalized_result {
                                            send_storage_synchronizer_error(error_notification_sender.clone(), notification_id, error).await;
                                        }

                                        decrement_atomic(pending_transaction_chunks.clone());
                                        return;
                                    }
                                },
                                Err(error) => {
                                    let error = format!("Failed to commit account states chunk! Error: {:?}", error);
                                    send_storage_synchronizer_error(error_notification_sender.clone(), notification_id, error).await;
                                }
                            }
                            decrement_atomic(pending_transaction_chunks.clone());
                        },
                        storage_data_chunk => {
                            panic!("Invalid storage data chunk sent to state snapshot receiver: {:?}", storage_data_chunk);
                        }
                    }
                }
            }
        }
    };

    // Spawn the receiver
    spawn(runtime, receiver);
}

/// Spawns a future on a specified runtime. If no runtime is specified, uses
/// the current runtime.
fn spawn(runtime: Option<Handle>, future: impl Future<Output = ()> + Send + 'static) {
    if let Some(runtime) = runtime {
        runtime.spawn(future);
    } else {
        tokio::spawn(future);
    }
}

/// Increments the given atomic u64
fn increment_atomic(atomic_u64: Arc<AtomicU64>) {
    atomic_u64.fetch_add(1, Ordering::Relaxed);
}

/// Decrements the given atomic u64
fn decrement_atomic(atomic_u64: Arc<AtomicU64>) {
    atomic_u64.fetch_sub(1, Ordering::Relaxed);
}

/// Sends an error notification to the notification listener
async fn send_storage_synchronizer_error(
    mut error_notification_sender: mpsc::UnboundedSender<ErrorNotification>,
    notification_id: NotificationId,
    error_message: String,
) {
    let error_message = format!("Storage synchronizer error: {:?}", error_message);
    error!("{:?}", error_message);

    // Send an error notification
    let error_notification = ErrorNotification {
        error: Error::UnexpectedError(error_message),
        notification_id,
    };
    if let Err(error) = error_notification_sender.send(error_notification).await {
        panic!("Failed to send error notification! Error: {:?}", error);
    }
}
