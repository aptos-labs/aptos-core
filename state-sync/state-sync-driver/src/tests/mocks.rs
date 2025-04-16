// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::Error,
    metadata_storage::MetadataStorageInterface,
    storage_synchronizer::{NotificationMetadata, StorageSynchronizerInterface},
    tests::utils::{create_empty_epoch_state, create_epoch_ending_ledger_info},
};
use anyhow::Result as AnyhowResult;
use aptos_crypto::HashValue;
use aptos_data_streaming_service::{
    data_notification::NotificationId,
    data_stream::{DataStreamId, DataStreamListener},
    streaming_client::{DataStreamingClient, Epoch, NotificationAndFeedback},
};
use aptos_executor_types::{ChunkCommitNotification, ChunkExecutorTrait};
use aptos_storage_interface::{
    chunk_to_commit::ChunkToCommit, DbReader, DbReaderWriter, DbWriter, LedgerSummary, Order,
    Result, StateSnapshotReceiver,
};
use aptos_types::{
    account_address::AccountAddress,
    contract_event::EventWithVersion,
    epoch_change::EpochChangeProof,
    epoch_state::EpochState,
    event::EventKey,
    ledger_info::LedgerInfoWithSignatures,
    proof::{
        AccumulatorConsistencyProof, SparseMerkleProof, SparseMerkleRangeProof,
        TransactionAccumulatorSummary,
    },
    state_proof::StateProof,
    state_store::{
        state_key::StateKey,
        state_value::{StateValue, StateValueChunkWithProof},
    },
    transaction::{
        AccountOrderedTransactionsWithProof, IndexedTransactionSummary, TransactionListWithProof,
        TransactionOutputListWithProof, TransactionWithProof, Version,
    },
};
use async_trait::async_trait;
use mockall::mock;
use std::sync::Arc;
use tokio::task::JoinHandle;

// TODO(joshlind): if we see these as generally useful, we should
// modify the definitions in the rest of the code.

/// Creates a mock chunk executor
pub fn create_mock_executor() -> MockChunkExecutor {
    MockChunkExecutor::new()
}

/// Creates a mock database reader
pub fn create_mock_db_reader() -> MockDatabaseReader {
    MockDatabaseReader::new()
}

/// Creates a mock database writer
pub fn create_mock_db_writer() -> MockDatabaseWriter {
    MockDatabaseWriter::new()
}

/// Creates a mock database reader writer
pub fn create_mock_reader_writer(
    reader: Option<MockDatabaseReader>,
    writer: Option<MockDatabaseWriter>,
) -> DbReaderWriter {
    create_mock_reader_writer_with_version(reader, writer, 0)
}

/// Creates a mock database reader writer with the given
/// highest synced transaction version.
pub fn create_mock_reader_writer_with_version(
    reader: Option<MockDatabaseReader>,
    writer: Option<MockDatabaseWriter>,
    highest_synced_version: u64,
) -> DbReaderWriter {
    let mut reader = reader.unwrap_or_else(create_mock_db_reader);
    reader
        .expect_get_synced_version()
        .returning(move || Ok(Some(highest_synced_version)));
    reader
        .expect_get_pre_committed_version()
        .returning(move || Ok(Some(highest_synced_version)));
    reader
        .expect_get_latest_epoch_state()
        .returning(|| Ok(create_empty_epoch_state()));
    reader
        .expect_get_latest_ledger_info()
        .returning(|| Ok(create_epoch_ending_ledger_info()));

    let writer = writer.unwrap_or_else(create_mock_db_writer);
    DbReaderWriter {
        reader: Arc::new(reader),
        writer: Arc::new(writer),
    }
}

/// Creates a mock state snapshot receiver
pub fn create_mock_receiver() -> MockSnapshotReceiver {
    MockSnapshotReceiver::new()
}

/// Creates a mock data streaming client
pub fn create_mock_streaming_client() -> MockStreamingClient {
    MockStreamingClient::new()
}

/// Creates a mock storage synchronizer
pub fn create_mock_storage_synchronizer() -> MockStorageSynchronizer {
    MockStorageSynchronizer::new()
}

