// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

//! This crate provides [`AptosDB`] which represents physical storage of the core Aptos data
//! structures.
//!
//! It relays read/write operations on the physical storage via [`schemadb`] to the underlying
//! Key-Value storage system, and implements aptos data structures on top of it.

// Used in this and other crates for testing.
#[cfg(any(test, feature = "fuzzing"))]
pub mod test_helper;

pub mod backup;
pub mod errors;
pub mod metrics;
pub mod schema;
pub mod state_restore;

mod db_options;
mod event_store;
mod ledger_store;
mod lru_node_cache;
mod pruner;
mod state_merkle_db;
mod state_store;
mod transaction_store;
mod utils;
mod versioned_node_cache;

#[cfg(test)]
mod aptosdb_test;

#[cfg(any(test, feature = "fuzzing"))]
use crate::state_store::buffered_state::BufferedState;
use crate::{
    backup::{backup_handler::BackupHandler, restore_handler::RestoreHandler, restore_utils},
    db_options::{
        gen_ledger_cfds, gen_state_merkle_cfds, ledger_db_column_families,
        state_merkle_db_column_families,
    },
    errors::AptosDbError,
    event_store::EventStore,
    ledger_store::LedgerStore,
    metrics::{
        API_LATENCY_SECONDS, COMMITTED_TXNS, LATEST_TXN_VERSION, LEDGER_VERSION, NEXT_BLOCK_EPOCH,
        OTHER_TIMERS_SECONDS, ROCKSDB_PROPERTIES,
    },
    pruner::{pruner_manager::PrunerManager, pruner_utils},
    schema::*,
    state_store::StateStore,
    transaction_store::TransactionStore,
};
use anyhow::{bail, ensure, Result};
#[cfg(any(test, feature = "fuzzing"))]
use aptos_config::config::DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD;
use aptos_config::config::{
    PrunerConfig, RocksdbConfig, RocksdbConfigs, NO_OP_STORAGE_PRUNER_CONFIG, TARGET_SNAPSHOT_SIZE,
};

use aptos_crypto::hash::HashValue;
use aptos_infallible::Mutex;
use aptos_logger::prelude::*;
use aptos_rocksdb_options::gen_rocksdb_options;
use aptos_types::proof::TransactionAccumulatorSummary;
use aptos_types::state_store::state_storage_usage::StateStorageUsage;
use aptos_types::{
    account_address::AccountAddress,
    account_config::{new_block_event_key, NewBlockEvent},
    contract_event::EventWithVersion,
    epoch_change::EpochChangeProof,
    epoch_state::EpochState,
    event::EventKey,
    ledger_info::LedgerInfoWithSignatures,
    proof::{
        accumulator::InMemoryAccumulator, AccumulatorConsistencyProof, SparseMerkleProofExt,
        TransactionInfoListWithProof,
    },
    state_proof::StateProof,
    state_store::{
        state_key::StateKey,
        state_key_prefix::StateKeyPrefix,
        state_value::{StateValue, StateValueChunkWithProof},
        table::{TableHandle, TableInfo},
    },
    transaction::{
        AccountTransactionsWithProof, Transaction, TransactionInfo, TransactionListWithProof,
        TransactionOutput, TransactionOutputListWithProof, TransactionToCommit,
        TransactionWithProof, Version,
    },
};
use aptos_vm::data_cache::AsMoveResolver;
use aptosdb_indexer::Indexer;
use itertools::zip_eq;
use move_deps::move_resource_viewer::MoveValueAnnotator;
use once_cell::sync::Lazy;
use schemadb::{SchemaBatch, DB};
use std::{
    collections::HashMap,
    iter::Iterator,
    path::Path,
    sync::{mpsc, Arc},
    thread,
    thread::JoinHandle,
    time::{Duration, Instant},
};

use crate::pruner::{
    ledger_pruner_manager::LedgerPrunerManager, ledger_store::ledger_store_pruner::LedgerPruner,
    state_pruner_manager::StatePrunerManager, state_store::StateMerklePruner,
};
use crate::stale_node_index::StaleNodeIndexSchema;
use crate::stale_node_index_cross_epoch::StaleNodeIndexCrossEpochSchema;
use storage_interface::{
    state_delta::StateDelta, state_view::DbStateView, DbReader, DbWriter, ExecutedTrees, Order,
    StateSnapshotReceiver,
};

pub const LEDGER_DB_NAME: &str = "ledger_db";
pub const STATE_MERKLE_DB_NAME: &str = "state_merkle_db";

// This is last line of defense against large queries slipping through external facing interfaces,
// like the API and State Sync, etc.
const MAX_LIMIT: u64 = 10000;

// TODO: Either implement an iteration API to allow a very old client to loop through a long history
// or guarantee that there is always a recent enough waypoint and client knows to boot from there.
const MAX_NUM_EPOCH_ENDING_LEDGER_INFO: usize = 100;
static ROCKSDB_PROPERTY_MAP: Lazy<HashMap<&str, String>> = Lazy::new(|| {
    [
        "rocksdb.num-immutable-mem-table",
        "rocksdb.mem-table-flush-pending",
        "rocksdb.compaction-pending",
        "rocksdb.background-errors",
        "rocksdb.cur-size-active-mem-table",
        "rocksdb.cur-size-all-mem-tables",
        "rocksdb.size-all-mem-tables",
        "rocksdb.num-entries-active-mem-table",
        "rocksdb.num-entries-imm-mem-tables",
        "rocksdb.num-deletes-active-mem-table",
        "rocksdb.num-deletes-imm-mem-tables",
        "rocksdb.estimate-num-keys",
        "rocksdb.estimate-table-readers-mem",
        "rocksdb.is-file-deletions-enabled",
        "rocksdb.num-snapshots",
        "rocksdb.oldest-snapshot-time",
        "rocksdb.num-live-versions",
        "rocksdb.current-super-version-number",
        "rocksdb.estimate-live-data-size",
        "rocksdb.min-log-number-to-keep",
        "rocksdb.min-obsolete-sst-number-to-keep",
        "rocksdb.total-sst-files-size",
        "rocksdb.live-sst-files-size",
        "rocksdb.base-level",
        "rocksdb.estimate-pending-compaction-bytes",
        "rocksdb.num-running-compactions",
        "rocksdb.num-running-flushes",
        "rocksdb.actual-delayed-write-rate",
        "rocksdb.is-write-stopped",
        "rocksdb.block-cache-capacity",
        "rocksdb.block-cache-usage",
        "rocksdb.block-cache-pinned-usage",
    ]
    .iter()
    .map(|x| (*x, format!("aptos_{}", x.replace('.', "_"))))
    .collect()
});

fn error_if_too_many_requested(num_requested: u64, max_allowed: u64) -> Result<()> {
    if num_requested > max_allowed {
        Err(AptosDbError::TooManyRequested(num_requested, max_allowed).into())
    } else {
        Ok(())
    }
}

fn update_rocksdb_properties(ledger_rocksdb: &DB, state_merkle_rocksdb: &DB) -> Result<()> {
    let _timer = OTHER_TIMERS_SECONDS
        .with_label_values(&["update_rocksdb_properties"])
        .start_timer();
    for cf_name in ledger_db_column_families() {
        for (rockdb_property_name, aptos_rocksdb_property_name) in &*ROCKSDB_PROPERTY_MAP {
            ROCKSDB_PROPERTIES
                .with_label_values(&[cf_name, aptos_rocksdb_property_name])
                .set(ledger_rocksdb.get_property(cf_name, rockdb_property_name)? as i64);
        }
    }
    for cf_name in state_merkle_db_column_families() {
        for (rockdb_property_name, aptos_rocksdb_property_name) in &*ROCKSDB_PROPERTY_MAP {
            ROCKSDB_PROPERTIES
                .with_label_values(&[cf_name, aptos_rocksdb_property_name])
                .set(state_merkle_rocksdb.get_property(cf_name, rockdb_property_name)? as i64);
        }
    }
    Ok(())
}

