// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

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
        ));

        let ledger_pruner =
            LedgerPrunerManager::new(Arc::clone(&ledger_db), pruner_config.ledger_pruner_config);

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
            ledger_commit_lock: std::sync::Mutex::new(()),
            indexer: None,
            skip_index_and_usage,
            indexer_async_v2: None,
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
        enable_indexer_async_v2: bool,
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
        );

        if !readonly && enable_indexer {
            myself.open_indexer(
                db_paths.default_root_path(),
                rocksdb_configs.index_db_config,
            )?;
        }

        if enable_indexer_async_v2 {
            myself.open_indexer_async_v2(
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
        let ledger_next_version = self.get_latest_version().map_or(0, |v| v + 1);
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

    fn open_indexer_async_v2(
        &mut self,
        db_root_path: impl AsRef<Path>,
        rocksdb_config: RocksdbConfig,
    ) -> Result<()> {
        let indexer_async_v2 = IndexerAsyncV2::open(db_root_path, rocksdb_config, DashMap::new())?;
        self.indexer_async_v2 = Some(indexer_async_v2);
        Ok(())
    }

    #[cfg(any(test, feature = "fuzzing"))]
    fn new_without_pruner<P: AsRef<Path> + Clone>(
        db_root_path: P,
        readonly: bool,
        buffered_state_target_items: usize,
        max_num_nodes_per_lru_cache_shard: usize,
        enable_indexer: bool,
        enable_indexer_async_v2: bool,
    ) -> Self {
        Self::open(
            StorageDirPaths::from_path(db_root_path),
            readonly,
            NO_OP_STORAGE_PRUNER_CONFIG, /* pruner */
            RocksdbConfigs::default(),
            enable_indexer,
            buffered_state_target_items,
            max_num_nodes_per_lru_cache_shard,
            enable_indexer_async_v2,
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
        },
    };
    API_LATENCY_SECONDS
        .with_label_values(&[api_name, res_type])
        .observe(timer.elapsed().as_secs_f64());

    res
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
