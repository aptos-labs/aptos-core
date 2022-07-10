// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    storage_synchronizer::StorageSynchronizerInterface,
    tests::utils::{create_startup_info, create_transaction_info},
};
use anyhow::Result;
use aptos_crypto::HashValue;
use aptos_types::{
    account_address::AccountAddress,
    contract_event::EventWithVersion,
    epoch_change::EpochChangeProof,
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
        AccountTransactionsWithProof, TransactionInfo, TransactionListWithProof,
        TransactionOutputListWithProof, TransactionToCommit, TransactionWithProof, Version,
    },
};
use async_trait::async_trait;
use data_streaming_service::{
    data_notification::NotificationId,
    data_stream::DataStreamListener,
    streaming_client::{DataStreamingClient, Epoch, NotificationFeedback},
};
use executor_types::{ChunkCommitNotification, ChunkExecutorTrait};
use mockall::mock;
use scratchpad::SparseMerkleTree;
use std::sync::Arc;
use storage_interface::{
    DbReader, DbReaderWriter, DbWriter, ExecutedTrees, Order, StartupInfo, StateSnapshotReceiver,
};
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
    let mut reader = reader.unwrap_or_else(create_mock_db_reader);
    reader
        .expect_get_latest_transaction_info_option()
        .returning(|| Ok(Some((0, create_transaction_info()))));
    reader
        .expect_get_startup_info()
        .returning(|| Ok(Some(create_startup_info())));

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
        ) -> Result<()>;

        fn apply_chunk<'a>(
            &self,
            txn_output_list_with_proof: TransactionOutputListWithProof,
            verified_target_li: &LedgerInfoWithSignatures,
            epoch_change_li: Option<&'a LedgerInfoWithSignatures>,
        ) -> anyhow::Result<()>;

        fn execute_and_commit_chunk<'a>(
            &self,
            txn_list_with_proof: TransactionListWithProof,
            verified_target_li: &LedgerInfoWithSignatures,
            epoch_change_li: Option<&'a LedgerInfoWithSignatures>,
        ) -> Result<ChunkCommitNotification>;

        fn apply_and_commit_chunk<'a>(
            &self,
            txn_output_list_with_proof: TransactionOutputListWithProof,
            verified_target_li: &LedgerInfoWithSignatures,
            epoch_change_li: Option<&'a LedgerInfoWithSignatures>,
        ) -> Result<ChunkCommitNotification>;

        fn commit_chunk(&self) -> Result<ChunkCommitNotification>;

        fn reset(&self) -> Result<()>;
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
        ) -> Result<Vec<EventWithVersion>>;

        fn get_block_timestamp(&self, version: u64) -> Result<u64>;

        fn get_last_version_before_timestamp(
            &self,
            _timestamp: u64,
            _ledger_version: Version,
        ) -> Result<Version>;

        fn get_latest_state_value(&self, state_key: StateKey) -> Result<Option<StateValue>>;

        fn get_latest_ledger_info_option(&self) -> Result<Option<LedgerInfoWithSignatures>>;

        fn get_latest_ledger_info(&self) -> Result<LedgerInfoWithSignatures>;

        fn get_latest_version_option(&self) -> Result<Option<Version>>;

        fn get_latest_version(&self) -> Result<Version>;

        fn get_latest_commit_metadata(&self) -> Result<(Version, u64)>;

        fn get_startup_info(&self) -> Result<Option<StartupInfo>>;

        fn get_account_transaction(
            &self,
            address: AccountAddress,
            seq_num: u64,
            include_events: bool,
            ledger_version: Version,
        ) -> Result<Option<TransactionWithProof>>;

        fn get_account_transactions(
            &self,
            address: AccountAddress,
            seq_num: u64,
            limit: u64,
            include_events: bool,
            ledger_version: Version,
        ) -> Result<AccountTransactionsWithProof>;

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

        fn get_latest_executed_trees(&self) -> Result<ExecutedTrees>;

        fn get_epoch_ending_ledger_info(&self, known_version: u64) -> Result<LedgerInfoWithSignatures>;

        fn get_latest_transaction_info_option(&self) -> Result<Option<(Version, TransactionInfo)>>;

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

        fn get_state_leaf_count(&self, version: Version) -> Result<usize>;

        fn get_state_value_chunk_with_proof(
            &self,
            version: Version,
            start_idx: usize,
            chunk_size: usize,
        ) -> Result<StateValueChunkWithProof>;

        fn get_state_prune_window(&self) -> Result<Option<usize>>;
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
        ) -> Result<()>;

        fn save_ledger_infos(&self, ledger_infos: &[LedgerInfoWithSignatures]) -> Result<()>;

        fn save_transactions<'a>(
            &self,
            txns_to_commit: &[TransactionToCommit],
            first_version: Version,
            base_state_version: Option<Version>,
            ledger_info_with_sigs: Option<&'a LedgerInfoWithSignatures>,
            state_tree: SparseMerkleTree<StateValue>,
        ) -> Result<()>;

        fn delete_genesis(&self) -> Result<()>;
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
        ) -> Result<DataStreamListener, data_streaming_service::error::Error>;

        async fn get_all_epoch_ending_ledger_infos(
            &self,
            start_epoch: Epoch,
        ) -> Result<DataStreamListener, data_streaming_service::error::Error>;

        async fn get_all_transaction_outputs(
            &self,
            start_version: Version,
            end_version: Version,
            proof_version: Version,
        ) -> Result<DataStreamListener, data_streaming_service::error::Error>;

        async fn get_all_transactions(
            &self,
            start_version: Version,
            end_version: Version,
            proof_version: Version,
            include_events: bool,
        ) -> Result<DataStreamListener, data_streaming_service::error::Error>;

        async fn continuously_stream_transaction_outputs(
            &self,
            start_version: Version,
            start_epoch: Epoch,
            target: Option<LedgerInfoWithSignatures>,
        ) -> Result<DataStreamListener, data_streaming_service::error::Error>;

        async fn continuously_stream_transactions(
            &self,
            start_version: Version,
            start_epoch: Epoch,
            include_events: bool,
            target: Option<LedgerInfoWithSignatures>,
        ) -> Result<DataStreamListener, data_streaming_service::error::Error>;

        async fn terminate_stream_with_feedback(
            &self,
            notification_id: NotificationId,
            notification_feedback: NotificationFeedback,
        ) -> Result<(), data_streaming_service::error::Error>;
    }
    impl Clone for StreamingClient {
        fn clone(&self) -> Self;
    }
}