#[derive(Debug)]
struct RocksdbPropertyReporter {
    sender: Mutex<mpsc::Sender<()>>,
    join_handle: Option<JoinHandle<()>>,
}

impl RocksdbPropertyReporter {
    fn new(ledger_rocksdb: Arc<DB>, state_merkle_rocksdb: Arc<DB>) -> Self {
        let (send, recv) = mpsc::channel();
        let join_handle = Some(thread::spawn(move || loop {
            if let Err(e) = update_rocksdb_properties(&ledger_rocksdb, &state_merkle_rocksdb) {
                warn!(
                    error = ?e,
                    "Updating rocksdb property failed."
                );
            }
            // report rocksdb properties each 10 seconds
            const TIMEOUT_MS: u64 = if cfg!(test) { 10 } else { 10000 };

            match recv.recv_timeout(Duration::from_millis(TIMEOUT_MS)) {
                Ok(_) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => (),
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }));
        Self {
            sender: Mutex::new(send),
            join_handle,
        }
    }
}

impl Drop for RocksdbPropertyReporter {
    fn drop(&mut self) {
        // Notify the property reporting thread to exit
        self.sender.lock().send(()).unwrap();
        self.join_handle
            .take()
            .expect("Rocksdb property reporting thread must exist.")
            .join()
            .expect("Rocksdb property reporting thread should join peacefully.");
    }
}

/// This holds a handle to the underlying DB responsible for physical storage and provides APIs for
/// access to the core Aptos data structures.
#[derive(Debug)]
pub struct AptosDB {
    ledger_db: Arc<DB>,
    state_merkle_db: Arc<DB>,
    event_store: Arc<EventStore>,
    ledger_store: Arc<LedgerStore>,
    state_store: Arc<StateStore>,
    transaction_store: Arc<TransactionStore>,
    ledger_pruner: LedgerPrunerManager,
    _rocksdb_property_reporter: RocksdbPropertyReporter,
    ledger_commit_lock: std::sync::Mutex<()>,
    indexer: Option<Indexer>,
}

impl AptosDB {
    fn new_with_dbs(
        ledger_rocksdb: DB,
        state_merkle_rocksdb: DB,
        pruner_config: PrunerConfig,
        target_snapshot_size: usize,
        max_nodes_per_lru_cache_shard: usize,
        hack_for_tests: bool,
    ) -> Self {
        let arc_ledger_rocksdb = Arc::new(ledger_rocksdb);
        let arc_state_merkle_rocksdb = Arc::new(state_merkle_rocksdb);
        let state_pruner = StatePrunerManager::new(
            Arc::clone(&arc_state_merkle_rocksdb),
            pruner_config.state_merkle_pruner_config,
        );
        let epoch_snapshot_pruner = StatePrunerManager::new(
            Arc::clone(&arc_state_merkle_rocksdb),
            pruner_config.epoch_snapshot_pruner_config.into(),
        );
        let state_store = Arc::new(StateStore::new(
            Arc::clone(&arc_ledger_rocksdb),
            Arc::clone(&arc_state_merkle_rocksdb),
            state_pruner,
            epoch_snapshot_pruner,
            target_snapshot_size,
            max_nodes_per_lru_cache_shard,
            hack_for_tests,
        ));
        let ledger_pruner = LedgerPrunerManager::new(
            Arc::clone(&arc_ledger_rocksdb),
            Arc::clone(&state_store),
            pruner_config.ledger_pruner_config,
        );

        AptosDB {
            ledger_db: Arc::clone(&arc_ledger_rocksdb),
            state_merkle_db: Arc::clone(&arc_state_merkle_rocksdb),
            event_store: Arc::new(EventStore::new(Arc::clone(&arc_ledger_rocksdb))),
            ledger_store: Arc::new(LedgerStore::new(Arc::clone(&arc_ledger_rocksdb))),
            state_store,
            transaction_store: Arc::new(TransactionStore::new(Arc::clone(&arc_ledger_rocksdb))),
            ledger_pruner,
            _rocksdb_property_reporter: RocksdbPropertyReporter::new(
                Arc::clone(&arc_ledger_rocksdb),
                Arc::clone(&arc_state_merkle_rocksdb),
            ),
            ledger_commit_lock: std::sync::Mutex::new(()),
            indexer: None,
        }
    }

    pub fn open<P: AsRef<Path> + Clone>(
        db_root_path: P,
        readonly: bool,
        pruner_config: PrunerConfig,
        rocksdb_configs: RocksdbConfigs,
        enable_indexer: bool,
        target_snapshot_size: usize,
        max_num_nodes_per_lru_cache_shard: usize,
    ) -> Result<Self> {
        ensure!(
            pruner_config.eq(&NO_OP_STORAGE_PRUNER_CONFIG) || !readonly,
            "Do not set prune_window when opening readonly.",
        );

        let ledger_db_path = db_root_path.as_ref().join(LEDGER_DB_NAME);
        let state_merkle_db_path = db_root_path.as_ref().join(STATE_MERKLE_DB_NAME);
        let instant = Instant::now();

        let (ledger_db, state_merkle_db) = if readonly {
            (
                DB::open_cf_readonly(
                    &gen_rocksdb_options(&rocksdb_configs.ledger_db_config, true),
                    ledger_db_path.clone(),
                    LEDGER_DB_NAME,
                    ledger_db_column_families(),
                )?,
                DB::open_cf_readonly(
                    &gen_rocksdb_options(&rocksdb_configs.state_merkle_db_config, true),
                    state_merkle_db_path.clone(),
                    STATE_MERKLE_DB_NAME,
                    state_merkle_db_column_families(),
                )?,
            )
        } else {
            (
                DB::open_cf(
                    &gen_rocksdb_options(&rocksdb_configs.ledger_db_config, false),
                    ledger_db_path.clone(),
                    LEDGER_DB_NAME,
                    gen_ledger_cfds(&rocksdb_configs.ledger_db_config),
                )?,
                DB::open_cf(
                    &gen_rocksdb_options(&rocksdb_configs.state_merkle_db_config, false),
                    state_merkle_db_path.clone(),
                    STATE_MERKLE_DB_NAME,
                    gen_state_merkle_cfds(&rocksdb_configs.state_merkle_db_config),
                )?,
            )
        };

        let mut myself = Self::new_with_dbs(
            ledger_db,
            state_merkle_db,
            pruner_config,
            target_snapshot_size,
            max_num_nodes_per_lru_cache_shard,
            readonly,
        );

        if !readonly && enable_indexer {
            myself.open_indexer(db_root_path, rocksdb_configs.index_db_config)?;
        }

        info!(
            ledger_db_path = ledger_db_path,
            state_merkle_db_path = state_merkle_db_path,
            time_ms = %instant.elapsed().as_millis(),
            "Opened AptosDB (LedgerDB + StateMerkleDB).",
        );
        Ok(myself)
    }

