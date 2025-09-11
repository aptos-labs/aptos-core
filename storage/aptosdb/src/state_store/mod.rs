// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! This file defines state store APIs that are related account state Merkle tree.

use crate::{
    ledger_db::LedgerDb,
    metrics::{OTHER_TIMERS_SECONDS, STATE_ITEMS, TOTAL_STATE_BYTES},
    pruner::{StateKvPrunerManager, StateMerklePrunerManager},
    schema::{
        db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
        stale_node_index::StaleNodeIndexSchema,
        stale_node_index_cross_epoch::StaleNodeIndexCrossEpochSchema,
        stale_state_value_index::StaleStateValueIndexSchema,
        stale_state_value_index_by_key_hash::StaleStateValueIndexByKeyHashSchema,
        state_value::StateValueSchema,
        state_value_by_key_hash::StateValueByKeyHashSchema,
        version_data::VersionDataSchema,
    },
    state_kv_db::StateKvDb,
    state_merkle_db::StateMerkleDb,
    state_restore::{StateSnapshotRestore, StateSnapshotRestoreMode, StateValueWriter},
    state_store::{buffered_state::BufferedState, persisted_state::PersistedState},
    utils::{
        iterators::PrefixedStateValueIterator,
        truncation_helper::{
            find_tree_root_at_or_before, get_max_version_in_state_merkle_db, truncate_ledger_db,
            truncate_state_kv_db, truncate_state_merkle_db,
        },
        ShardedStateKvSchemaBatch,
    },
};
use aptos_crypto::{
    hash::{CryptoHash, CORRUPTION_SENTINEL, SPARSE_MERKLE_PLACEHOLDER_HASH},
    HashValue,
};
use aptos_db_indexer::db_indexer::InternalIndexerDB;
use aptos_db_indexer_schemas::{
    metadata::{MetadataKey, MetadataValue, StateSnapshotProgress},
    schema::indexer_metadata::InternalIndexerMetadataSchema,
};
use aptos_infallible::Mutex;
use aptos_jellyfish_merkle::iterator::JellyfishMerkleIterator;
use aptos_logger::info;
use aptos_metrics_core::TimerHelper;
use aptos_schemadb::batch::{NativeBatch, SchemaBatch, WriteBatch};
use aptos_scratchpad::SparseMerkleTree;
use aptos_storage_interface::{
    db_ensure as ensure, db_other_bail as bail,
    state_store::{
        state::{LedgerState, State},
        state_summary::{ProvableStateSummary, StateSummary},
        state_update_refs::{PerVersionStateUpdateRefs, StateUpdateRefs},
        state_view::{
            cached_state_view::{ShardedStateCache, StateCacheShard},
            hot_state_view::HotStateView,
        },
        state_with_summary::{LedgerStateWithSummary, StateWithSummary},
        versioned_state_value::StateUpdateRef,
    },
    AptosDbError, DbReader, Result, StateSnapshotReceiver,
};
use aptos_types::{
    proof::{definition::LeafCount, SparseMerkleProofExt, SparseMerkleRangeProof},
    state_store::{
        hot_state::HotStateConfig,
        state_key::{prefix::StateKeyPrefix, StateKey},
        state_slot::StateSlot,
        state_storage_usage::StateStorageUsage,
        state_value::{
            StaleStateValueByKeyHashIndex, StaleStateValueIndex, StateValue,
            StateValueChunkWithProof,
        },
        NUM_STATE_SHARDS,
    },
    transaction::Version,
};
use claims::{assert_ge, assert_le};
use itertools::Itertools;
use rayon::prelude::*;
use std::{
    ops::Deref,
    sync::{Arc, MutexGuard},
};

pub(crate) mod buffered_state;
mod state_merkle_batch_committer;
mod state_snapshot_committer;

pub mod hot_state;
mod persisted_state;
#[cfg(test)]
mod tests;

type StateValueBatch = crate::state_restore::StateValueBatch<StateKey, Option<StateValue>>;

// We assume TARGET_SNAPSHOT_INTERVAL_IN_VERSION > block size.
const MAX_WRITE_SETS_AFTER_SNAPSHOT: LeafCount = buffered_state::TARGET_SNAPSHOT_INTERVAL_IN_VERSION
    * (buffered_state::ASYNC_COMMIT_CHANNEL_BUFFER_SIZE + 2 + 1/*  Rendezvous channel */)
    * 2;

pub const MAX_COMMIT_PROGRESS_DIFFERENCE: u64 = 1_000_000;

pub(crate) struct StateDb {
    pub ledger_db: Arc<LedgerDb>,
    pub state_merkle_db: Arc<StateMerkleDb>,
    pub state_kv_db: Arc<StateKvDb>,
    pub state_merkle_pruner: StateMerklePrunerManager<StaleNodeIndexSchema>,
    pub epoch_snapshot_pruner: StateMerklePrunerManager<StaleNodeIndexCrossEpochSchema>,
    pub state_kv_pruner: StateKvPrunerManager,
    pub skip_usage: bool,
}

pub(crate) struct StateStore {
    pub state_db: Arc<StateDb>,
    /// The `base` of buffered_state is the latest snapshot in state_merkle_db while `current`
    /// is the latest state sparse merkle tree that is replayed from that snapshot until the latest
    /// write set stored in ledger_db.
    buffered_state: Mutex<BufferedState>,
    /// CurrentState is shared between this and the buffered_state.
    /// On read, we don't need to lock the `buffered_state` to get the latest state.
    current_state: Arc<Mutex<LedgerStateWithSummary>>,
    /// Tracks a persisted smt, any state older than that is guaranteed to be found in RocksDB
    persisted_state: PersistedState,
    buffered_state_target_items: usize,
    internal_indexer_db: Option<InternalIndexerDB>,
}

impl Deref for StateStore {
    type Target = StateDb;

    fn deref(&self) -> &Self::Target {
        self.state_db.deref()
    }
}