// This automatically creates a MockStorageSynchronizer.
mock! {
    pub StorageSynchronizer {}
    impl StorageSynchronizerInterface for StorageSynchronizer {
        fn apply_transaction_outputs(
            &mut self,
            notification_id: NotificationId,
            output_list_with_proof: TransactionOutputListWithProof,
            target_ledger_info: LedgerInfoWithSignatures,
            end_of_epoch_ledger_info: Option<LedgerInfoWithSignatures>,
        ) -> Result<(), crate::error::Error>;

        fn execute_transactions(
            &mut self,
            notification_id: NotificationId,
            transaction_list_with_proof: TransactionListWithProof,
            target_ledger_info: LedgerInfoWithSignatures,
            end_of_epoch_ledger_info: Option<LedgerInfoWithSignatures>,
        ) -> Result<(), crate::error::Error>;

        fn initialize_state_synchronizer(
            &mut self,
            epoch_change_proofs: Vec<LedgerInfoWithSignatures>,
            target_ledger_info: LedgerInfoWithSignatures,
            target_output_with_proof: TransactionOutputListWithProof,
        ) -> Result<JoinHandle<()>, crate::error::Error>;

        fn pending_storage_data(&self) -> bool;

        fn save_state_values(
            &mut self,
            notification_id: NotificationId,
            state_value_chunk_with_proof: StateValueChunkWithProof,
        ) -> Result<(), crate::error::Error>;

        fn reset_chunk_executor(&mut self) -> Result<(), crate::error::Error>;
    }
    impl Clone for StorageSynchronizer {
        fn clone(&self) -> Self;
    }
}