    fn open_indexer(
        &mut self,
        db_root_path: impl AsRef<Path>,
        rocksdb_config: RocksdbConfig,
    ) -> Result<()> {
        let indexer = Indexer::open(&db_root_path, rocksdb_config)?;
        let ledger_next_version = self
            .get_latest_transaction_info_option()?
            .map_or(0, |(v, _)| v + 1);
        info!(
            indexer_next_version = indexer.next_version(),
            ledger_next_version = ledger_next_version,
            "Opened AptosDB Indexer.",
        );

        if indexer.next_version() < ledger_next_version {
            let state_view = DbStateView {
                db: self.state_store.clone(),
                version: Some(ledger_next_version - 1),
            };
            let resolver = state_view.as_move_resolver();
            let annotator = MoveValueAnnotator::new(&resolver);

            const BATCH_SIZE: Version = 10000;
            let mut next_version = indexer.next_version();
            while next_version < ledger_next_version {
                info!(next_version = next_version, "AptosDB Indexer catching up. ",);
                let end_version = std::cmp::min(ledger_next_version, next_version + BATCH_SIZE);
                let write_sets = self
                    .transaction_store
                    .get_write_sets(next_version, end_version)?;
                let write_sets_ref: Vec<_> = write_sets.iter().collect();
                indexer.index_with_annotator(&annotator, next_version, &write_sets_ref)?;

                next_version = end_version;
            }
        }
        info!("AptosDB Indexer caught up.");

        self.indexer = Some(indexer);
        Ok(())
    }

    pub fn open_as_secondary<P: AsRef<Path> + Clone>(
        db_root_path: P,
        secondary_db_root_path: P,
        mut rocksdb_configs: RocksdbConfigs,
    ) -> Result<Self> {
        let ledger_db_primary_path = db_root_path.as_ref().join(LEDGER_DB_NAME);
        let ledger_db_secondary_path = secondary_db_root_path.as_ref().join(LEDGER_DB_NAME);
        let state_merkle_db_primary_path = db_root_path.as_ref().join(STATE_MERKLE_DB_NAME);
        let state_merkle_db_secondary_path =
            secondary_db_root_path.as_ref().join(STATE_MERKLE_DB_NAME);

        // Secondary needs `max_open_files = -1` per
        // https://github.com/facebook/rocksdb/wiki/Read-only-and-Secondary-instances
        rocksdb_configs.ledger_db_config.max_open_files = -1;
        rocksdb_configs.state_merkle_db_config.max_open_files = -1;

        Ok(Self::new_with_dbs(
            DB::open_cf_as_secondary(
                &gen_rocksdb_options(&rocksdb_configs.ledger_db_config, false),
                ledger_db_primary_path,
                ledger_db_secondary_path,
                "ledgerdb_sec",
                ledger_db_column_families(),
            )?,
            DB::open_cf_as_secondary(
                &gen_rocksdb_options(&rocksdb_configs.state_merkle_db_config, false),
                state_merkle_db_primary_path,
                state_merkle_db_secondary_path,
                "state_merkle_db_sec",
                state_merkle_db_column_families(),
            )?,
            NO_OP_STORAGE_PRUNER_CONFIG,
            TARGET_SNAPSHOT_SIZE,
            0,
            true,
        ))
    }

    #[cfg(any(test, feature = "fuzzing"))]
    fn new_without_pruner<P: AsRef<Path> + Clone>(
        db_root_path: P,
        readonly: bool,
        target_snapshot_size: usize,
        max_num_nodes_per_lru_cache_shard: usize,
        enable_indexer: bool,
    ) -> Self {
        Self::open(
            db_root_path,
            readonly,
            NO_OP_STORAGE_PRUNER_CONFIG, /* pruner */
            RocksdbConfigs::default(),
            enable_indexer,
            target_snapshot_size,
            max_num_nodes_per_lru_cache_shard,
        )
        .expect("Unable to open AptosDB")
    }

    /// This opens db in non-readonly mode, without the pruner.
    #[cfg(any(test, feature = "fuzzing"))]
    pub fn new_for_test<P: AsRef<Path> + Clone>(db_root_path: P) -> Self {
        Self::new_without_pruner(
            db_root_path,
            false,
            TARGET_SNAPSHOT_SIZE,
            DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
            false,
        )
    }

    /// This opens db in non-readonly mode, without the pruner and cache.
    #[cfg(any(test, feature = "fuzzing"))]
    pub fn new_for_test_no_cache<P: AsRef<Path> + Clone>(db_root_path: P) -> Self {
        Self::new_without_pruner(db_root_path, false, TARGET_SNAPSHOT_SIZE, 0, false)
    }

    /// This opens db in non-readonly mode, without the pruner, and with the indexer
    #[cfg(any(test, feature = "fuzzing"))]
    pub fn new_for_test_with_indexer<P: AsRef<Path> + Clone>(db_root_path: P) -> Self {
        Self::new_without_pruner(
            db_root_path,
            false,
            TARGET_SNAPSHOT_SIZE,
            DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
            true,
        )
    }

    /// This opens db in non-readonly mode, without the pruner.
    #[cfg(any(test, feature = "fuzzing"))]
    pub fn new_for_test_with_target_snapshot_size<P: AsRef<Path> + Clone>(
        db_root_path: P,
        target_snapshot_size: usize,
    ) -> Self {
        Self::new_without_pruner(
            db_root_path,
            false,
            target_snapshot_size,
            DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
            false,
        )
    }

    /// This opens db in non-readonly mode, without the pruner.
    #[cfg(any(test, feature = "fuzzing"))]
    pub fn new_readonly_for_test<P: AsRef<Path> + Clone>(db_root_path: P) -> Self {
        Self::new_without_pruner(
            db_root_path,
            true,
            TARGET_SNAPSHOT_SIZE,
            DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
            false,
        )
    }

    /// This gets the current buffered_state in StateStore.
    #[cfg(any(test, feature = "fuzzing"))]
    pub fn buffered_state(&self) -> &Mutex<BufferedState> {
        self.state_store.buffered_state()
    }

    /// This force the db to update rocksdb properties immediately.
    pub fn update_rocksdb_properties(&self) -> Result<()> {
        update_rocksdb_properties(&self.ledger_db, &self.state_merkle_db)
    }

    /// Returns ledger infos reflecting epoch bumps starting with the given epoch. If there are no
    /// more than `MAX_NUM_EPOCH_ENDING_LEDGER_INFO` results, this function returns all of them,
    /// otherwise the first `MAX_NUM_EPOCH_ENDING_LEDGER_INFO` results are returned and a flag
    /// (when true) will be used to indicate the fact that there is more.
    fn get_epoch_ending_ledger_infos(
        &self,
        start_epoch: u64,
        end_epoch: u64,
    ) -> Result<(Vec<LedgerInfoWithSignatures>, bool)> {
        self.get_epoch_ending_ledger_infos_impl(
            start_epoch,
            end_epoch,
            MAX_NUM_EPOCH_ENDING_LEDGER_INFO,
        )
    }

    fn get_epoch_ending_ledger_infos_impl(
        &self,
        start_epoch: u64,
        end_epoch: u64,
        limit: usize,
    ) -> Result<(Vec<LedgerInfoWithSignatures>, bool)> {
        ensure!(
            start_epoch <= end_epoch,
            "Bad epoch range [{}, {})",
            start_epoch,
            end_epoch,
        );
        // Note that the latest epoch can be the same with the current epoch (in most cases), or
        // current_epoch + 1 (when the latest ledger_info carries next validator set)
        let latest_epoch = self
            .ledger_store
            .get_latest_ledger_info()?
            .ledger_info()
            .next_block_epoch();
        ensure!(
            end_epoch <= latest_epoch,
            "Unable to provide epoch change ledger info for still open epoch. asked upper bound: {}, last sealed epoch: {}",
            end_epoch,
            latest_epoch - 1,  // okay to -1 because genesis LedgerInfo has .next_block_epoch() == 1
        );

        let (paging_epoch, more) = if end_epoch - start_epoch > limit as u64 {
            (start_epoch + limit as u64, true)
        } else {
            (end_epoch, false)
        };

        let lis = self
            .ledger_store
            .get_epoch_ending_ledger_info_iter(start_epoch, paging_epoch)?
            .collect::<Result<Vec<_>>>()?;
        ensure!(
            lis.len() == (paging_epoch - start_epoch) as usize,
            "DB corruption: missing epoch ending ledger info for epoch {}",
            lis.last()
                .map(|li| li.ledger_info().next_block_epoch() - 1)
                .unwrap_or(start_epoch),
        );
        Ok((lis, more))
    }