/// Creates a mock storage synchronizer that is not currently handling
/// any pending storage data.
pub fn create_ready_storage_synchronizer(expect_reset_executor: bool) -> MockStorageSynchronizer {
    let mut mock_storage_synchronizer = create_mock_storage_synchronizer();
    mock_storage_synchronizer
        .expect_pending_storage_data()
        .return_const(false);
    if expect_reset_executor {
        mock_storage_synchronizer
            .expect_finish_chunk_executor()
            .return_const(());
        mock_storage_synchronizer
            .expect_reset_chunk_executor()
            .return_const(Ok(()));
    }

    mock_storage_synchronizer
}

// This automatically creates a MockChunkExecutor.
mock! {
    pub ChunkExecutor {}
    impl ChunkExecutorTrait for ChunkExecutor {
        fn execute_chunk<'a>(
            &self,
            txn_list_with_proof: TransactionListWithProof,
            verified_target_li: &LedgerInfoWithSignatures,
            epoch_change_li: Option<&'a LedgerInfoWithSignatures>,
        ) -> AnyhowResult<()>;

        fn apply_chunk<'a>(
            &self,
            txn_output_list_with_proof: TransactionOutputListWithProof,
            verified_target_li: &LedgerInfoWithSignatures,
            epoch_change_li: Option<&'a LedgerInfoWithSignatures>,
        ) -> AnyhowResult<()>;

        fn enqueue_chunk_by_execution<'a>(
            &self,
            txn_list_with_proof: TransactionListWithProof,
            verified_target_li: &LedgerInfoWithSignatures,
            epoch_change_li: Option<&'a LedgerInfoWithSignatures>,
        ) -> AnyhowResult<()>;

        fn enqueue_chunk_by_transaction_outputs<'a>(
            &self,
            txn_output_list_with_proof: TransactionOutputListWithProof,
            verified_target_li: &LedgerInfoWithSignatures,
            epoch_change_li: Option<&'a LedgerInfoWithSignatures>,
        ) -> AnyhowResult<()>;

        fn update_ledger(&self) -> AnyhowResult<()>;

        fn commit_chunk(&self) -> AnyhowResult<ChunkCommitNotification>;

        fn reset(&self) -> AnyhowResult<()>;

        fn finish(&self);
    }
}

// This automatically creates a MockDatabaseReader.
mock! {
    pub DatabaseReader {}
    impl DbReader for DatabaseReader {
        fn get_epoch_ending_ledger_infos(
            &self,
            start_epoch: u64,
            end_epoch: u64,
        ) -> Result<EpochChangeProof>;

        fn get_transactions(
            &self,
            start_version: Version,
            batch_size: u64,
            ledger_version: Version,
            fetch_events: bool,
        ) -> Result<TransactionListWithProof>;

        fn get_transaction_by_hash(
            &self,
            hash: HashValue,
            ledger_version: Version,
            fetch_events: bool,
        ) -> Result<Option<TransactionWithProof>>;

        fn get_transaction_by_version(
            &self,
            version: Version,
            ledger_version: Version,
            fetch_events: bool,
        ) -> Result<TransactionWithProof>;

        fn get_transaction_outputs(
            &self,
            start_version: Version,
            limit: u64,
            ledger_version: Version,
        ) -> Result<TransactionOutputListWithProof>;

        fn get_events(
            &self,
            event_key: &EventKey,
            start: u64,
            order: Order,
            limit: u64,
            ledger_version: Version,
        ) -> Result<Vec<EventWithVersion>>;

        fn get_block_timestamp(&self, version: u64) -> Result<u64>;

        fn get_last_version_before_timestamp(
            &self,
            _timestamp: u64,
            _ledger_version: Version,
        ) -> Result<Version>;

        fn get_latest_epoch_state(&self) -> Result<EpochState>;

        fn get_latest_ledger_info_option(&self) -> Result<Option<LedgerInfoWithSignatures>>;

        fn get_latest_ledger_info(&self) -> Result<LedgerInfoWithSignatures>;

        fn get_synced_version(&self) -> Result<Option<Version>>;

        fn get_pre_committed_version(&self) -> Result<Option<Version>>;

        fn get_latest_ledger_info_version(&self) -> Result<Version>;

        fn get_latest_commit_metadata(&self) -> Result<(Version, u64)>;

        fn get_account_ordered_transaction(
            &self,
            address: AccountAddress,
            seq_num: u64,
            include_events: bool,
            ledger_version: Version,
        ) -> Result<Option<TransactionWithProof>>;

        fn get_account_ordered_transactions(
            &self,
            address: AccountAddress,
            seq_num: u64,
            limit: u64,
            include_events: bool,
            ledger_version: Version,
        ) -> Result<AccountOrderedTransactionsWithProof>;

        fn get_account_transaction_summaries(
            &self,
            address: AccountAddress,
            start_version: Option<u64>,
            end_version: Option<u64>,
            limit: u64,
            ledger_version: Version,
        ) -> Result<Vec<IndexedTransactionSummary>>;

        fn get_state_proof_with_ledger_info(
            &self,
            known_version: u64,
            ledger_info: LedgerInfoWithSignatures,
        ) -> Result<StateProof>;

        fn get_state_proof(&self, known_version: u64) -> Result<StateProof>;

        fn get_state_value_with_proof_by_version(
            &self,
            state_key: &StateKey,
            version: Version,
        ) -> Result<(Option<StateValue>, SparseMerkleProof)>;

        fn get_pre_committed_ledger_summary(&self) -> Result<LedgerSummary>;

        fn get_epoch_ending_ledger_info(&self, known_version: u64) -> Result<LedgerInfoWithSignatures>;

        fn get_accumulator_root_hash(&self, _version: Version) -> Result<HashValue>;

        fn get_accumulator_consistency_proof(
            &self,
            _client_known_version: Option<Version>,
            _ledger_version: Version,
        ) -> Result<AccumulatorConsistencyProof>;

        fn get_accumulator_summary(
            &self,
            ledger_version: Version,
        ) -> Result<TransactionAccumulatorSummary>;

        fn get_state_item_count(&self, version: Version) -> Result<usize>;

        fn get_state_value_chunk_with_proof(
            &self,
            version: Version,
            start_idx: usize,
            chunk_size: usize,
        ) -> Result<StateValueChunkWithProof>;

        fn get_epoch_snapshot_prune_window(&self) -> Result<usize>;
    }
}

