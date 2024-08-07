// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    backup::{backup_handler::BackupHandler, restore_utils},
    common::MAX_NUM_EPOCH_ENDING_LEDGER_INFO,
    event_store::EventStore,
    ledger_db::{
        ledger_metadata_db::LedgerMetadataDb,
        transaction_auxiliary_data_db::TransactionAuxiliaryDataDb,
        transaction_info_db::TransactionInfoDb, LedgerDb, LedgerDbSchemaBatches,
    },
    metrics::{
        API_LATENCY_SECONDS, COMMITTED_TXNS, LATEST_TXN_VERSION, LEDGER_VERSION, NEXT_BLOCK_EPOCH,
        OTHER_TIMERS_SECONDS,
    },
    pruner::{LedgerPrunerManager, PrunerManager, StateKvPrunerManager, StateMerklePrunerManager},
    rocksdb_property_reporter::RocksdbPropertyReporter,
    schema::{
        block_info::BlockInfoSchema,
        db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
        transaction_accumulator_root_hash::TransactionAccumulatorRootHashSchema,
    },
    state_kv_db::StateKvDb,
    state_merkle_db::StateMerkleDb,
    state_store::StateStore,
    transaction_store::TransactionStore,
    utils::new_sharded_kv_schema_batch,
};
use aptos_config::config::{
    PrunerConfig, RocksdbConfig, RocksdbConfigs, StorageDirPaths, NO_OP_STORAGE_PRUNER_CONFIG,
};
use aptos_crypto::HashValue;
use aptos_db_indexer::{db_indexer::InternalIndexerDB, Indexer};
use aptos_experimental_runtimes::thread_manager::{optimal_min_len, THREAD_MANAGER};
use aptos_logger::prelude::*;
use aptos_metrics_core::TimerHelper;
use aptos_resource_viewer::AptosValueAnnotator;
use aptos_schemadb::SchemaBatch;
use aptos_scratchpad::SparseMerkleTree;
use aptos_storage_interface::{
    cached_state_view::ShardedStateCache, db_ensure as ensure, db_other_bail as bail,
    state_delta::StateDelta, AptosDbError, DbReader, DbWriter, ExecutedTrees, Order, Result,
    StateSnapshotReceiver, MAX_REQUEST_LIMIT,
};
use aptos_types::{
    account_address::AccountAddress,
    account_config::{new_block_event_key, NewBlockEvent},
    contract_event::{ContractEvent, EventWithVersion},
    epoch_change::EpochChangeProof,
    epoch_state::EpochState,
    event::EventKey,
    ledger_info::LedgerInfoWithSignatures,
    proof::{
        accumulator::InMemoryAccumulator, AccumulatorConsistencyProof, SparseMerkleProofExt,
        TransactionAccumulatorRangeProof, TransactionAccumulatorSummary,
        TransactionInfoListWithProof,
    },
    state_proof::StateProof,
    state_store::{
        state_key::{prefix::StateKeyPrefix, StateKey},
        state_storage_usage::StateStorageUsage,
        state_value::{StateValue, StateValueChunkWithProof},
        table::{TableHandle, TableInfo},
        ShardedStateUpdates,
    },
    transaction::{
        AccountTransactionsWithProof, Transaction, TransactionAuxiliaryData, TransactionInfo,
        TransactionListWithProof, TransactionOutput, TransactionOutputListWithProof,
        TransactionToCommit, TransactionWithProof, Version,
    },
    write_set::WriteSet,
};
use rayon::prelude::*;
use std::{
    cell::Cell,
    fmt::{Debug, Formatter},
    iter::Iterator,
    path::Path,
    sync::Arc,
    time::Instant,
};

#[cfg(test)]
mod aptosdb_test;
#[cfg(any(test, feature = "fuzzing"))]
pub mod test_helper;

/// This holds a handle to the underlying DB responsible for physical storage and provides APIs for
/// access to the core Aptos data structures.
pub struct AptosDB {
    pub(crate) ledger_db: Arc<LedgerDb>,
    pub(crate) state_kv_db: Arc<StateKvDb>,
    pub(crate) event_store: Arc<EventStore>,
    pub(crate) state_store: Arc<StateStore>,
    pub(crate) transaction_store: Arc<TransactionStore>,
    ledger_pruner: LedgerPrunerManager,
    _rocksdb_property_reporter: RocksdbPropertyReporter,
    pre_commit_lock: std::sync::Mutex<()>,
    commit_lock: std::sync::Mutex<()>,
    indexer: Option<Indexer>,
    skip_index_and_usage: bool,
}

