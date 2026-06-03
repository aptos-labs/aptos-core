// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    db::AptosDB,
    native_state_committer::NativeStateCommitter,
    position_buffered_state::{
        new_empty_position_state, position_state_at_version, PositionLedgerStateWithSummary,
        PositionProofReader, PositionSlot,
    },
    position_db::{PositionDb, NUM_NATIVE_VALUE_SHARDS},
    position_merkle_db::PositionMerkleDb,
    position_state_store::PositionStateStore,
    utils::truncation_helper::{
        get_position_commit_progress, get_position_merkle_commit_progress,
        truncate_position_db_shards, truncate_position_merkle_db,
    },
};
use aptos_config::config::{RocksdbConfig, StorageDirPaths};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_logger::info;
use aptos_schemadb::{Cache, Env};
use aptos_storage_interface::{AptosDbError, Result};
use aptos_types::{state_store::state_value::StateValue, transaction::Version};
use std::{collections::HashMap, sync::Arc};

pub struct PositionBundle {
    pub kv_db: Arc<PositionDb>,
    pub merkle_db: Arc<PositionMerkleDb>,
    /// `None` in readonly mode.
    pub(crate) state_store: Option<Arc<PositionStateStore>>,
    /// JMT version the in-memory MapLayer chain is rooted at — the
    /// version `PositionProofReader` queries. Stays at init time
    /// because the chain doesn't rebase as new snapshots are taken
    /// (the chain just grows). When chain-rebasing lands, update
    /// this alongside the rebase.
    pub(crate) snapshot_version: Option<Version>,
}

impl AptosDB {
    pub fn position(&self) -> Option<&Arc<PositionBundle>> {
        self.position.as_ref()
    }

    pub fn native_state_committer(&self) -> Option<NativeStateCommitter> {
        let bundle = self.position.as_ref()?;
        Some(NativeStateCommitter::new(bundle.kv_db.clone()))
    }

    /// Called automatically from `open_internal` when
    /// `ENABLE_TRADING_NATIVE` is `true`. Shares `env` and
    /// `block_cache` with the main AptosDB so RocksDB background
    /// threads and the block cache stay singleton.
    pub fn init_native_position(
        &mut self,
        db_paths: &StorageDirPaths,
        kv_config: RocksdbConfig,
        merkle_config: RocksdbConfig,
        env: &Env,
        block_cache: &Cache,
        readonly: bool,
    ) -> Result<()> {
        if self.position.is_some() {
            return Err(AptosDbError::Other(
                "init_native_position called twice; native-position subsystem is already \
                 attached to this AptosDB"
                    .to_string(),
            ));
        }

        let position_db =
            PositionDb::new(db_paths, kv_config, Some(env), Some(block_cache), readonly)?;
        let merkle_db = PositionMerkleDb::new(
            db_paths,
            merkle_config,
            Some(env),
            Some(block_cache),
            readonly,
            /* max_nodes_per_lru_cache_shard */ 0,
        )?;

        // Mirror `StateStore::sync_commit_progress`: align both
        // position DBs with the ledger's `OverallCommitProgress`
        // (truncating ahead-of-chain rows from a crash) and find the
        // latest JMT snapshot at or before that point.
        let merkle_progress = if readonly {
            None
        } else {
            self.sync_position_commit_progress(&position_db, &merkle_db)?
        };

        let kv_db = Arc::new(position_db);
        let merkle_db = Arc::new(merkle_db);

        let state_store = if readonly {
            None
        } else {
            let last_snapshot = match merkle_progress {
                Some(version) => {
                    let root_hash = merkle_db.get_root_hash(version)?;
                    position_state_at_version(version, root_hash)
                },
                None => new_empty_position_state(),
            };
            Some(Arc::new(PositionStateStore::new_at_snapshot(
                Arc::clone(&merkle_db),
                Arc::clone(&self.ledger_db),
                last_snapshot,
            )))
        };

        // Replay write sets between the JMT snapshot and the chain
        // tip so the in-memory pipeline + the JMT catch up to
        // `OverallCommitProgress`.
        if let Some(store) = state_store.as_ref()
            && let Some(v_overall) = self.ledger_db.metadata_db().get_synced_version()?
        {
            let snapshot_next_version = merkle_progress.map_or(0, |v| v + 1);
            if snapshot_next_version <= v_overall {
                self.replay_position_after_snapshot(
                    store,
                    &merkle_db,
                    snapshot_next_version,
                    v_overall + 1,
                )?;
            }
        }

        self.position = Some(Arc::new(PositionBundle {
            kv_db,
            merkle_db,
            state_store,
            snapshot_version: merkle_progress,
        }));

        info!(
            num_shards = NUM_NATIVE_VALUE_SHARDS,
            readonly = readonly,
            "Native-position subsystem initialized."
        );

        Ok(())
    }