// This automatically creates a MockDatabaseWriter.
mock! {
    pub DatabaseWriter {}
    impl DbWriter for DatabaseWriter {
        fn get_state_snapshot_receiver(
            &self,
            version: Version,
            expected_root_hash: HashValue,
        ) -> Result<Box<dyn StateSnapshotReceiver<StateKey, StateValue>>>;

        fn finalize_state_snapshot(
            &self,
            version: Version,
            output_with_proof: TransactionOutputListWithProof,
            ledger_infos: &[LedgerInfoWithSignatures],
        ) -> Result<()>;

        fn save_transactions<'a, 'b>(
            &self,
            chunk: ChunkToCommit<'b>,
            ledger_info_with_sigs: Option<&'a LedgerInfoWithSignatures>,
            sync_commit: bool,
        ) -> Result<()>;
    }
}

// This automatically creates a MockMetadataStorage.
mock! {
    pub MetadataStorage {}
    impl MetadataStorageInterface for MetadataStorage {
        fn is_snapshot_sync_complete(
            &self,
            target_ledger_info: &LedgerInfoWithSignatures,
        ) -> Result<bool, Error>;

        fn get_last_persisted_state_value_index(
            &self,
            target_ledger_info: &LedgerInfoWithSignatures,
        ) -> Result<u64, Error>;

        fn previous_snapshot_sync_target(&self) -> Result<Option<LedgerInfoWithSignatures>, Error>;

        fn update_last_persisted_state_value_index(
            &self,
            target_ledger_info: &LedgerInfoWithSignatures,
            last_persisted_state_value_index: u64,
            snapshot_sync_completed: bool,
        ) -> Result<(), Error>;
    }

    impl Clone for MetadataStorage {
        fn clone(&self) -> Self;
    }
}

// This automatically creates a MockSnapshotReceiver.
mock! {
    pub SnapshotReceiver {}
    impl StateSnapshotReceiver<StateKey, StateValue> for SnapshotReceiver {
        fn add_chunk(&mut self, chunk: Vec<(StateKey, StateValue)>, proof: SparseMerkleRangeProof) -> Result<()>;

        fn finish(self) -> Result<()>;

        fn finish_box(self: Box<Self>) -> Result<()>;
    }
}