    /// Returns the transaction with proof for a given version, or error if the transaction is not
    /// found.
    fn get_transaction_with_proof(
        &self,
        version: Version,
        ledger_version: Version,
        fetch_events: bool,
    ) -> Result<TransactionWithProof> {
        self.error_if_ledger_pruned("Transaction", version)?;

        let proof = self
            .ledger_store
            .get_transaction_info_with_proof(version, ledger_version)?;
        let transaction = self.transaction_store.get_transaction(version)?;

        // If events were requested, also fetch those.
        let events = if fetch_events {
            Some(self.event_store.get_events_by_version(version)?)
        } else {
            None
        };

        Ok(TransactionWithProof {
            version,
            transaction,
            events,
            proof,
        })
    }

    // ================================== Backup APIs ===================================

    /// Gets an instance of `BackupHandler` for data backup purpose.
    pub fn get_backup_handler(&self) -> BackupHandler {
        BackupHandler::new(
            Arc::clone(&self.ledger_store),
            Arc::clone(&self.transaction_store),
            Arc::clone(&self.state_store),
            Arc::clone(&self.event_store),
        )
    }

    /// Creates new physical DB checkpoint in directory specified by `path`.
    pub fn create_checkpoint<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let start = Instant::now();
        let ledger_db_path = path.as_ref().join(LEDGER_DB_NAME);
        let state_merkle_db_path = path.as_ref().join(STATE_MERKLE_DB_NAME);
        self.ledger_db.create_checkpoint(&ledger_db_path)?;
        self.state_merkle_db
            .create_checkpoint(&state_merkle_db_path)?;
        info!(
            path = path.as_ref(),
            time_ms = %start.elapsed().as_millis(),
            "Made AptosDB checkpoint."
        );
        Ok(())
    }

    // ================================== Private APIs ==================================
    fn get_events_by_event_key(
        &self,
        event_key: &EventKey,
        start_seq_num: u64,
        order: Order,
        limit: u64,
        ledger_version: Version,
    ) -> Result<Vec<EventWithVersion>> {
        error_if_too_many_requested(limit, MAX_LIMIT)?;
        let get_latest = order == Order::Descending && start_seq_num == u64::max_value();

        let cursor = if get_latest {
            // Caller wants the latest, figure out the latest seq_num.
            // In the case of no events on that path, use 0 and expect empty result below.
            self.event_store
                .get_latest_sequence_number(ledger_version, event_key)?
                .unwrap_or(0)
        } else {
            start_seq_num
        };

        // Convert requested range and order to a range in ascending order.
        let (first_seq, real_limit) = get_first_seq_num_and_limit(order, cursor, limit)?;

        // Query the index.
        let mut event_indices = self.event_store.lookup_events_by_key(
            event_key,
            first_seq,
            real_limit,
            ledger_version,
        )?;

        // When descending, it's possible that user is asking for something beyond the latest
        // sequence number, in which case we will consider it a bad request and return an empty
        // list.
        // For example, if the latest sequence number is 100, and the caller is asking for 110 to
        // 90, we will get 90 to 100 from the index lookup above. Seeing that the last item
        // is 100 instead of 110 tells us 110 is out of bound.
        if order == Order::Descending {
            if let Some((seq_num, _, _)) = event_indices.last() {
                if *seq_num < cursor {
                    event_indices = Vec::new();
                }
            }
        }

        let mut events_with_version = event_indices
            .into_iter()
            .map(|(seq, ver, idx)| {
                let event = self.event_store.get_event_by_version_and_index(ver, idx)?;
                ensure!(
                    seq == event.sequence_number(),
                    "Index broken, expected seq:{}, actual:{}",
                    seq,
                    event.sequence_number()
                );
                Ok(EventWithVersion::new(ver, event))
            })
            .collect::<Result<Vec<_>>>()?;
        if order == Order::Descending {
            events_with_version.reverse();
        }

        Ok(events_with_version)
    }

    fn save_transactions_impl(
        &self,
        txns_to_commit: &[TransactionToCommit],
        first_version: u64,
        expected_state_db_usage: StateStorageUsage,
        cs: &mut SchemaBatch,
    ) -> Result<HashValue> {
        let last_version = first_version + txns_to_commit.len() as u64 - 1;

        // Account state updates.
        {
            let _timer = OTHER_TIMERS_SECONDS
                .with_label_values(&["save_transactions_state"])
                .start_timer();

            let state_updates_vec = txns_to_commit
                .iter()
                .map(|txn_to_commit| txn_to_commit.state_updates())
                .collect::<Vec<_>>();
            self.state_store.put_value_sets(
                state_updates_vec,
                first_version,
                expected_state_db_usage,
                cs,
            )?;
        }

        // Event updates. Gather event accumulator root hashes.
        {
            let _timer = OTHER_TIMERS_SECONDS
                .with_label_values(&["save_transactions_events"])
                .start_timer();
            zip_eq(first_version..=last_version, txns_to_commit)
                .map(|(ver, txn_to_commit)| {
                    self.event_store.put_events(ver, txn_to_commit.events(), cs)
                })
                .collect::<Result<Vec<_>>>()?;
        }

        let new_root_hash = {
            let _timer = OTHER_TIMERS_SECONDS
                .with_label_values(&["save_transactions_txn_infos"])
                .start_timer();
            zip_eq(first_version..=last_version, txns_to_commit).try_for_each(
                |(ver, txn_to_commit)| {
                    // Transaction updates. Gather transaction hashes.
                    self.transaction_store
                        .put_transaction(ver, txn_to_commit.transaction(), cs)?;
                    self.transaction_store
                        .put_write_set(ver, txn_to_commit.write_set(), cs)
                },
            )?;
            // Transaction accumulator updates. Get result root hash.
            let txn_infos: Vec<_> = txns_to_commit
                .iter()
                .map(|t| t.transaction_info())
                .cloned()
                .collect();
            self.ledger_store
                .put_transaction_infos(first_version, &txn_infos, cs)?
        };
        Ok(new_root_hash)
    }

    /// Write the whole schema batch including all data necessary to mutate the ledger
    /// state of some transaction by leveraging rocksdb atomicity support. Also committed are the
    /// LedgerCounters.
    fn commit(&self, batch: SchemaBatch) -> Result<()> {
        self.ledger_db.write_schemas(batch)?;
        Ok(())
    }

    fn get_table_info_option(&self, handle: TableHandle) -> Result<Option<TableInfo>> {
        match &self.indexer {
            Some(indexer) => indexer.get_table_info(handle),
            None => {
                bail!("Indexer not enabled.");
            }
        }
    }

    fn error_if_ledger_pruned(&self, data_type: &str, version: Version) -> Result<()> {
        let min_readable_version = self.ledger_pruner.get_min_readable_version();
        ensure!(
            version >= min_readable_version,
            "{} at version {} is pruned, min available version is {}.",
            data_type,
            version,
            min_readable_version
        );
        Ok(())
    }

    fn error_if_state_merkle_pruned(&self, data_type: &str, version: Version) -> Result<()> {
        let min_readable_version = self
            .state_store
            .state_db
            .state_pruner
            .get_min_readable_version();
        if version >= min_readable_version {
            return Ok(());
        }

        let min_readable_epoch_snapshot_version = self
            .state_store
            .state_db
            .epoch_snapshot_pruner
            .get_min_readable_version();
        if version >= min_readable_epoch_snapshot_version {
            self.ledger_store.ensure_epoch_ending(version)
        } else {
            bail!(
                "{} at version {} is pruned. snapshots are available at >= {}, epoch snapshots are available at >= {}",
                data_type,
                version,
                min_readable_version,
                min_readable_epoch_snapshot_version,
            )
        }
    }
}