// "using an Arc<dyn DbReader> as an Arc<dyn StateReader>" is not allowed in stable Rust. Actually we
// want another trait, `StateReader`, which is a subset of `DbReader` here but Rust does not support trait
// upcasting coercion for now. Should change it to a different trait once upcasting is stabilized.
// ref: https://github.com/rust-lang/rust/issues/65991
impl DbReader for StateDb {
    /// Returns the latest state snapshot strictly before `next_version` if any.
    fn get_state_snapshot_before(
        &self,
        next_version: Version,
    ) -> Result<Option<(Version, HashValue)>> {
        self.state_merkle_db
            .get_state_snapshot_version_before(next_version)?
            .map(|ver| Ok((ver, self.state_merkle_db.get_root_hash(ver)?)))
            .transpose()
    }

    /// Get the latest state value of the given key up to the given version. Only used for testing for now
    /// but should replace the `get_value_with_proof_by_version` call for VM execution if just fetch the
    /// value without proof.
    fn get_state_value_by_version(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<Option<StateValue>> {
        Ok(self
            .get_state_value_with_version_by_version(state_key, version)?
            .map(|(_, value)| value))
    }

    /// Gets the latest state value and its corresponding version when it's of the given key up
    /// to the given version.
    fn get_state_value_with_version_by_version(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<Option<(Version, StateValue)>> {
        self.state_kv_db
            .get_state_value_with_version_by_version(state_key, version)
    }

    /// Returns the proof of the given state key and version.
    fn get_state_proof_by_version_ext(
        &self,
        key_hash: &HashValue,
        version: Version,
        root_depth: usize,
    ) -> Result<SparseMerkleProofExt> {
        let (_, proof) = self
            .state_merkle_db
            .get_with_proof_ext(key_hash, version, root_depth)?;
        Ok(proof)
    }

    /// Get the state value with proof given the state key and version
    fn get_state_value_with_proof_by_version_ext(
        &self,
        key_hash: &HashValue,
        version: Version,
        root_depth: usize,
    ) -> Result<(Option<StateValue>, SparseMerkleProofExt)> {
        let (leaf_data, proof) = self
            .state_merkle_db
            .get_with_proof_ext(key_hash, version, root_depth)?;
        Ok((
            match leaf_data {
                Some((_val_hash, (key, ver))) => Some(self.expect_value_by_version(&key, ver)?),
                None => None,
            },
            proof,
        ))
    }

    fn get_state_storage_usage(&self, version: Option<Version>) -> Result<StateStorageUsage> {
        version.map_or(Ok(StateStorageUsage::zero()), |version| {
            Ok(match self.ledger_db.metadata_db().get_usage(version) {
                Ok(data) => data,
                _ => {
                    ensure!(self.skip_usage, "VersionData at {version} is missing.");
                    StateStorageUsage::new_untracked()
                },
            })
        })
    }
}

impl DbReader for StateStore {
    fn get_persisted_state(&self) -> Result<(Arc<dyn HotStateView>, State)> {
        Ok(self.persisted_state.get_state())
    }

    fn get_persisted_state_summary(&self) -> Result<StateSummary> {
        Ok(self.persisted_state.get_state_summary())
    }

    /// Returns the latest state snapshot strictly before `next_version` if any.
    fn get_state_snapshot_before(
        &self,
        next_version: Version,
    ) -> Result<Option<(Version, HashValue)>> {
        self.deref().get_state_snapshot_before(next_version)
    }

    /// Get the latest state value of the given key up to the given version. Only used for testing for now
    /// but should replace the `get_value_with_proof_by_version` call for VM execution if just fetch the
    /// value without proof.
    fn get_state_value_by_version(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<Option<StateValue>> {
        self.deref().get_state_value_by_version(state_key, version)
    }

    /// Gets the latest state value and the its corresponding version when its of the given key up
    /// to the given version.
    fn get_state_value_with_version_by_version(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<Option<(Version, StateValue)>> {
        self.deref()
            .get_state_value_with_version_by_version(state_key, version)
    }

    /// Returns the proof of the given state key and version.
    fn get_state_proof_by_version_ext(
        &self,
        key_hash: &HashValue,
        version: Version,
        root_depth: usize,
    ) -> Result<SparseMerkleProofExt> {
        self.deref()
            .get_state_proof_by_version_ext(key_hash, version, root_depth)
    }

    /// Get the state value with proof extension given the state key and version
    fn get_state_value_with_proof_by_version_ext(
        &self,
        key_hash: &HashValue,
        version: Version,
        root_depth: usize,
    ) -> Result<(Option<StateValue>, SparseMerkleProofExt)> {
        self.deref()
            .get_state_value_with_proof_by_version_ext(key_hash, version, root_depth)
    }
}

impl StateDb {
    fn expect_value_by_version(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<StateValue> {
        self.get_state_value_by_version(state_key, version)
            .and_then(|opt| {
                opt.ok_or_else(|| {
                    AptosDbError::NotFound(format!(
                        "State Value is missing for key {:?} by version {}",
                        state_key, version
                    ))
                })
            })
    }
}

impl StateStore {
    pub fn new(
        ledger_db: Arc<LedgerDb>,
        state_merkle_db: Arc<StateMerkleDb>,
        state_kv_db: Arc<StateKvDb>,
        state_merkle_pruner: StateMerklePrunerManager<StaleNodeIndexSchema>,
        epoch_snapshot_pruner: StateMerklePrunerManager<StaleNodeIndexCrossEpochSchema>,
        state_kv_pruner: StateKvPrunerManager,
        buffered_state_target_items: usize,
        hack_for_tests: bool,
        empty_buffered_state_for_restore: bool,
        skip_usage: bool,
        internal_indexer_db: Option<InternalIndexerDB>,
    ) -> Self {
        if !hack_for_tests && !empty_buffered_state_for_restore {
            Self::sync_commit_progress(
                Arc::clone(&ledger_db),
                Arc::clone(&state_kv_db),
                Arc::clone(&state_merkle_db),
                /*crash_if_difference_is_too_large=*/ true,
            );
        }
        let state_db = Arc::new(StateDb {
            ledger_db,
            state_merkle_db,
            state_kv_db,
            state_merkle_pruner,
            epoch_snapshot_pruner,
            state_kv_pruner,
            skip_usage,
        });
        let current_state = Arc::new(Mutex::new(LedgerStateWithSummary::new_empty(
            HotStateConfig::default(),
        )));
        let persisted_state = PersistedState::new_empty();
        let buffered_state = if empty_buffered_state_for_restore {
            BufferedState::new_at_snapshot(
                &state_db,
                StateWithSummary::new_empty(HotStateConfig::default()),
                buffered_state_target_items,
                current_state.clone(),
                persisted_state.clone(),
            )
        } else {
            Self::create_buffered_state_from_latest_snapshot(
                &state_db,
                buffered_state_target_items,
                hack_for_tests,
                /*check_max_versions_after_snapshot=*/ true,
                current_state.clone(),
                persisted_state.clone(),
            )
            .expect("buffered state creation failed.")
        };

        Self {
            state_db,
            buffered_state: Mutex::new(buffered_state),
            buffered_state_target_items,
            current_state,
            persisted_state,
            internal_indexer_db,
        }
    }

    // We commit the overall commit progress at the last, and use it as the source of truth of the
    // commit progress.
    pub fn sync_commit_progress(
        ledger_db: Arc<LedgerDb>,
        state_kv_db: Arc<StateKvDb>,
        state_merkle_db: Arc<StateMerkleDb>,
        crash_if_difference_is_too_large: bool,
    ) {
        let ledger_metadata_db = ledger_db.metadata_db();
        if let Some(overall_commit_progress) = ledger_metadata_db
            .get_synced_version()
            .expect("DB read failed.")
        {
            info!(
                overall_commit_progress = overall_commit_progress,
                "Start syncing databases..."
            );
            let ledger_commit_progress = ledger_metadata_db
                .get_ledger_commit_progress()
                .expect("Failed to read ledger commit progress.");
            assert_ge!(ledger_commit_progress, overall_commit_progress);

            let state_kv_commit_progress = state_kv_db
                .metadata_db()
                .get::<DbMetadataSchema>(&DbMetadataKey::StateKvCommitProgress)
                .expect("Failed to read state K/V commit progress.")
                .expect("State K/V commit progress cannot be None.")
                .expect_version();
            assert_ge!(state_kv_commit_progress, overall_commit_progress);

            // LedgerCommitProgress was not guaranteed to commit after all ledger changes finish,
            // have to attempt truncating every column family.
            info!(
                ledger_commit_progress = ledger_commit_progress,
                "Attempt ledger truncation...",
            );
            let difference = ledger_commit_progress - overall_commit_progress;
            if crash_if_difference_is_too_large {
                assert_le!(difference, MAX_COMMIT_PROGRESS_DIFFERENCE);
            }
            truncate_ledger_db(ledger_db.clone(), overall_commit_progress)
                .expect("Failed to truncate ledger db.");

            // State K/V commit progress isn't (can't be) written atomically with the data,
            // because there are shards, so we have to attempt truncation anyway.
            info!(
                state_kv_commit_progress = state_kv_commit_progress,
                "Start state KV truncation..."
            );
            let difference = state_kv_commit_progress - overall_commit_progress;
            if crash_if_difference_is_too_large {
                assert_le!(difference, MAX_COMMIT_PROGRESS_DIFFERENCE);
            }
            truncate_state_kv_db(
                &state_kv_db,
                state_kv_commit_progress,
                overall_commit_progress,
                std::cmp::max(difference as usize, 1), /* batch_size */
            )
            .expect("Failed to truncate state K/V db.");

            let state_merkle_max_version = get_max_version_in_state_merkle_db(&state_merkle_db)
                .expect("Failed to get state merkle max version.")
                .expect("State merkle max version cannot be None.");
            if state_merkle_max_version > overall_commit_progress {
                let difference = state_merkle_max_version - overall_commit_progress;
                if crash_if_difference_is_too_large {
                    assert_le!(difference, MAX_COMMIT_PROGRESS_DIFFERENCE);
                }
            }
            let state_merkle_target_version = find_tree_root_at_or_before(
                ledger_metadata_db,
                &state_merkle_db,
                overall_commit_progress,
            )
            .expect("DB read failed.")
            .unwrap_or_else(|| {
                panic!(
                    "Could not find a valid root before or at version {}, maybe it was pruned?",
                    overall_commit_progress
                )
            });
            if state_merkle_target_version < state_merkle_max_version {
                info!(
                    state_merkle_max_version = state_merkle_max_version,
                    target_version = state_merkle_target_version,
                    "Start state merkle truncation..."
                );
                truncate_state_merkle_db(&state_merkle_db, state_merkle_target_version)
                    .expect("Failed to truncate state merkle db.");
            }
        } else {
            info!("No overall commit progress was found!");
        }
    }

    #[cfg(feature = "db-debugger")]
    pub fn catch_up_state_merkle_db(
        ledger_db: Arc<LedgerDb>,
        state_merkle_db: Arc<StateMerkleDb>,
        state_kv_db: Arc<StateKvDb>,
    ) -> Result<Option<Version>> {
        use aptos_config::config::NO_OP_STORAGE_PRUNER_CONFIG;

        let state_merkle_pruner = StateMerklePrunerManager::new(
            Arc::clone(&state_merkle_db),
            NO_OP_STORAGE_PRUNER_CONFIG.state_merkle_pruner_config,
        );
        let epoch_snapshot_pruner = StateMerklePrunerManager::new(
            Arc::clone(&state_merkle_db),
            NO_OP_STORAGE_PRUNER_CONFIG.state_merkle_pruner_config,
        );
        let state_kv_pruner = StateKvPrunerManager::new(
            Arc::clone(&state_kv_db),
            NO_OP_STORAGE_PRUNER_CONFIG.ledger_pruner_config,
        );
        let state_db = Arc::new(StateDb {
            ledger_db,
            state_merkle_db,
            state_kv_db,
            state_merkle_pruner,
            epoch_snapshot_pruner,
            state_kv_pruner,
            skip_usage: false,
        });
        let current_state = Arc::new(Mutex::new(LedgerStateWithSummary::new_empty(
            HotStateConfig::default(),
        )));
        let persisted_state = PersistedState::new_empty();
        let _ = Self::create_buffered_state_from_latest_snapshot(
            &state_db,
            0,
            /*hack_for_tests=*/ false,
            /*check_max_versions_after_snapshot=*/ false,
            current_state.clone(),
            persisted_state,
        )?;
        let base_version = current_state.lock().version();
        Ok(base_version)
    }

    fn create_buffered_state_from_latest_snapshot(
        state_db: &Arc<StateDb>,
        buffered_state_target_items: usize,
        hack_for_tests: bool,
        check_max_versions_after_snapshot: bool,
        out_current_state: Arc<Mutex<LedgerStateWithSummary>>,
        out_persisted_state: PersistedState,
    ) -> Result<BufferedState> {
        let num_transactions = state_db
            .ledger_db
            .metadata_db()
            .get_synced_version()?
            .map_or(0, |v| v + 1);

        let latest_snapshot_version = state_db
            .state_merkle_db
            .get_state_snapshot_version_before(Version::MAX)
            .expect("Failed to query latest node on initialization.");

        info!(
            num_transactions = num_transactions,
            latest_snapshot_version = latest_snapshot_version,
            "Initializing BufferedState."
        );
        let latest_snapshot_root_hash = if let Some(version) = latest_snapshot_version {
            state_db
                .state_merkle_db
                .get_root_hash(version)
                .expect("Failed to query latest checkpoint root hash on initialization.")
        } else {
            *SPARSE_MERKLE_PLACEHOLDER_HASH
        };
        let usage = state_db.get_state_storage_usage(latest_snapshot_version)?;
        let state = StateWithSummary::new_at_version(
            latest_snapshot_version,
            *SPARSE_MERKLE_PLACEHOLDER_HASH, // TODO(HotState): for now hot state always starts from empty upon restart.
            latest_snapshot_root_hash,
            usage,
            HotStateConfig::default(),
        );
        let mut buffered_state = BufferedState::new_at_snapshot(
            state_db,
            state.clone(),
            buffered_state_target_items,
            out_current_state.clone(),
            out_persisted_state.clone(),
        );

        // In some backup-restore tests we hope to open the db without consistency check.
        if hack_for_tests {
            return Ok(buffered_state);
        }

        // Make sure the committed transactions is ahead of the latest snapshot.
        let snapshot_next_version = latest_snapshot_version.map_or(0, |v| v + 1);

        // For non-restore cases, always snapshot_next_version <= num_transactions.
        if snapshot_next_version > num_transactions {
            info!(
                snapshot_next_version = snapshot_next_version,
                num_transactions = num_transactions,
                "snapshot is after latest transaction version. It should only happen in restore mode",
            );
        }

        // Replaying the committed write sets after the latest snapshot.
        if snapshot_next_version < num_transactions {
            if check_max_versions_after_snapshot {
                ensure!(
                    num_transactions - snapshot_next_version <= MAX_WRITE_SETS_AFTER_SNAPSHOT,
                    "Too many versions after state snapshot. snapshot_next_version: {}, num_transactions: {}",
                    snapshot_next_version,
                    num_transactions,
                );
            }
            let write_sets = state_db
                .ledger_db
                .write_set_db()
                .get_write_sets(snapshot_next_version, num_transactions)?;
            let txn_info_iter = state_db
                .ledger_db
                .transaction_info_db()
                .get_transaction_info_iter(snapshot_next_version, write_sets.len())?;
            let last_checkpoint_index = txn_info_iter
                .into_iter()
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .enumerate()
                .filter(|(_idx, txn_info)| txn_info.has_state_checkpoint_hash())
                .next_back()
                .map(|(idx, _)| idx);

            let state_update_refs = StateUpdateRefs::index_write_sets(
                state.next_version(),
                &write_sets,
                write_sets.len(),
                last_checkpoint_index,
            );
            let current_state = out_current_state.lock().clone();
            let (hot_state, state) = out_persisted_state.get_state();
            let (new_state, _state_reads) = current_state.ledger_state().update_with_db_reader(
                &state,
                hot_state,
                &state_update_refs,
                state_db.clone(),
            )?;
            let state_summary = out_persisted_state.get_state_summary();
            let new_state_summary = current_state.ledger_state_summary().update(
                &ProvableStateSummary::new(state_summary, state_db.as_ref()),
                &state_update_refs,
            )?;
            let updated =
                LedgerStateWithSummary::from_state_and_summary(new_state, new_state_summary);

            // synchronously commit the snapshot at the last checkpoint here if not committed to disk yet.
            buffered_state.update(
                updated, 0,    /* estimated_items, doesn't matter since we sync-commit */
                true, /* sync_commit */
            )?;
        }

        let current_state = out_current_state.lock().clone();
        info!(
            latest_in_memory_version = current_state.version(),
            latest_in_memory_root_hash = current_state.summary().root_hash(),
            latest_snapshot_version = current_state.last_checkpoint().version(),
            latest_snapshot_root_hash = current_state.last_checkpoint().summary().root_hash(),
            "StateStore initialization finished.",
        );
        Ok(buffered_state)
    }

    pub fn reset(&self) {
        self.buffered_state.lock().quit();
        *self.buffered_state.lock() = Self::create_buffered_state_from_latest_snapshot(
            &self.state_db,
            self.buffered_state_target_items,
            false,
            true,
            self.current_state.clone(),
            self.persisted_state.clone(),
        )
        .expect("buffered state creation failed.");
    }

    pub fn buffered_state(&self) -> &Mutex<BufferedState> {
        &self.buffered_state
    }

    pub fn current_state_locked(&self) -> MutexGuard<LedgerStateWithSummary> {
        self.current_state.lock()
    }

    /// Returns the key, value pairs for a particular state key prefix at desired version. This
    /// API can be used to get all resources of an account by passing the account address as the
    /// key prefix.
    pub fn get_prefixed_state_value_iterator(
        &self,
        key_prefix: &StateKeyPrefix,
        first_key_opt: Option<&StateKey>,
        desired_version: Version,
    ) -> Result<PrefixedStateValueIterator> {
        // this can only handle non-sharded db scenario.
        // For sharded db, should look at API side using internal indexer to handle this request
        PrefixedStateValueIterator::new(
            &self.state_kv_db,
            key_prefix.clone(),
            first_key_opt.cloned(),
            desired_version,
        )
    }

    /// Gets the proof that proves a range of accounts.
    pub fn get_value_range_proof(
        &self,
        rightmost_key: HashValue,
        version: Version,
    ) -> Result<SparseMerkleRangeProof> {
        self.state_merkle_db.get_range_proof(rightmost_key, version)
    }

    /// Without the executor and execution pipeline, the State and StateSummary for both the
    /// latest version and the last checkpoint version need to be calculated before committing
    /// to the DB. This is useful for the db-restore tooling and tests.
    pub fn calculate_state_and_put_updates(
        &self,
        state_update_refs: &StateUpdateRefs,
        ledger_batch: &mut SchemaBatch,
        sharded_state_kv_batches: &mut ShardedStateKvSchemaBatch,
    ) -> Result<LedgerState> {
        let current = self.current_state_locked().ledger_state();
        let (hot_state, persisted) = self.get_persisted_state()?;
        let (new_state, reads) = current.update_with_db_reader(
            &persisted,
            hot_state,
            state_update_refs,
            self.state_db.clone(),
        )?;

        self.put_state_updates(
            &new_state,
            &state_update_refs.per_version,
            &reads,
            ledger_batch,
            sharded_state_kv_batches,
        )?;

        Ok(new_state)
    }

    pub fn put_state_updates(
        &self,
        state: &LedgerState,
        state_update_refs: &PerVersionStateUpdateRefs,
        state_reads: &ShardedStateCache,
        ledger_batch: &mut SchemaBatch,
        sharded_state_kv_batches: &mut ShardedStateKvSchemaBatch,
    ) -> Result<()> {
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["put_value_sets"]);
        let current_state = self.current_state_locked().state().clone();

        self.put_stats_and_indices(
            &current_state,
            state,
            state_update_refs,
            state_reads,
            ledger_batch,
            sharded_state_kv_batches,
        )?;

        self.put_state_values(state_update_refs, sharded_state_kv_batches)
    }

    pub fn put_state_values(
        &self,
        state_update_refs: &PerVersionStateUpdateRefs,
        sharded_state_kv_batches: &mut ShardedStateKvSchemaBatch,
    ) -> Result<()> {
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["add_state_kv_batch"]);

        // TODO(aldenhu): put by refs; batch put
        sharded_state_kv_batches
            .par_iter_mut()
            .zip_eq(state_update_refs.shards.par_iter())
            .try_for_each(|(batch, updates)| {
                updates
                    .iter()
                    .filter_map(|(key, update)| {
                        update
                            .state_op
                            .as_write_op_opt()
                            .map(|write_op| (key, update.version, write_op))
                    })
                    .try_for_each(|(key, version, write_op)| {
                        if self.state_kv_db.enabled_sharding() {
                            batch.put::<StateValueByKeyHashSchema>(
                                &(CryptoHash::hash(*key), version),
                                &write_op.as_state_value_opt().cloned(),
                            )
                        } else {
                            batch.put::<StateValueSchema>(
                                &((*key).clone(), version),
                                &write_op.as_state_value_opt().cloned(),
                            )
                        }
                    })
            })
    }

    pub fn get_usage(&self, version: Option<Version>) -> Result<StateStorageUsage> {
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["get_usage"]);
        self.state_db.get_state_storage_usage(version)
    }

    /// Put storage usage stats and State key and value indices into the batch.
    /// The state KV indices will be generated as follows:
    /// 1. A deletion at current version is always coupled with stale index for the tombstone with
    /// `stale_since_version` equal to the version, to ensure tombstone is cleared from db after
    /// pruner processes the current version.
    /// 2. An update at current version will first try to find the corresponding old value, if it
    /// exists, a stale index of that old value will be added. Otherwise, it's a no-op. Because
    /// non-existence means either the key never shows up or it got deleted. Neither case needs
    /// extra stale index as 1 cover the latter case.
    pub fn put_stats_and_indices(
        &self,
        current_state: &State,
        latest_state: &LedgerState,
        state_update_refs: &PerVersionStateUpdateRefs,
        // TODO(grao): Restructure this function.
        state_reads: &ShardedStateCache,
        batch: &mut SchemaBatch,
        sharded_state_kv_batches: &mut ShardedStateKvSchemaBatch,
    ) -> Result<()> {
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["put_stats_and_indices"]);

        Self::put_stale_state_value_index(
            state_update_refs,
            sharded_state_kv_batches,
            self.state_kv_db.enabled_sharding(),
            state_reads,
            latest_state.usage().is_untracked() || current_state.version().is_none(), // ignore_state_cache_miss
        );

        {
            let _timer = OTHER_TIMERS_SECONDS.timer_with(&["put_stats_and_indices__put_usage"]);
            if latest_state.last_checkpoint().next_version() > current_state.next_version() {
                // has a checkpoint in the chunk
                Self::put_usage(latest_state.last_checkpoint(), batch)?;
            }
            if !latest_state.is_checkpoint() {
                // latest state isn't a checkpoint
                Self::put_usage(latest_state, batch)?;
            }
            STATE_ITEMS.set(latest_state.usage().items() as i64);
            TOTAL_STATE_BYTES.set(latest_state.usage().bytes() as i64);
        }

        Ok(())
    }

    fn put_stale_state_value_index(
        state_update_refs: &PerVersionStateUpdateRefs,
        sharded_state_kv_batches: &mut ShardedStateKvSchemaBatch,
        enable_sharding: bool,
        sharded_state_cache: &ShardedStateCache,
        ignore_state_cache_miss: bool,
    ) {
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["put_stale_kv_index"]);

        // calculate total state size in bytes
        sharded_state_cache
            .shards
            .par_iter()
            .zip_eq(state_update_refs.shards.par_iter())
            .zip_eq(sharded_state_kv_batches.par_iter_mut())
            .enumerate()
            .for_each(|(shard_id, ((cache, updates), batch))| {
                Self::put_stale_state_value_index_for_shard(
                    shard_id,
                    state_update_refs.first_version,
                    state_update_refs.num_versions,
                    cache,
                    updates,
                    batch,
                    enable_sharding,
                    ignore_state_cache_miss,
                );
            })
    }

