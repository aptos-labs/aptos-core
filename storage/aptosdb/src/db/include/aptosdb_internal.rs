// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::CONCURRENCY_GAUGE;
use aptos_metrics_core::IntGaugeHelper;
use aptos_storage_interface::block_info::BlockInfo;

impl AptosDB {
    fn new_with_dbs(
        ledger_db: LedgerDb,
        state_merkle_db: StateMerkleDb,
        state_kv_db: StateKvDb,
        pruner_config: PrunerConfig,
        buffered_state_target_items: usize,
        hack_for_tests: bool,
        empty_buffered_state_for_restore: bool,
        skip_index_and_usage: bool,
        internal_indexer_db: Option<InternalIndexerDB>,
    ) -> Self {
        let ledger_db = Arc::new(ledger_db);
        let state_merkle_db = Arc::new(state_merkle_db);
        let state_kv_db = Arc::new(state_kv_db);
        let state_merkle_pruner = StateMerklePrunerManager::new(
            Arc::clone(&state_merkle_db),
            pruner_config.state_merkle_pruner_config,
        );
        let epoch_snapshot_pruner = StateMerklePrunerManager::new(
            Arc::clone(&state_merkle_db),
            pruner_config.epoch_snapshot_pruner_config.into(),
        );
        let state_kv_pruner =
            StateKvPrunerManager::new(Arc::clone(&state_kv_db), pruner_config.ledger_pruner_config);
        let state_store = Arc::new(StateStore::new(
            Arc::clone(&ledger_db),
            Arc::clone(&state_merkle_db),
            Arc::clone(&state_kv_db),
            state_merkle_pruner,
            epoch_snapshot_pruner,
            state_kv_pruner,
            buffered_state_target_items,
            hack_for_tests,
            empty_buffered_state_for_restore,
            skip_index_and_usage,
            internal_indexer_db.clone(),
        ));

        let ledger_pruner = LedgerPrunerManager::new(
            Arc::clone(&ledger_db),
            pruner_config.ledger_pruner_config,
            internal_indexer_db,
        );

        AptosDB {
            ledger_db: Arc::clone(&ledger_db),
            state_kv_db: Arc::clone(&state_kv_db),
            event_store: Arc::new(EventStore::new(ledger_db.event_db().db_arc())),
            state_store,
            transaction_store: Arc::new(TransactionStore::new(Arc::clone(&ledger_db))),
            ledger_pruner,
            _rocksdb_property_reporter: RocksdbPropertyReporter::new(
                ledger_db,
                state_merkle_db,
                state_kv_db,
            ),
            pre_commit_lock: std::sync::Mutex::new(()),
            commit_lock: std::sync::Mutex::new(()),
            indexer: None,
            skip_index_and_usage,
            update_subscriber: None,
        }
    }

    fn open_internal(
        db_paths: &StorageDirPaths,
        readonly: bool,
        pruner_config: PrunerConfig,
        rocksdb_configs: RocksdbConfigs,
        enable_indexer: bool,
        buffered_state_target_items: usize,
        max_num_nodes_per_lru_cache_shard: usize,
        empty_buffered_state_for_restore: bool,
        internal_indexer_db: Option<InternalIndexerDB>,
    ) -> Result<Self> {
        ensure!(
            pruner_config.eq(&NO_OP_STORAGE_PRUNER_CONFIG) || !readonly,
            "Do not set prune_window when opening readonly.",
        );

        let (ledger_db, state_merkle_db, state_kv_db) = Self::open_dbs(
            db_paths,
            rocksdb_configs,
            readonly,
            max_num_nodes_per_lru_cache_shard,
        )?;

        let mut myself = Self::new_with_dbs(
            ledger_db,
            state_merkle_db,
            state_kv_db,
            pruner_config,
            buffered_state_target_items,
            readonly,
            empty_buffered_state_for_restore,
            rocksdb_configs.enable_storage_sharding,
            internal_indexer_db,
        );

        if !readonly {
            if let Some(version) = myself.get_synced_version()? {
                myself.ledger_pruner
                    .maybe_set_pruner_target_db_version(version);
                myself.state_store
                    .state_kv_pruner
                    .maybe_set_pruner_target_db_version(version);
            }
            if let Some(version) = myself.get_latest_state_checkpoint_version()? {
                myself.state_store.state_merkle_pruner.maybe_set_pruner_target_db_version(version);
                myself.state_store.epoch_snapshot_pruner.maybe_set_pruner_target_db_version(version);
            }
        }

        if !readonly && enable_indexer {
            myself.open_indexer(
                db_paths.default_root_path(),
                rocksdb_configs.index_db_config,
            )?;
        }

        Ok(myself)
    }