impl DbReader for AptosDB {
    fn get_epoch_ending_ledger_infos(
        &self,
        start_epoch: u64,
        end_epoch: u64,
    ) -> Result<EpochChangeProof> {
        gauged_api("get_epoch_ending_ledger_infos", || {
            let (ledger_info_with_sigs, more) =
                Self::get_epoch_ending_ledger_infos(self, start_epoch, end_epoch)?;
            Ok(EpochChangeProof::new(ledger_info_with_sigs, more))
        })
    }

    fn get_state_values_by_key_prefix(
        &self,
        key_prefix: &StateKeyPrefix,
        version: Version,
    ) -> Result<HashMap<StateKey, StateValue>> {
        gauged_api("get_state_values_by_key_prefix", || {
            self.error_if_ledger_pruned("State", version)?;
            self.state_store
                .get_values_by_key_prefix(key_prefix, version)
        })
    }

    fn get_latest_ledger_info_option(&self) -> Result<Option<LedgerInfoWithSignatures>> {
        gauged_api("get_latest_ledger_info_option", || {
            Ok(self.ledger_store.get_latest_ledger_info_option())
        })
    }

    fn get_account_transaction(
        &self,
        address: AccountAddress,
        seq_num: u64,
        include_events: bool,
        ledger_version: Version,
    ) -> Result<Option<TransactionWithProof>> {
        gauged_api("get_account_transaction", || {
            self.transaction_store
                .get_account_transaction_version(address, seq_num, ledger_version)?
                .map(|txn_version| {
                    self.get_transaction_with_proof(txn_version, ledger_version, include_events)
                })
                .transpose()
        })
    }

    fn get_account_transactions(
        &self,
        address: AccountAddress,
        start_seq_num: u64,
        limit: u64,
        include_events: bool,
        ledger_version: Version,
    ) -> Result<AccountTransactionsWithProof> {
        gauged_api("get_account_transactions", || {
            error_if_too_many_requested(limit, MAX_LIMIT)?;

            let txns_with_proofs = self
                .transaction_store
                .get_account_transaction_version_iter(
                    address,
                    start_seq_num,
                    limit,
                    ledger_version,
                )?
                .map(|result| {
                    let (_seq_num, txn_version) = result?;
                    self.get_transaction_with_proof(txn_version, ledger_version, include_events)
                })
                .collect::<Result<Vec<_>>>()?;

            Ok(AccountTransactionsWithProof::new(txns_with_proofs))
        })
    }

    /// This API is best-effort in that it CANNOT provide absence proof.
    fn get_transaction_by_hash(
        &self,
        hash: HashValue,
        ledger_version: Version,
        fetch_events: bool,
    ) -> Result<Option<TransactionWithProof>> {
        gauged_api("get_transaction_by_hash", || {
            self.transaction_store
                .get_transaction_version_by_hash(&hash, ledger_version)?
                .map(|v| self.get_transaction_with_proof(v, ledger_version, fetch_events))
                .transpose()
        })
    }

    /// Returns the transaction by version, delegates to `AptosDB::get_transaction_with_proof`.
    /// Returns an error if the provided version is not found.
    fn get_transaction_by_version(
        &self,
        version: Version,
        ledger_version: Version,
        fetch_events: bool,
    ) -> Result<TransactionWithProof> {
        gauged_api("get_transaction_by_version", || {
            self.get_transaction_with_proof(version, ledger_version, fetch_events)
        })
    }

    // ======================= State Synchronizer Internal APIs ===================================
    /// Returns batch of transactions for the purpose of synchronizing state to another node.
    ///
    /// If any version beyond ledger_version is requested, it is ignored.
    /// Returns an error if any version <= ledger_version is requested but not found.
    ///
    /// This is used by the State Synchronizer module internally.
    fn get_transactions(
        &self,
        start_version: Version,
        limit: u64,
        ledger_version: Version,
        fetch_events: bool,
    ) -> Result<TransactionListWithProof> {
        gauged_api("get_transactions", || {
            error_if_too_many_requested(limit, MAX_LIMIT)?;

            if start_version > ledger_version || limit == 0 {
                return Ok(TransactionListWithProof::new_empty());
            }
            self.error_if_ledger_pruned("Transaction", start_version)?;

            let limit = std::cmp::min(limit, ledger_version - start_version + 1);

            let txns = (start_version..start_version + limit)
                .map(|version| self.transaction_store.get_transaction(version))
                .collect::<Result<Vec<_>>>()?;
            let txn_infos = (start_version..start_version + limit)
                .map(|version| self.ledger_store.get_transaction_info(version))
                .collect::<Result<Vec<_>>>()?;
            let events = if fetch_events {
                Some(
                    (start_version..start_version + limit)
                        .map(|version| self.event_store.get_events_by_version(version))
                        .collect::<Result<Vec<_>>>()?,
                )
            } else {
                None
            };
            let proof = TransactionInfoListWithProof::new(
                self.ledger_store.get_transaction_range_proof(
                    Some(start_version),
                    limit,
                    ledger_version,
                )?,
                txn_infos,
            );

            Ok(TransactionListWithProof::new(
                txns,
                events,
                Some(start_version),
                proof,
            ))
        })
    }

    fn get_gas_prices(
        &self,
        start_version: Version,
        limit: u64,
        ledger_version: Version,
    ) -> Result<Vec<u64>> {
        const MAX_GAS_LOOKUP: u64 = 100_000;
        gauged_api("get_gas_prices", || {
            error_if_too_many_requested(limit, MAX_GAS_LOOKUP)?;

            if start_version > ledger_version || limit == 0 {
                return Ok(vec![]);
            }

            // This is just an estimation, so we cna just skip over errors
            let limit = std::cmp::min(limit, ledger_version - start_version + 1);
            let txns = self
                .transaction_store
                .get_transaction_iter(start_version, limit as usize)?;
            let gas_prices: Vec<_> = txns
                .filter_map(|txn| {
                    if let Ok(Transaction::UserTransaction(txn)) = txn {
                        Some(txn.gas_unit_price())
                    } else {
                        None
                    }
                })
                .collect();

            Ok(gas_prices)
        })
    }

    /// Get the first version that txn starts existent.
    fn get_first_txn_version(&self) -> Result<Option<Version>> {
        gauged_api("get_first_txn_version", || {
            Ok(Some(self.ledger_pruner.get_min_readable_version()))
        })
    }

    /// Get the first version that will likely not be pruned soon
    fn get_first_viable_txn_version(&self) -> Result<Version> {
        gauged_api("get_first_viable_txn_version", || {
            Ok(self.ledger_pruner.get_min_viable_version())
        })
    }

    /// Get the first version that write set starts existent.
    fn get_first_write_set_version(&self) -> Result<Option<Version>> {
        gauged_api("get_first_write_set_version", || {
            Ok(Some(self.ledger_pruner.get_min_readable_version()))
        })
    }