    fn put_stale_state_value_index_for_shard<'kv>(
        shard_id: usize,
        first_version: Version,
        num_versions: usize,
        cache: &StateCacheShard,
        updates: &[(&'kv StateKey, StateUpdateRef<'kv>)],
        batch: &mut NativeBatch,
        enable_sharding: bool,
        ignore_state_cache_miss: bool,
    ) {
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&[&format!("put_stale_kv_index__{shard_id}")]);

        let mut iter = updates.iter();
        for version in first_version..first_version + num_versions as Version {
            let ver_iter = iter
                .take_while_ref(|(_k, u)| u.version == version)
                // ignore hot state only ops
                // TODO(HotState): revisit
                .filter(|(_key, update)| update.state_op.is_value_write_op());

            for (key, update_to_cold) in ver_iter {
                if update_to_cold.state_op.expect_as_write_op().is_delete() {
                    // This is a tombstone, can be pruned once this `version` goes out of
                    // the pruning window.
                    Self::put_state_kv_index(batch, enable_sharding, version, version, key);
                }

                // TODO(aldenhu): cache changes here, should consume it.
                let old_entry = cache
                    // TODO(HotState): Revisit: assuming every write op results in a hot slot
                    .insert(
                        (*key).clone(),
                        update_to_cold
                            .to_result_slot()
                            .expect("hot state ops should have been filtered out above"),
                    )
                    .unwrap_or_else(|| {
                        // n.b. all updated state items must be read and recorded in the state cache,
                        // otherwise we can't calculate the correct usage. The is_untracked() hack
                        // is to allow some db tests without real execution layer to pass.
                        assert!(ignore_state_cache_miss, "Must cache read.");
                        StateSlot::ColdVacant
                    });

                if old_entry.is_occupied() {
                    // The value at the old version can be pruned once the pruning window hits
                    // this `version`.
                    Self::put_state_kv_index(
                        batch,
                        enable_sharding,
                        version,
                        old_entry.expect_value_version(),
                        key,
                    )
                }
            }
        }
    }

    fn put_state_kv_index(
        batch: &mut NativeBatch,
        enable_sharding: bool,
        stale_since_version: Version,
        version: Version,
        key: &StateKey,
    ) {
        if enable_sharding {
            batch
                .put::<StaleStateValueIndexByKeyHashSchema>(
                    &StaleStateValueByKeyHashIndex {
                        stale_since_version,
                        version,
                        state_key_hash: key.hash(),
                    },
                    &(),
                )
                .unwrap();
        } else {
            batch
                .put::<StaleStateValueIndexSchema>(
                    &StaleStateValueIndex {
                        stale_since_version,
                        version,
                        state_key: (*key).clone(),
                    },
                    &(),
                )
                .unwrap();
        }
    }

    fn put_usage(state: &State, batch: &mut SchemaBatch) -> Result<()> {
        if let Some(version) = state.version() {
            let usage = state.usage();
            info!("Write usage at version {version}, {usage:?}.");
            batch.put::<VersionDataSchema>(&version, &usage.into())?;
        } else {
            assert_eq!(state.usage().items(), 0);
            assert_eq!(state.usage().bytes(), 0);
        }

        Ok(())
    }

    pub(crate) fn shard_state_value_batch(
        &self,
        sharded_batch: &mut ShardedStateKvSchemaBatch,
        values: &StateValueBatch,
        enable_sharding: bool,
    ) -> Result<()> {
        values.iter().for_each(|((key, version), value)| {
            let shard_id = key.get_shard_id();
            assert!(
                shard_id < NUM_STATE_SHARDS,
                "Invalid shard id: {}",
                shard_id
            );
            if enable_sharding {
                sharded_batch[shard_id]
                    .put::<StateValueByKeyHashSchema>(&(key.hash(), *version), value)
                    .expect("Inserting into sharded schema batch should never fail");
            } else {
                sharded_batch[shard_id]
                    .put::<StateValueSchema>(&(key.clone(), *version), value)
                    .expect("Inserting into sharded schema batch should never fail");
            }
        });
        Ok(())
    }

    pub fn get_root_hash(&self, version: Version) -> Result<HashValue> {
        self.state_merkle_db.get_root_hash(version)
    }

    pub fn get_value_count(&self, version: Version) -> Result<usize> {
        self.state_merkle_db.get_leaf_count(version)
    }

    pub fn get_state_key_and_value_iter(
        self: &Arc<Self>,
        version: Version,
        start_idx: usize,
    ) -> Result<impl Iterator<Item = Result<(StateKey, StateValue)>> + Send + Sync> {
        let store = Arc::clone(self);
        Ok(JellyfishMerkleIterator::new_by_index(
            Arc::clone(&self.state_merkle_db),
            version,
            start_idx,
        )?
        .map(move |res| match res {
            Ok((_hashed_key, (key, version))) => {
                Ok((key.clone(), store.expect_value_by_version(&key, version)?))
            },
            Err(err) => Err(err),
        }))
    }

    pub fn get_value_chunk_with_proof(
        self: &Arc<Self>,
        version: Version,
        first_index: usize,
        chunk_size: usize,
    ) -> Result<StateValueChunkWithProof> {
        let state_key_values: Vec<(StateKey, StateValue)> = self
            .get_value_chunk_iter(version, first_index, chunk_size)?
            .collect::<Result<Vec<_>>>()?;
        self.get_value_chunk_proof(version, first_index, state_key_values)
    }

    pub fn get_value_chunk_iter(
        self: &Arc<Self>,
        version: Version,
        first_index: usize,
        chunk_size: usize,
    ) -> Result<impl Iterator<Item = Result<(StateKey, StateValue)>> + Send + Sync> {
        let store = Arc::clone(self);
        let value_chunk_iter = JellyfishMerkleIterator::new_by_index(
            Arc::clone(&self.state_merkle_db),
            version,
            first_index,
        )?
        .take(chunk_size)
        .map(move |res| {
            res.and_then(|(_, (key, version))| {
                Ok((key.clone(), store.expect_value_by_version(&key, version)?))
            })
        });

        Ok(value_chunk_iter)
    }

    pub fn get_value_chunk_proof(
        self: &Arc<Self>,
        version: Version,
        first_index: usize,
        state_key_values: Vec<(StateKey, StateValue)>,
    ) -> Result<StateValueChunkWithProof> {
        ensure!(
            !state_key_values.is_empty(),
            "State chunk starting at {}",
            first_index,
        );
        let last_index = (state_key_values.len() - 1 + first_index) as u64;
        let first_key = state_key_values.first().expect("checked to exist").0.hash();
        let last_key = state_key_values.last().expect("checked to exist").0.hash();
        let proof = self.get_value_range_proof(last_key, version)?;
        let root_hash = self.get_root_hash(version)?;

        Ok(StateValueChunkWithProof {
            first_index: first_index as u64,
            last_index,
            first_key,
            last_key,
            raw_values: state_key_values,
            proof,
            root_hash,
        })
    }

    // state sync doesn't query for the progress, but keeps its record by itself.
    // TODO: change to async comment once it does like https://github.com/aptos-labs/aptos-core/blob/159b00f3d53e4327523052c1b99dd9889bf13b03/storage/backup/backup-cli/src/backup_types/state_snapshot/restore.rs#L147 or overlap at least two chunks.
    pub fn get_snapshot_receiver(
        self: &Arc<Self>,
        version: Version,
        expected_root_hash: HashValue,
    ) -> Result<Box<dyn StateSnapshotReceiver<StateKey, StateValue>>> {
        Ok(Box::new(StateSnapshotRestore::new(
            &self.state_merkle_db,
            self,
            version,
            expected_root_hash,
            false, /* async_commit */
            StateSnapshotRestoreMode::Default,
        )?))
    }

    #[cfg(test)]
    pub fn get_all_jmt_nodes_referenced(
        &self,
        version: Version,
    ) -> Result<Vec<aptos_jellyfish_merkle::node_type::NodeKey>> {
        aptos_jellyfish_merkle::JellyfishMerkleTree::new(self.state_merkle_db.as_ref())
            .get_all_nodes_referenced(version)
    }

    #[cfg(test)]
    pub fn get_all_jmt_nodes(&self) -> Result<Vec<aptos_jellyfish_merkle::node_type::NodeKey>> {
        let mut iter = self
            .state_db
            .state_merkle_db
            .metadata_db()
            .iter::<crate::schema::jellyfish_merkle_node::JellyfishMerkleNodeSchema>()?;
        iter.seek_to_first();

        let all_rows = iter.collect::<Result<Vec<_>>>()?;

        let mut keys: Vec<aptos_jellyfish_merkle::node_type::NodeKey> =
            all_rows.into_iter().map(|(k, _v)| k).collect();
        if self.state_merkle_db.sharding_enabled() {
            for i in 0..NUM_STATE_SHARDS {
                let mut iter =
                    self.state_merkle_db
                        .db_shard(i)
                        .iter::<crate::schema::jellyfish_merkle_node::JellyfishMerkleNodeSchema>()?;
                iter.seek_to_first();

                let all_rows = iter.collect::<Result<Vec<_>>>()?;
                keys.extend(all_rows.into_iter().map(|(k, _v)| k).collect::<Vec<_>>());
            }
        }
        Ok(keys)
    }

    pub fn init_state_ignoring_summary(&self, version: Option<Version>) -> Result<()> {
        let usage = self.get_usage(version)?;
        // TODO(HotState): pass proper config?
        let state = State::new_at_version(version, usage, HotStateConfig::default());
        let ledger_state = LedgerState::new(state.clone(), state);
        self.set_state_ignoring_summary(ledger_state);

        Ok(())
    }

    pub fn set_state_ignoring_summary(&self, ledger_state: LedgerState) {
        let hot_smt = SparseMerkleTree::new(*CORRUPTION_SENTINEL);
        let smt = SparseMerkleTree::new(*CORRUPTION_SENTINEL);
        let last_checkpoint_summary = StateSummary::new_at_version(
            ledger_state.last_checkpoint().version(),
            hot_smt.clone(),
            smt.clone(),
        );
        let summary = StateSummary::new_at_version(ledger_state.version(), hot_smt, smt);

        let last_checkpoint = StateWithSummary::new(
            ledger_state.last_checkpoint().clone(),
            last_checkpoint_summary.clone(),
        );
        let latest = StateWithSummary::new(ledger_state.latest().clone(), summary);
        let current = LedgerStateWithSummary::from_latest_and_last_checkpoint(
            latest,
            last_checkpoint.clone(),
        );

        self.persisted_state.hack_reset(last_checkpoint.clone());
        *self.current_state_locked() = current;
        self.buffered_state
            .lock()
            .force_last_snapshot(last_checkpoint);
    }
}