    /// Align position DB progress with the chain. Truncates `kv_db`
    /// down to `OverallCommitProgress` if it ran ahead (crash between
    /// the position commit and the ledger's commit-progress write),
    /// and returns the merkle DB's latest snapshot version after
    /// truncating it to its own progress. The merkle DB should never
    /// exceed `OverallCommitProgress` in normal operation.
    fn sync_position_commit_progress(
        &self,
        position_db: &PositionDb,
        merkle_db: &PositionMerkleDb,
    ) -> Result<Option<Version>> {
        let v_overall = self.ledger_db.metadata_db().get_synced_version()?;

        if let Some(v_kv) = get_position_commit_progress(position_db)? {
            let target = v_overall.map_or(0, |v| std::cmp::min(v_kv, v));
            if v_kv != target {
                info!(
                    v_kv = v_kv,
                    v_overall = ?v_overall,
                    target = target,
                    "Truncating position_db down to chain's OverallCommitProgress."
                );
            }
            truncate_position_db_shards(position_db, target)?;
        }

        let progress = get_position_merkle_commit_progress(merkle_db)?;
        if let Some(v_merkle) = progress {
            if let Some(v) = v_overall {
                assert!(
                    v_merkle <= v,
                    "position_merkle_db at version {v_merkle} is ahead of chain {v}"
                );
            }
            truncate_position_merkle_db(merkle_db, v_merkle)?;
        }
        Ok(progress)
    }

    /// Replay `WriteSet`s in `[snapshot_next_version, num_transactions)`
    /// — the gap between the persisted JMT snapshot and the chain
    /// tip. Coalesces latest-wins-per-key, extends in one shot, and
    /// sync-commits the resulting snapshot before returning.
    fn replay_position_after_snapshot(
        &self,
        store: &PositionStateStore,
        merkle_db: &Arc<PositionMerkleDb>,
        snapshot_next_version: Version,
        num_transactions: u64,
    ) -> Result<()> {
        info!(
            snapshot_next_version = snapshot_next_version,
            num_transactions = num_transactions,
            "Replaying position write sets to catch up the in-memory pipeline."
        );

        let write_sets = self
            .ledger_db
            .write_set_db()
            .get_write_sets(snapshot_next_version, num_transactions)?;

        let mut pending_leaf_updates: HashMap<HashValue, PositionSlot> = HashMap::new();
        for write_set in &write_sets {
            for (key, op) in write_set.native_position_iter() {
                let maybe_value = op.as_write_op().as_state_value_opt().cloned();
                let value_hash = maybe_value.as_ref().map(StateValue::hash);
                pending_leaf_updates.insert(key.hash(), PositionSlot {
                    state_key: key.clone(),
                    value_hash,
                    value: None,
                });
            }
        }

        if pending_leaf_updates.is_empty() {
            return Ok(());
        }

        let state_lock = store.current_state();
        let pipeline_latest = state_lock.lock().latest().clone();
        let snapshot_version = pipeline_latest.version();

        let target_version = num_transactions - 1;
        let updates: Vec<_> = pending_leaf_updates.into_iter().collect();
        let proof_reader = PositionProofReader {
            merkle_db: Arc::clone(merkle_db),
            version: snapshot_version,
        };
        let new_latest = pipeline_latest.extend(target_version, updates, &proof_reader)?;

        // Treat the target as a checkpoint so the buffered_state
        // sync-commits the JMT snapshot before we return.
        let new_state = PositionLedgerStateWithSummary::from_latest_and_last_checkpoint(
            new_latest.clone(),
            new_latest,
        );
        let mut bufstate = store.buffered_state_locked();
        bufstate.update(
            new_state,
            (),
            write_sets.len(),
            /* sync_commit = */ true,
        )?;
        Ok(())
    }
}