    /// Returns a batch of transactions for the purpose of synchronizing state to another node.
    ///
    /// If any version beyond ledger_version is requested, it is ignored.
    /// Returns an error if any version <= ledger_version is requested but not found.
    ///
    /// This is used by the State Synchronizer module internally.
    fn get_transaction_outputs(
        &self,
        start_version: Version,
        limit: u64,
        ledger_version: Version,
    ) -> Result<TransactionOutputListWithProof> {
        gauged_api("get_transactions_outputs", || {
            error_if_too_many_requested(limit, MAX_LIMIT)?;

            if start_version > ledger_version || limit == 0 {
                return Ok(TransactionOutputListWithProof::new_empty());
            }

            self.error_if_ledger_pruned("Transaction", start_version)?;

            let limit = std::cmp::min(limit, ledger_version - start_version + 1);

            let (txn_infos, txns_and_outputs) = (start_version..start_version + limit)
                .map(|version| {
                    let txn_info = self.ledger_store.get_transaction_info(version)?;
                    let events = self.event_store.get_events_by_version(version)?;
                    let write_set = self.transaction_store.get_write_set(version)?;
                    let txn = self.transaction_store.get_transaction(version)?;
                    let txn_output = TransactionOutput::new(
                        write_set,
                        events,
                        txn_info.gas_used(),
                        txn_info.status().clone().into(),
                    );
                    Ok((txn_info, (txn, txn_output)))
                })
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .unzip();
            let proof = TransactionInfoListWithProof::new(
                self.ledger_store.get_transaction_range_proof(
                    Some(start_version),
                    limit,
                    ledger_version,
                )?,
                txn_infos,
            );

            Ok(TransactionOutputListWithProof::new(
                txns_and_outputs,
                Some(start_version),
                proof,
            ))
        })
    }

    fn get_events(
        &self,
        event_key: &EventKey,
        start: u64,
        order: Order,
        limit: u64,
        ledger_version: Version,
    ) -> Result<Vec<EventWithVersion>> {
        gauged_api("get_events", || {
            self.get_events_by_event_key(event_key, start, order, limit, ledger_version)
        })
    }

    /// Gets ledger info at specified version and ensures it's an epoch ending.
    fn get_epoch_ending_ledger_info(&self, version: u64) -> Result<LedgerInfoWithSignatures> {
        gauged_api("get_epoch_ending_ledger_info", || {
            self.ledger_store.get_epoch_ending_ledger_info(version)
        })
    }

    fn get_state_proof_with_ledger_info(
        &self,
        known_version: u64,
        ledger_info_with_sigs: LedgerInfoWithSignatures,
    ) -> Result<StateProof> {
        gauged_api("get_state_proof_with_ledger_info", || {
            let ledger_info = ledger_info_with_sigs.ledger_info();
            ensure!(
                known_version <= ledger_info.version(),
                "Client known_version {} larger than ledger version {}.",
                known_version,
                ledger_info.version(),
            );
            let known_epoch = self.ledger_store.get_epoch(known_version)?;
            let end_epoch = ledger_info.next_block_epoch();
            let epoch_change_proof = if known_epoch < end_epoch {
                let (ledger_infos_with_sigs, more) =
                    self.get_epoch_ending_ledger_infos(known_epoch, end_epoch)?;
                EpochChangeProof::new(ledger_infos_with_sigs, more)
            } else {
                EpochChangeProof::new(vec![], /* more = */ false)
            };

            Ok(StateProof::new(ledger_info_with_sigs, epoch_change_proof))
        })
    }

    fn get_state_proof(&self, known_version: u64) -> Result<StateProof> {
        gauged_api("get_state_proof", || {
            let ledger_info_with_sigs = self.ledger_store.get_latest_ledger_info()?;
            self.get_state_proof_with_ledger_info(known_version, ledger_info_with_sigs)
        })
    }

    fn get_state_value_by_version(
        &self,
        state_store_key: &StateKey,
        version: Version,
    ) -> Result<Option<StateValue>> {
        gauged_api("get_state_value_by_version", || {
            self.error_if_ledger_pruned("State", version)?;

            self.state_store
                .get_state_value_by_version(state_store_key, version)
        })
    }