impl StateValueWriter<StateKey, StateValue> for StateStore {
    // This already turns on sharded KV
    fn write_kv_batch(
        &self,
        version: Version,
        node_batch: &StateValueBatch,
        progress: StateSnapshotProgress,
    ) -> Result<()> {
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["state_value_writer_write_chunk"]);
        let mut batch = SchemaBatch::new();
        let mut sharded_schema_batch = self.state_kv_db.new_sharded_native_batches();

        batch.put::<DbMetadataSchema>(
            &DbMetadataKey::StateSnapshotKvRestoreProgress(version),
            &DbMetadataValue::StateSnapshotProgress(progress),
        )?;

        if self.internal_indexer_db.is_some()
            && self
                .internal_indexer_db
                .as_ref()
                .unwrap()
                .statekeys_enabled()
        {
            let keys = node_batch.iter().map(|(key, _)| key.0.clone()).collect();
            self.internal_indexer_db
                .as_ref()
                .unwrap()
                .write_keys_to_indexer_db(&keys, version, progress)?;
        }
        self.shard_state_value_batch(
            &mut sharded_schema_batch,
            node_batch,
            self.state_kv_db.enabled_sharding(),
        )?;
        self.state_kv_db
            .commit(version, Some(batch), sharded_schema_batch)
    }

    fn kv_finish(&self, version: Version, usage: StateStorageUsage) -> Result<()> {
        self.ledger_db.metadata_db().put_usage(version, usage)?;
        if let Some(internal_indexer_db) = self.internal_indexer_db.as_ref() {
            if version > 0 {
                let mut batch = SchemaBatch::new();
                batch.put::<InternalIndexerMetadataSchema>(
                    &MetadataKey::LatestVersion,
                    &MetadataValue::Version(version - 1),
                )?;
                if internal_indexer_db.statekeys_enabled() {
                    batch.put::<InternalIndexerMetadataSchema>(
                        &MetadataKey::StateVersion,
                        &MetadataValue::Version(version - 1),
                    )?;
                }
                if internal_indexer_db.transaction_enabled() {
                    batch.put::<InternalIndexerMetadataSchema>(
                        &MetadataKey::TransactionVersion,
                        &MetadataValue::Version(version - 1),
                    )?;
                }
                if internal_indexer_db.event_enabled() {
                    batch.put::<InternalIndexerMetadataSchema>(
                        &MetadataKey::EventVersion,
                        &MetadataValue::Version(version - 1),
                    )?;
                }
                internal_indexer_db
                    .get_inner_db_ref()
                    .write_schemas(batch)?;
            }
        }

        Ok(())
    }

    fn get_progress(&self, version: Version) -> Result<Option<StateSnapshotProgress>> {
        let main_db_progress = self
            .state_kv_db
            .metadata_db()
            .get::<DbMetadataSchema>(&DbMetadataKey::StateSnapshotKvRestoreProgress(version))?
            .map(|v| v.expect_state_snapshot_progress());

        // verify if internal indexer db and main db are consistent before starting the restore
        if self.internal_indexer_db.is_some()
            && self
                .internal_indexer_db
                .as_ref()
                .unwrap()
                .statekeys_enabled()
        {
            let progress_opt = self
                .internal_indexer_db
                .as_ref()
                .unwrap()
                .get_restore_progress(version)?;

            match (main_db_progress, progress_opt) {
                (None, None) => (),
                (None, Some(_)) => (),
                (Some(main_progress), Some(indexer_progress)) => {
                    if main_progress.key_hash > indexer_progress.key_hash {
                        bail!(
                            "Inconsistent restore progress between main db and internal indexer db. main db: {:?}, internal indexer db: {:?}",
                            main_progress,
                            indexer_progress,
                        );
                    }
                },
                _ => {
                    bail!(
                        "Inconsistent restore progress between main db and internal indexer db. main db: {:?}, internal indexer db: {:?}",
                        main_db_progress,
                        progress_opt,
                    );
                },
            }
        }

        Ok(main_db_progress)
    }
}