    fn open_indexer(
        &mut self,
        db_root_path: impl AsRef<Path>,
        rocksdb_config: RocksdbConfig,
    ) -> Result<()> {
        let indexer = Indexer::open(&db_root_path, rocksdb_config)?;
        let ledger_next_version = self.get_synced_version()?.map_or(0, |v| v + 1);
        info!(
            indexer_next_version = indexer.next_version(),
            ledger_next_version = ledger_next_version,
            "Opened AptosDB Indexer.",
        );

        if indexer.next_version() < ledger_next_version {
            use aptos_storage_interface::state_store::state_view::db_state_view::DbStateViewAtVersion;
            let db: Arc<dyn DbReader> = self.state_store.clone();

            let state_view = db.state_view_at_version(Some(ledger_next_version - 1))?;
            let annotator = AptosValueAnnotator::new(&state_view);

            const BATCH_SIZE: Version = 10000;
            let mut next_version = indexer.next_version();
            while next_version < ledger_next_version {
                info!(next_version = next_version, "AptosDB Indexer catching up. ",);
                let end_version = std::cmp::min(ledger_next_version, next_version + BATCH_SIZE);
                let write_sets = self
                    .ledger_db
                    .write_set_db()
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

    #[cfg(any(test, feature = "fuzzing", feature = "consensus-only-perf-test"))]
    fn new_without_pruner<P: AsRef<Path> + Clone>(
        db_root_path: P,
        readonly: bool,
        buffered_state_target_items: usize,
        max_num_nodes_per_lru_cache_shard: usize,
        enable_indexer: bool,
        enable_sharding: bool,
    ) -> Self {
        Self::open(
            StorageDirPaths::from_path(db_root_path),
            readonly,
            NO_OP_STORAGE_PRUNER_CONFIG, /* pruner */
            RocksdbConfigs {
                enable_storage_sharding: enable_sharding,
                ..Default::default()
            },
            enable_indexer,
            buffered_state_target_items,
            max_num_nodes_per_lru_cache_shard,
            None,
        )
        .expect("Unable to open AptosDB")
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
            .state_merkle_pruner
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
            self.ledger_db.metadata_db().ensure_epoch_ending(version)
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

    fn error_if_state_kv_pruned(&self, data_type: &str, version: Version) -> Result<()> {
        let min_readable_version = self.state_store.state_kv_pruner.get_min_readable_version();
        ensure!(
            version >= min_readable_version,
            "{} at version {} is pruned, min available version is {}.",
            data_type,
            version,
            min_readable_version
        );
        Ok(())
    }

    fn get_raw_block_info_by_height(&self, block_height: u64) -> Result<BlockInfo> {
        if !self.skip_index_and_usage {
            let (first_version, new_block_event) = self.event_store.get_event_by_key(
                &new_block_event_key(),
                block_height,
                self.ensure_synced_version()?,
            )?;
            let new_block_event = bcs::from_bytes(new_block_event.event_data())?;
            Ok(BlockInfo::from_new_block_event(
                first_version,
                &new_block_event,
            ))
        } else {
            Ok(self
                .ledger_db
                .metadata_db()
                .get_block_info(block_height)?
                .ok_or_else(|| {
                    AptosDbError::NotFound(format!("BlockInfo not found at height {block_height}"))
                })?)
        }
    }

    fn get_raw_block_info_by_version(
        &self,
        version: Version,
    ) -> Result<(u64 /* block_height */, BlockInfo)> {
        let synced_version = self.ensure_synced_version()?;
        ensure!(
            version <= synced_version,
            "Requested version {version} > synced version {synced_version}",
        );

        if !self.skip_index_and_usage {
            let (first_version, event_index, block_height) = self
                .event_store
                .lookup_event_before_or_at_version(&new_block_event_key(), version)?
                .ok_or_else(|| AptosDbError::NotFound("NewBlockEvent".to_string()))?;
            let new_block_event = self
                .event_store
                .get_event_by_version_and_index(first_version, event_index)?;
            let new_block_event = bcs::from_bytes(new_block_event.event_data())?;
            Ok((
                block_height,
                BlockInfo::from_new_block_event(first_version, &new_block_event),
            ))
        } else {
            let block_height = self
                .ledger_db
                .metadata_db()
                .get_block_height_by_version(version)?;

            let block_info = self.get_raw_block_info_by_height(block_height)?;
            Ok((block_height, block_info))
        }
    }

    fn to_api_block_info(
        &self,
        block_height: u64,
        block_info: BlockInfo,
    ) -> Result<(Version, Version, NewBlockEvent)> {
        // N.b. Must use committed_version because if synced version is used, we won't be able
        // to tell the end of the latest block.
        let committed_version = self.get_latest_ledger_info_version()?;
        ensure!(
            block_info.first_version() <= committed_version,
            "block first version {} > committed version {committed_version}",
            block_info.first_version(),
        );

        // TODO(grao): Consider return BlockInfo instead of NewBlockEvent.
        let new_block_event = self
            .ledger_db
            .event_db()
            .expect_new_block_event(block_info.first_version())?;

        let last_version = match self.get_raw_block_info_by_height(block_height + 1) {
            Ok(next_block_info) => next_block_info.first_version() - 1,
            Err(AptosDbError::NotFound(..)) => committed_version,
            Err(err) => return Err(err),
        };

        Ok((
            block_info.first_version(),
            last_version,
            bcs::from_bytes(new_block_event.event_data())?,
        ))
    }
}

impl Debug for AptosDB {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("{AptosDB}")
    }
}

fn error_if_too_many_requested(num_requested: u64, max_allowed: u64) -> Result<()> {
    if num_requested > max_allowed {
        Err(AptosDbError::TooManyRequested(num_requested, max_allowed))
    } else {
        Ok(())
    }
}

thread_local! {
    static ENTERED_GAUGED_API: Cell<bool> = const { Cell::new(false) };
}

fn gauged_api<T, F>(api_name: &'static str, api_impl: F) -> Result<T>
where
    F: FnOnce() -> Result<T>,
{
    let nested = ENTERED_GAUGED_API.with(|entered| {
        if entered.get() {
            true
        } else {
            entered.set(true);
            false
        }
    });

    if nested {
        api_impl()
    } else {
        let _guard = CONCURRENCY_GAUGE.concurrency_with(&[api_name]);

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
            },
        };
        API_LATENCY_SECONDS
            .with_label_values(&[api_name, res_type])
            .observe(timer.elapsed().as_secs_f64());
        ENTERED_GAUGED_API.with(|entered| entered.set(false));

        res
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