    /// Returns the proof of the given state key and version.
    fn get_state_proof_by_version_ext(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<SparseMerkleProofExt> {
        gauged_api("get_state_proof_by_version_ext", || {
            self.error_if_state_merkle_pruned("State merkle", version)?;

            self.state_store
                .get_state_proof_by_version_ext(state_key, version)
        })
    }

    fn get_state_value_with_proof_by_version_ext(
        &self,
        state_store_key: &StateKey,
        version: Version,
    ) -> Result<(Option<StateValue>, SparseMerkleProofExt)> {
        gauged_api("get_state_value_with_proof_by_version_ext", || {
            self.error_if_state_merkle_pruned("State merkle", version)?;

            self.state_store
                .get_state_value_with_proof_by_version_ext(state_store_key, version)
        })
    }

    fn get_latest_epoch_state(&self) -> Result<EpochState> {
        gauged_api("get_latest_epoch_state", || {
            let latest_ledger_info = self.ledger_store.get_latest_ledger_info()?;
            match latest_ledger_info.ledger_info().next_epoch_state() {
                Some(epoch_state) => Ok(epoch_state.clone()),
                None => self
                    .ledger_store
                    .get_epoch_state(latest_ledger_info.ledger_info().epoch()),
            }
        })
    }

    fn get_latest_executed_trees(&self) -> Result<ExecutedTrees> {
        gauged_api("get_latest_executed_trees", || {
            let buffered_state = self.state_store.buffered_state().lock();
            let num_txns = buffered_state
                .current_state()
                .current_version
                .map_or(0, |v| v + 1);

            let frozen_subtrees = self.ledger_store.get_frozen_subtree_hashes(num_txns)?;
            let transaction_accumulator =
                Arc::new(InMemoryAccumulator::new(frozen_subtrees, num_txns)?);
            let executed_trees = ExecutedTrees::new(
                buffered_state.current_state().clone(),
                transaction_accumulator,
            );
            Ok(executed_trees)
        })
    }

    fn get_block_timestamp(&self, version: u64) -> Result<u64> {
        gauged_api("get_block_timestamp", || {
            self.error_if_ledger_pruned("NewBlockEvent", version)?;
            ensure!(version <= self.get_latest_version()?);

            let (_first_version, new_block_event) = self.event_store.get_block_metadata(version)?;
            Ok(new_block_event.proposed_time())
        })
    }

    fn get_next_block_event(&self, version: Version) -> Result<(Version, NewBlockEvent)> {
        gauged_api("get_next_block_event", || {
            self.error_if_ledger_pruned("NewBlockEvent", version)?;
            if let Some((block_version, _, _)) = self
                .event_store
                .lookup_event_at_or_after_version(&new_block_event_key(), version)?
            {
                self.event_store.get_block_metadata(block_version)
            } else {
                bail!(
                    "Failed to find a block event at or after version {}",
                    version
                )
            }
        })
    }

    fn get_block_info_by_version(
        &self,
        version: Version,
    ) -> Result<(Version, Version, NewBlockEvent)> {
        gauged_api("get_block_info", || {
            self.error_if_ledger_pruned("NewBlockEvent", version)?;

            let latest_li = self.get_latest_ledger_info()?;
            let committed_version = latest_li.ledger_info().version();
            ensure!(
                version <= committed_version,
                "Requested version {} > committed version {}",
                version,
                committed_version
            );

            let (first_version, new_block_event) = self.event_store.get_block_metadata(version)?;

            let last_version = self
                .event_store
                .lookup_event_after_version(&new_block_event_key(), version)?
                .map_or(committed_version, |(v, _, _)| v - 1);

            Ok((first_version, last_version, new_block_event))
        })
    }

    fn get_block_info_by_height(&self, height: u64) -> Result<(Version, Version, NewBlockEvent)> {
        gauged_api("get_block_info_by_height", || {
            let latest_li = self.get_latest_ledger_info()?;
            let committed_version = latest_li.ledger_info().version();

            let event_key = new_block_event_key();
            let (first_version, new_block_event) =
                self.event_store
                    .get_event_by_key(&event_key, height, committed_version)?;
            let last_version = self
                .event_store
                .lookup_event_after_version(&event_key, first_version)?
                .map_or(committed_version, |(v, _, _)| v - 1);

            Ok((
                first_version,
                last_version,
                bcs::from_bytes(new_block_event.event_data())?,
            ))
        })
    }

    fn get_last_version_before_timestamp(
        &self,
        timestamp: u64,
        ledger_version: Version,
    ) -> Result<Version> {
        gauged_api("get_last_version_before_timestamp", || {
            self.event_store
                .get_last_version_before_timestamp(timestamp, ledger_version)
        })
    }

    fn get_latest_transaction_info_option(&self) -> Result<Option<(Version, TransactionInfo)>> {
        gauged_api("get_latest_transaction_info_option", || {
            self.ledger_store.get_latest_transaction_info_option()
        })
    }

    fn get_latest_state_checkpoint_version(&self) -> Result<Option<Version>> {
        gauged_api("get_latest_state_checkpoint_version", || {
            Ok(self
                .state_store
                .buffered_state()
                .lock()
                .current_checkpoint_version())
        })
    }

    fn get_state_snapshot_before(
        &self,
        next_version: Version,
    ) -> Result<Option<(Version, HashValue)>> {
        self.error_if_state_merkle_pruned("State merkle", next_version)?;
        gauged_api("get_state_snapshot_before", || {
            self.state_store.get_state_snapshot_before(next_version)
        })
    }

    fn get_accumulator_root_hash(&self, version: Version) -> Result<HashValue> {
        gauged_api("get_accumulator_root_hash", || {
            self.error_if_ledger_pruned("Transaction accumulator", version)?;
            self.ledger_store.get_root_hash(version)
        })
    }

    fn get_accumulator_consistency_proof(
        &self,
        client_known_version: Option<Version>,
        ledger_version: Version,
    ) -> Result<AccumulatorConsistencyProof> {
        gauged_api("get_accumulator_consistency_proof", || {
            self.error_if_ledger_pruned(
                "Transaction accumulator",
                client_known_version.unwrap_or(0),
            )?;
            self.ledger_store
                .get_consistency_proof(client_known_version, ledger_version)
        })
    }

    fn get_accumulator_summary(
        &self,
        ledger_version: Version,
    ) -> Result<TransactionAccumulatorSummary> {
        let num_txns = ledger_version + 1;
        let frozen_subtrees = self.ledger_store.get_frozen_subtree_hashes(num_txns)?;
        TransactionAccumulatorSummary::new(InMemoryAccumulator::new(frozen_subtrees, num_txns)?)
    }

    fn get_state_leaf_count(&self, version: Version) -> Result<usize> {
        gauged_api("get_state_leaf_count", || {
            self.error_if_state_merkle_pruned("State merkle", version)?;
            self.state_store.get_value_count(version)
        })
    }

    fn get_state_value_chunk_with_proof(
        &self,
        version: Version,
        first_index: usize,
        chunk_size: usize,
    ) -> Result<StateValueChunkWithProof> {
        gauged_api("get_state_value_chunk_with_proof", || {
            self.error_if_state_merkle_pruned("State merkle", version)?;
            self.state_store
                .get_value_chunk_with_proof(version, first_index, chunk_size)
        })
    }

    fn is_state_pruner_enabled(&self) -> Result<bool> {
        gauged_api("is_state_pruner_enabled", || {
            Ok(self.state_store.state_db.state_pruner.is_pruner_enabled())
        })
    }

    fn get_epoch_snapshot_prune_window(&self) -> Result<usize> {
        gauged_api("get_state_prune_window", || {
            Ok(self
                .state_store
                .state_db
                .epoch_snapshot_pruner
                .get_prune_window() as usize)
        })
    }

    fn is_ledger_pruner_enabled(&self) -> Result<bool> {
        gauged_api("is_ledger_pruner_enabled", || {
            Ok(self.ledger_pruner.is_pruner_enabled())
        })
    }

    fn get_ledger_prune_window(&self) -> Result<usize> {
        gauged_api("get_ledger_prune_window", || {
            Ok(self.ledger_pruner.get_prune_window() as usize)
        })
    }

    fn get_table_info(&self, handle: TableHandle) -> Result<TableInfo> {
        gauged_api("get_table_info", || {
            self.get_table_info_option(handle)?
                .ok_or_else(|| AptosDbError::NotFound(format!("TableInfo for {:?}", handle)).into())
        })
    }

    /// Returns whether the indexer DB has been enabled or not
    fn indexer_enabled(&self) -> bool {
        self.indexer.is_some()
    }

    fn get_state_storage_usage(&self, version: Option<Version>) -> Result<StateStorageUsage> {
        gauged_api("get_state_storage_usage", || {
            if let Some(v) = version {
                self.error_if_ledger_pruned("state storage usage", v)?;
            }
            self.state_store.get_usage(version)
        })
    }
}

impl DbWriter for AptosDB {
    /// `first_version` is the version of the first transaction in `txns_to_commit`.
    /// When `ledger_info_with_sigs` is provided, verify that the transaction accumulator root hash
    /// it carries is generated after the `txns_to_commit` are applied.
    /// Note that even if `txns_to_commit` is empty, `first_version` is checked to be
    /// `ledger_info_with_sigs.ledger_info.version + 1` if `ledger_info_with_sigs` is not `None`.
    fn save_transactions(
        &self,
        txns_to_commit: &[TransactionToCommit],
        first_version: Version,
        base_state_version: Option<Version>,
        ledger_info_with_sigs: Option<&LedgerInfoWithSignatures>,
        sync_commit: bool,
        latest_in_memory_state: StateDelta,
    ) -> Result<()> {
        gauged_api("save_transactions", || {
            // Executing and committing from more than one threads not allowed -- consensus and
            // state sync must hand over to each other after all pending execution and committing
            // complete.
            let _lock = self.ledger_commit_lock.lock();

            let num_txns = txns_to_commit.len() as u64;
            // ledger_info_with_sigs could be None if we are doing state synchronization. In this case
            // txns_to_commit should not be empty. Otherwise it is okay to commit empty blocks.
            ensure!(
                ledger_info_with_sigs.is_some() || num_txns > 0,
                "txns_to_commit is empty while ledger_info_with_sigs is None.",
            );

            let last_version = first_version + num_txns - 1;

            if let Some(x) = ledger_info_with_sigs {
                let claimed_last_version = x.ledger_info().version();
                ensure!(
                    claimed_last_version  == last_version,
                    "Transaction batch not applicable: first_version {}, num_txns {}, last_version_in_ledger_info {}",
                    first_version,
                    num_txns,
                    claimed_last_version,
                );
            }

            // Gather db mutations to `batch`.
            let mut batch = SchemaBatch::new();

            let new_root_hash = self.save_transactions_impl(
                txns_to_commit,
                first_version,
                latest_in_memory_state.current.usage(),
                &mut batch,
            )?;

            // If expected ledger info is provided, verify result root hash and save the ledger info.
            if let Some(x) = ledger_info_with_sigs {
                let expected_root_hash = x.ledger_info().transaction_accumulator_hash();
                ensure!(
                    new_root_hash == expected_root_hash,
                    "Root hash calculated doesn't match expected. {:?} vs {:?}",
                    new_root_hash,
                    expected_root_hash,
                );

                self.ledger_store.put_ledger_info(x, &mut batch)?;
            }

            ensure!(Some(last_version) == latest_in_memory_state.current_version,
                "the last_version {:?} to commit doesn't match the current_version {:?} in latest_in_memory_state",
                last_version,
               latest_in_memory_state.current_version.expect("Must exist")
            );

            // Persist.
            {
                let _timer = OTHER_TIMERS_SECONDS
                    .with_label_values(&["save_transactions_commit"])
                    .start_timer();
                self.commit(batch)?;
            }

            {
                let mut buffered_state = self.state_store.buffered_state().lock();
                ensure!(
                    base_state_version == buffered_state.current_state().base_version,
                    "base_state_version {:?} does not equal to the base_version {:?} in buffered state with current version {:?}",
                    base_state_version,
                    buffered_state.current_state().base_version,
                    buffered_state.current_state().current_version,
                );
                let mut end_with_reconfig = false;
                let updates_until_latest_checkpoint_since_current = if let Some(
                    latest_checkpoint_version,
                ) =
                    latest_in_memory_state.base_version
                {
                    if latest_checkpoint_version >= first_version {
                        let idx = (latest_checkpoint_version - first_version) as usize;
                        ensure!(
                            txns_to_commit[idx].is_state_checkpoint(),
                            "The new latest snapshot version passed in {:?} does not match with the last checkpoint version in txns_to_commit {:?}",
                            latest_checkpoint_version,
                            first_version + idx as u64
                    );
                        end_with_reconfig = txns_to_commit[idx].is_reconfig();
                        Some(
                            txns_to_commit[..=idx]
                                .iter()
                                .flat_map(|txn_to_commit| txn_to_commit.state_updates().clone())
                                .collect(),
                        )
                    } else {
                        None
                    }
                } else {
                    None
                };
                buffered_state.update(
                    updates_until_latest_checkpoint_since_current,
                    latest_in_memory_state,
                    end_with_reconfig || sync_commit,
                )?;
            }

            // If commit succeeds and there are at least one transaction written to the storage, we
            // will inform the pruner thread to work.
            if num_txns > 0 {
                let last_version = first_version + num_txns - 1;
                COMMITTED_TXNS.inc_by(num_txns);
                LATEST_TXN_VERSION.set(last_version as i64);
                // Activate the ledger pruner. Note the state merkle pruner is activated when
                // state snapshots are persisted in their async thread.
                self.ledger_pruner
                    .maybe_set_pruner_target_db_version(last_version);
            }

            // Note: this must happen after txns have been saved to db because types can be newly
            // created in this same chunk of transactions.
            if let Some(indexer) = &self.indexer {
                let write_sets: Vec<_> = txns_to_commit.iter().map(|txn| txn.write_set()).collect();
                indexer.index(self.state_store.clone(), first_version, &write_sets)?;
            }

            // Once everything is successfully persisted, update the latest in-memory ledger info.
            if let Some(x) = ledger_info_with_sigs {
                self.ledger_store.set_latest_ledger_info(x.clone());

                LEDGER_VERSION.set(x.ledger_info().version() as i64);
                NEXT_BLOCK_EPOCH.set(x.ledger_info().next_block_epoch() as i64);
            }

            Ok(())
        })
    }