#[cfg(test)]
mod test_only {
    use crate::state_store::StateStore;
    use aptos_crypto::HashValue;
    use aptos_schemadb::batch::SchemaBatch;
    use aptos_storage_interface::state_store::{
        state_summary::ProvableStateSummary, state_update_refs::StateUpdateRefs,
        state_with_summary::LedgerStateWithSummary,
    };
    use aptos_types::{
        state_store::{state_key::StateKey, state_value::StateValue},
        transaction::Version,
        write_set::{BaseStateOp, WriteOp},
    };
    use itertools::Itertools;

    impl StateStore {
        /// assumes state checkpoint at the last version
        pub fn commit_block_for_test<
            UpdateIter: IntoIterator<Item = (StateKey, Option<StateValue>)>,
            VersionIter: IntoIterator<Item = UpdateIter>,
        >(
            &self,
            first_version: Version,
            updates_by_version: VersionIter,
        ) -> HashValue {
            self.commit_block_for_test_impl(
                first_version,
                updates_by_version.into_iter().map(|updates| {
                    updates.into_iter().map(|(key, val_opt)| {
                        (
                            key,
                            val_opt
                                .map_or_else(
                                    WriteOp::legacy_deletion,
                                    WriteOp::modification_to_value,
                                )
                                .into_base_op(),
                        )
                    })
                }),
            )
        }