// DbReader implementations and private functions used by them.
include!("include/aptosdb_reader.rs");
// DbWriter implementations and private functions used by them.
include!("include/aptosdb_writer.rs");
// Other private methods.
include!("include/aptosdb_internal.rs");
// Testonly methods.
#[cfg(any(test, feature = "fuzzing", feature = "consensus-only-perf-test"))]
include!("include/aptosdb_testonly.rs");

#[cfg(feature = "consensus-only-perf-test")]
pub mod fake_aptosdb;

impl AptosDB {
    pub fn open(
        db_paths: StorageDirPaths,
        readonly: bool,
        pruner_config: PrunerConfig,
        rocksdb_configs: RocksdbConfigs,
        enable_indexer: bool,
        buffered_state_target_items: usize,
        max_num_nodes_per_lru_cache_shard: usize,
        internal_indexer_db: Option<InternalIndexerDB>,
    ) -> Result<Self> {
        Self::open_internal(
            &db_paths,
            readonly,
            pruner_config,
            rocksdb_configs,
            enable_indexer,
            buffered_state_target_items,
            max_num_nodes_per_lru_cache_shard,
            false,
            internal_indexer_db,
        )
    }

    pub fn open_kv_only(
        db_paths: StorageDirPaths,
        readonly: bool,
        pruner_config: PrunerConfig,
        rocksdb_configs: RocksdbConfigs,
        enable_indexer: bool,
        buffered_state_target_items: usize,
        max_num_nodes_per_lru_cache_shard: usize,
        internal_indexer_db: Option<InternalIndexerDB>,
    ) -> Result<Self> {
        Self::open_internal(
            &db_paths,
            readonly,
            pruner_config,
            rocksdb_configs,
            enable_indexer,
            buffered_state_target_items,
            max_num_nodes_per_lru_cache_shard,
            true,
            internal_indexer_db,
        )
    }

    pub fn open_dbs(
        db_paths: &StorageDirPaths,
        rocksdb_configs: RocksdbConfigs,
        readonly: bool,
        max_num_nodes_per_lru_cache_shard: usize,
    ) -> Result<(LedgerDb, StateMerkleDb, StateKvDb)> {
        let ledger_db = LedgerDb::new(db_paths.ledger_db_root_path(), rocksdb_configs, readonly)?;
        let state_kv_db = StateKvDb::new(
            db_paths,
            rocksdb_configs,
            readonly,
            ledger_db.metadata_db_arc(),
        )?;
        let state_merkle_db = StateMerkleDb::new(
            db_paths,
            rocksdb_configs,
            readonly,
            max_num_nodes_per_lru_cache_shard,
        )?;

        Ok((ledger_db, state_merkle_db, state_kv_db))
    }

    /// Gets an instance of `BackupHandler` for data backup purpose.
    pub fn get_backup_handler(&self) -> BackupHandler {
        BackupHandler::new(Arc::clone(&self.state_store), Arc::clone(&self.ledger_db))
    }

    /// Creates new physical DB checkpoint in directory specified by `path`.
    pub fn create_checkpoint(
        db_path: impl AsRef<Path>,
        cp_path: impl AsRef<Path>,
        sharding: bool,
    ) -> Result<()> {
        let start = Instant::now();

        info!(sharding = sharding, "Creating checkpoint for AptosDB.");

        LedgerDb::create_checkpoint(db_path.as_ref(), cp_path.as_ref(), sharding)?;
        if sharding {
            StateKvDb::create_checkpoint(db_path.as_ref(), cp_path.as_ref())?;
        }
        StateMerkleDb::create_checkpoint(db_path.as_ref(), cp_path.as_ref(), sharding)?;

        info!(
            db_path = db_path.as_ref(),
            cp_path = cp_path.as_ref(),
            time_ms = %start.elapsed().as_millis(),
            "Made AptosDB checkpoint."
        );
        Ok(())
    }

    pub fn commit_genesis_ledger_info(&self, genesis_li: &LedgerInfoWithSignatures) -> Result<()> {
        let ledger_metadata_db = self.ledger_db.metadata_db();
        let current_epoch = ledger_metadata_db
            .get_latest_ledger_info_option()
            .map_or(0, |li| li.ledger_info().next_block_epoch());
        ensure!(
            genesis_li.ledger_info().epoch() == current_epoch && current_epoch == 0,
            "Genesis ledger info epoch is not 0"
        );
        let ledger_batch = SchemaBatch::new();
        ledger_metadata_db.put_ledger_info(genesis_li, &ledger_batch)?;
        ledger_metadata_db.write_schemas(ledger_batch)
    }
}