    fn get_state_snapshot_receiver(
        &self,
        version: Version,
        expected_root_hash: HashValue,
    ) -> Result<Box<dyn StateSnapshotReceiver<StateKey, StateValue>>> {
        gauged_api("get_state_snapshot_receiver", || {
            self.state_store
                .get_snapshot_receiver(version, expected_root_hash)
        })
    }

    fn finalize_state_snapshot(
        &self,
        version: Version,
        output_with_proof: TransactionOutputListWithProof,
        ledger_infos: &[LedgerInfoWithSignatures],
    ) -> Result<()> {
        gauged_api("finalize_state_snapshot", || {
            // Ensure the output with proof only contains a single transaction output and info
            let num_transaction_outputs = output_with_proof.transactions_and_outputs.len();
            let num_transaction_infos = output_with_proof.proof.transaction_infos.len();
            ensure!(
                num_transaction_outputs == 1,
                "Number of transaction outputs should == 1, but got: {}",
                num_transaction_outputs
            );
            ensure!(
                num_transaction_infos == 1,
                "Number of transaction infos should == 1, but got: {}",
                num_transaction_infos
            );

            // TODO(joshlind): include confirm_or_save_frozen_subtrees in the change set
            // bundle below.

            // Update the merkle accumulator using the given proof
            let frozen_subtrees = output_with_proof
                .proof
                .ledger_info_to_transaction_infos_proof
                .left_siblings();
            restore_utils::confirm_or_save_frozen_subtrees(
                self.ledger_db.clone(),
                version,
                frozen_subtrees,
                None,
            )?;

            // Create a single change set for all further write operations
            let mut batch = SchemaBatch::new();

            // Save the target transactions, outputs, infos and events
            let (transactions, outputs): (Vec<Transaction>, Vec<TransactionOutput>) =
                output_with_proof
                    .transactions_and_outputs
                    .into_iter()
                    .unzip();
            let events = outputs
                .clone()
                .into_iter()
                .map(|output| output.events().to_vec())
                .collect::<Vec<_>>();
            let transaction_infos = output_with_proof.proof.transaction_infos;
            restore_utils::save_transactions(
                self.ledger_db.clone(),
                self.ledger_store.clone(),
                self.transaction_store.clone(),
                self.event_store.clone(),
                version,
                &transactions,
                &transaction_infos,
                &events,
                Some(&mut batch),
            )?;
            restore_utils::save_transaction_outputs(
                self.ledger_db.clone(),
                self.transaction_store.clone(),
                version,
                outputs,
                Some(&mut batch),
            )?;

            // Save the epoch ending ledger infos
            restore_utils::save_ledger_infos(
                self.ledger_db.clone(),
                self.ledger_store.clone(),
                ledger_infos,
                Some(&mut batch),
            )?;

            // Delete the genesis transaction
            StateMerklePruner::prune_genesis(self.state_merkle_db.clone(), &mut batch)?;
            LedgerPruner::prune_genesis(
                self.ledger_db.clone(),
                self.state_store.clone(),
                &mut batch,
            )?;

            // Apply the change set writes to the database (atomically) and update in-memory state
            self.ledger_db.clone().write_schemas(batch)?;
            restore_utils::update_latest_ledger_info(self.ledger_store.clone(), ledger_infos)?;
            self.state_store.reset();

            Ok(())
        })
    }
}

// Convert requested range and order to a range in ascending order.
fn get_first_seq_num_and_limit(order: Order, cursor: u64, limit: u64) -> Result<(u64, u64)> {
    ensure!(limit > 0, "limit should > 0, got {}", limit);

    Ok(if order == Order::Ascending {
        (cursor, limit)
    } else if limit <= cursor {
        (cursor - limit + 1, limit)
    } else {
        (0, cursor + 1)
    })
}

pub trait GetRestoreHandler {
    /// Gets an instance of `RestoreHandler` for data restore purpose.
    fn get_restore_handler(&self) -> RestoreHandler;
}

impl GetRestoreHandler for Arc<AptosDB> {
    fn get_restore_handler(&self) -> RestoreHandler {
        RestoreHandler::new(
            Arc::clone(&self.ledger_db),
            Arc::clone(self),
            Arc::clone(&self.ledger_store),
            Arc::clone(&self.transaction_store),
            Arc::clone(&self.state_store),
            Arc::clone(&self.event_store),
        )
    }
}

fn gauged_api<T, F>(api_name: &'static str, api_impl: F) -> Result<T>
where
    F: FnOnce() -> Result<T>,
{
    let timer = Instant::now();

    let res = api_impl();

    let res_type = match &res {
        Ok(_) => "Ok",
        Err(e) => {
            warn!(
                api_name = api_name,
                error = ?e,
                "AptosDB API returned error."
            );
            "Err"
        }
    };
    API_LATENCY_SECONDS
        .with_label_values(&[api_name, res_type])
        .observe(timer.elapsed().as_secs_f64());

    res
}