        fn commit_block_for_test_impl<
            UpdateIter: IntoIterator<Item = (StateKey, BaseStateOp)>,
            VersionIter: IntoIterator<Item = UpdateIter>,
        >(
            &self,
            first_version: Version,
            updates_by_version: VersionIter,
        ) -> HashValue {
            assert_eq!(first_version, self.current_state_locked().next_version());

            let updates_by_version = updates_by_version
                .into_iter()
                .map(|updates| updates.into_iter().collect_vec())
                .collect_vec();
            let num_versions = updates_by_version.len();
            assert!(num_versions > 0);
            let last_version = first_version + num_versions as Version - 1;

            let state_update_refs = StateUpdateRefs::index(
                first_version,
                updates_by_version
                    .iter()
                    .map(|updates| updates.iter().map(|(k, op)| (k, op))),
                num_versions,
                Some(num_versions - 1),
            );

            let mut ledger_batch = SchemaBatch::new();
            let mut sharded_state_kv_batches = self.state_kv_db.new_sharded_native_batches();

            let new_ledger_state = self
                .calculate_state_and_put_updates(
                    &state_update_refs,
                    &mut ledger_batch,
                    &mut sharded_state_kv_batches,
                )
                .unwrap();

            self.ledger_db
                .metadata_db()
                .write_schemas(ledger_batch)
                .unwrap();
            self.state_kv_db
                .commit(last_version, None, sharded_state_kv_batches)
                .unwrap();

            let current = self.current_state_locked().ledger_state_summary();
            let persisted = self.persisted_state.get_state_summary();

            let new_state_summary = current
                .update(
                    &ProvableStateSummary::new(persisted, self.state_db.as_ref()),
                    &state_update_refs,
                )
                .unwrap();
            let root_hash = new_state_summary.root_hash();

            self.buffered_state
                .lock()
                .update(
                    LedgerStateWithSummary::from_state_and_summary(
                        new_ledger_state,
                        new_state_summary,
                    ),
                    0,    /* estimated_items, doesn't matter since we sync-commit */
                    true, /* sync_commit */
                )
                .unwrap();

            root_hash
        }
    }
}