// This automatically creates a MockStreamingClient.
mock! {
    pub StreamingClient {}
    #[async_trait]
    impl DataStreamingClient for StreamingClient {
        async fn get_all_state_values(
            &self,
            version: Version,
            start_index: Option<u64>,
        ) -> AnyhowResult<DataStreamListener, aptos_data_streaming_service::error::Error>;

        async fn get_all_epoch_ending_ledger_infos(
            &self,
            start_epoch: Epoch,
        ) -> AnyhowResult<DataStreamListener, aptos_data_streaming_service::error::Error>;

        async fn get_all_transaction_outputs(
            &self,
            start_version: Version,
            end_version: Version,
            proof_version: Version,
        ) -> AnyhowResult<DataStreamListener, aptos_data_streaming_service::error::Error>;

        async fn get_all_transactions(
            &self,
            start_version: Version,
            end_version: Version,
            proof_version: Version,
            include_events: bool,
        ) -> AnyhowResult<DataStreamListener, aptos_data_streaming_service::error::Error>;

        async fn get_all_transactions_or_outputs(
            &self,
            start_version: Version,
            end_version: Version,
            proof_version: Version,
            include_events: bool,
        ) -> AnyhowResult<DataStreamListener, aptos_data_streaming_service::error::Error>;

        async fn continuously_stream_transaction_outputs(
            &self,
            start_version: Version,
            start_epoch: Epoch,
            target: Option<LedgerInfoWithSignatures>,
        ) -> AnyhowResult<DataStreamListener, aptos_data_streaming_service::error::Error>;

        async fn continuously_stream_transactions(
            &self,
            start_version: Version,
            start_epoch: Epoch,
            include_events: bool,
            target: Option<LedgerInfoWithSignatures>,
        ) -> AnyhowResult<DataStreamListener, aptos_data_streaming_service::error::Error>;

        async fn continuously_stream_transactions_or_outputs(
            &self,
            start_version: Version,
            start_epoch: Epoch,
            include_events: bool,
            target: Option<LedgerInfoWithSignatures>,
        ) -> AnyhowResult<DataStreamListener, aptos_data_streaming_service::error::Error>;

        async fn terminate_stream_with_feedback(
            &self,
            data_stream_id: DataStreamId,
            notification_and_feedback: Option<NotificationAndFeedback>,
        ) -> AnyhowResult<(), aptos_data_streaming_service::error::Error>;
    }
    impl Clone for StreamingClient {
        fn clone(&self) -> Self;
    }
}

// This automatically creates a MockStorageSynchronizer.
mock! {
    pub StorageSynchronizer {}
    #[async_trait]
    impl StorageSynchronizerInterface for StorageSynchronizer {
        async fn apply_transaction_outputs(
            &mut self,
            notification_metadata: NotificationMetadata,
            output_list_with_proof: TransactionOutputListWithProof,
            target_ledger_info: LedgerInfoWithSignatures,
            end_of_epoch_ledger_info: Option<LedgerInfoWithSignatures>,
        ) -> AnyhowResult<(), crate::error::Error>;

        async fn execute_transactions(
            &mut self,
            notification_metadata: NotificationMetadata,
            transaction_list_with_proof: TransactionListWithProof,
            target_ledger_info: LedgerInfoWithSignatures,
            end_of_epoch_ledger_info: Option<LedgerInfoWithSignatures>,
        ) -> AnyhowResult<(), crate::error::Error>;

        fn initialize_state_synchronizer(
            &mut self,
            epoch_change_proofs: Vec<LedgerInfoWithSignatures>,
            target_ledger_info: LedgerInfoWithSignatures,
            target_output_with_proof: TransactionOutputListWithProof,
        ) -> AnyhowResult<JoinHandle<()>, crate::error::Error>;

        fn pending_storage_data(&self) -> bool;

        async fn save_state_values(
            &mut self,
            notification_id: NotificationId,
            state_value_chunk_with_proof: StateValueChunkWithProof,
        ) -> AnyhowResult<(), crate::error::Error>;

        fn reset_chunk_executor(&self) -> AnyhowResult<(), crate::error::Error>;

        fn finish_chunk_executor(&self);
    }
    impl Clone for StorageSynchronizer {
        fn clone(&self) -> Self;
    }
}
