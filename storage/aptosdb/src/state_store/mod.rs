// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This file defines state store APIs that are related account state Merkle tree.

use crate::{
    db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
    epoch_by_version::EpochByVersionSchema,
    metrics::{STATE_ITEMS, TOTAL_STATE_BYTES},
    schema::state_value::StateValueSchema,
    stale_state_value_index::StaleStateValueIndexSchema,
    state_merkle_db::StateMerkleDb,
    state_restore::{StateSnapshotProgress, StateSnapshotRestore, StateValueWriter},
    state_store::buffered_state::BufferedState,
    version_data::VersionDataSchema,
    AptosDbError, LedgerStore, StaleNodeIndexCrossEpochSchema, StaleNodeIndexSchema,
    StatePrunerManager, TransactionStore, OTHER_TIMERS_SECONDS,
};
use anyhow::{anyhow, ensure, format_err, Result};
use aptos_crypto::{
    hash::{CryptoHash, SPARSE_MERKLE_PLACEHOLDER_HASH},
    HashValue,
};
use aptos_infallible::Mutex;
use aptos_jellyfish_merkle::iterator::JellyfishMerkleIterator;
use aptos_logger::info;
use aptos_state_view::StateViewId;
use aptos_types::{
    proof::{definition::LeafCount, SparseMerkleProofExt, SparseMerkleRangeProof},
    state_store::{
        state_key::StateKey,
        state_key_prefix::StateKeyPrefix,
        state_storage_usage::StateStorageUsage,
        state_value::{StaleStateValueIndex, StateValue, StateValueChunkWithProof},
    },
    transaction::Version,
};
use executor_types::in_memory_state_calculator::InMemoryStateCalculator;
use once_cell::sync::Lazy;
use schemadb::{ReadOptions, SchemaBatch, DB};
use std::{
    collections::{HashMap, HashSet},
    ops::Deref,
    sync::Arc,
};
use storage_interface::{
    cached_state_view::CachedStateView, state_delta::StateDelta,
    sync_proof_fetcher::SyncProofFetcher, DbReader, StateSnapshotReceiver,
};

pub(crate) mod buffered_state;
mod state_merkle_batch_committer;
mod state_snapshot_committer;

#[cfg(test)]
mod state_store_test;

type StateValueBatch = crate::state_restore::StateValueBatch<StateKey, Option<StateValue>>;

pub const MAX_VALUES_TO_FETCH_FOR_KEY_PREFIX: usize = 10_000;
// We assume TARGET_SNAPSHOT_INTERVAL_IN_VERSION > block size.
const MAX_WRITE_SETS_AFTER_SNAPSHOT: LeafCount = buffered_state::TARGET_SNAPSHOT_INTERVAL_IN_VERSION
    * (buffered_state::ASYNC_COMMIT_CHANNEL_BUFFER_SIZE + 2 + 1/*  Rendezvous channel */)
    * 2;

static IO_POOL: Lazy<rayon::ThreadPool> = Lazy::new(|| {
    rayon::ThreadPoolBuilder::new()
        .num_threads(32)
        .thread_name(|index| format!("kv_reader_{}", index))
        .build()
        .unwrap()
});

#[derive(Debug)]
pub(crate) struct StateDb {
    pub ledger_db: Arc<DB>,
    pub state_merkle_db: Arc<StateMerkleDb>,
    pub state_pruner: StatePrunerManager<StaleNodeIndexSchema>,
    pub epoch_snapshot_pruner: StatePrunerManager<StaleNodeIndexCrossEpochSchema>,
}

#[derive(Debug)]
pub(crate) struct StateStore {
    pub(crate) state_db: Arc<StateDb>,
    // The `base` of buffered_state is the latest snapshot in state_merkle_db while `current`
    // is the latest state sparse merkle tree that is replayed from that snapshot until the latest
    // write set stored in ledger_db.
    buffered_state: Mutex<BufferedState>,
    target_snapshot_size: usize,
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

    /// Returns the proof of the given state key and version.
    fn get_state_proof_by_version_ext(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<SparseMerkleProofExt> {
        let (_, proof) = self
            .state_merkle_db
            .get_with_proof_ext(state_key, version)?;
        Ok(proof)
    }

    /// Get the state value with proof given the state key and version
    fn get_state_value_with_proof_by_version_ext(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<(Option<StateValue>, SparseMerkleProofExt)> {
        let (leaf_data, proof) = self
            .state_merkle_db
            .get_with_proof_ext(state_key, version)?;
        Ok((
            match leaf_data {
                Some((_, (key, version))) => Some(self.expect_value_by_version(&key, version)?),
                None => None,
            },
            proof,
        ))
    }

    fn get_state_storage_usage(&self, version: Option<Version>) -> Result<StateStorageUsage> {
        version.map_or(Ok(StateStorageUsage::zero()), |version| {
            Ok(self
                .ledger_db
                .get::<VersionDataSchema>(&version)?
                .ok_or_else(|| AptosDbError::NotFound(format!("VersionData at {}", version)))?
                .get_state_storage_usage())
        })
    }
}

impl StateDb {
    /// Get the latest state value and the its corresponding version when its of the given key up
    /// to the given version.
    pub fn get_state_value_with_version_by_version(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<Option<(Version, StateValue)>> {
        let mut read_opts = ReadOptions::default();
        // We want `None` if the state_key changes in iteration.
        read_opts.set_prefix_same_as_start(true);
        let mut iter = self.ledger_db.iter::<StateValueSchema>(read_opts)?;
        iter.seek(&(state_key.clone(), version))?;
        Ok(iter
            .next()
            .transpose()?
            .and_then(|((_, version), value_opt)| value_opt.map(|value| (version, value))))
    }

    /// Get the latest ended epoch strictly before required version, i.e. if the passed in version
    /// ends an epoch, return one epoch early than that.
    pub fn get_previous_epoch_ending(&self, version: Version) -> Result<Option<(u64, Version)>> {
        if version == 0 {
            return Ok(None);
        }
        let prev_version = version - 1;

        let mut iter = self
            .ledger_db
            .iter::<EpochByVersionSchema>(ReadOptions::default())?;
        // Search for the end of the previous epoch.
        iter.seek_for_prev(&prev_version)?;
        iter.next().transpose()
    }
}

impl DbReader for StateStore {
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

    /// Returns the proof of the given state key and version.
    fn get_state_proof_by_version_ext(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<SparseMerkleProofExt> {
        self.deref()
            .get_state_proof_by_version_ext(state_key, version)
    }

    /// Get the state value with proof extension given the state key and version
    fn get_state_value_with_proof_by_version_ext(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<(Option<StateValue>, SparseMerkleProofExt)> {
        self.deref()
            .get_state_value_with_proof_by_version_ext(state_key, version)
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
                    format_err!(
                        "State Value is missing for key {:?} by version {}",
                        state_key,
                        version
                    )
                })
            })
    }
}

impl StateStore {
    pub fn new(
        ledger_db: Arc<DB>,
        state_merkle_db: Arc<DB>,
        state_pruner: StatePrunerManager<StaleNodeIndexSchema>,
        epoch_snapshot_pruner: StatePrunerManager<StaleNodeIndexCrossEpochSchema>,
        target_snapshot_size: usize,
        max_nodes_per_lru_cache_shard: usize,
        hack_for_tests: bool,
    ) -> Self {
        let state_merkle_db = Arc::new(StateMerkleDb::new(
            state_merkle_db,
            max_nodes_per_lru_cache_shard,
        ));
        let state_db = Arc::new(StateDb {
            ledger_db,
            state_merkle_db,
            state_pruner,
            epoch_snapshot_pruner,
        });
        let buffered_state = Mutex::new(
            Self::create_buffered_state_from_latest_snapshot(
                &state_db,
                target_snapshot_size,
                hack_for_tests,
            )
            .expect("buffered state creation failed."),
        );
        Self {
            state_db,
            buffered_state,
            target_snapshot_size,
        }
    }

    fn create_buffered_state_from_latest_snapshot(
        state_db: &Arc<StateDb>,
        target_snapshot_size: usize,
        hack_for_tests: bool,
    ) -> Result<BufferedState> {
        let ledger_store = LedgerStore::new(Arc::clone(&state_db.ledger_db));
        let num_transactions = ledger_store
            .get_latest_transaction_info_option()?
            .map(|(version, _)| version + 1)
            .unwrap_or(0);

        let latest_snapshot_version = state_db
            .state_merkle_db
            .get_state_snapshot_version_before(num_transactions)
            .expect("Failed to query latest node on initialization.");
        let latest_snapshot_root_hash = if let Some(version) = latest_snapshot_version {
            state_db
                .state_merkle_db
                .get_root_hash(version)
                .expect("Failed to query latest checkpoint root hash on initialization.")
        } else {
            *SPARSE_MERKLE_PLACEHOLDER_HASH
        };
        let usage = state_db.get_state_storage_usage(latest_snapshot_version)?;
        let mut buffered_state = BufferedState::new(
            state_db,
            StateDelta::new_at_checkpoint(
                latest_snapshot_root_hash,
                usage,
                latest_snapshot_version,
            ),
            target_snapshot_size,
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
            ensure!(
                num_transactions - snapshot_next_version <= MAX_WRITE_SETS_AFTER_SNAPSHOT,
                "Too many versions after state snapshot. snapshot_next_version: {}, num_transactions: {}",
                snapshot_next_version,
                num_transactions,
            );
            let latest_snapshot_state_view = CachedStateView::new(
                StateViewId::Miscellaneous,
                state_db.clone(),
                num_transactions,
                buffered_state.current_state().current.clone(),
                Arc::new(SyncProofFetcher::new(state_db.clone())),
            )?;
            let write_sets = TransactionStore::new(Arc::clone(&state_db.ledger_db))
                .get_write_sets(snapshot_next_version, num_transactions)?;
            let txn_info_iter =
                ledger_store.get_transaction_info_iter(snapshot_next_version, write_sets.len())?;
            let last_checkpoint_index = txn_info_iter
                .into_iter()
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .enumerate()
                .filter(|(_idx, txn_info)| txn_info.is_state_checkpoint())
                .last()
                .map(|(idx, _)| idx);
            latest_snapshot_state_view.prime_cache_by_write_set(&write_sets)?;
            let calculator = InMemoryStateCalculator::new(
                buffered_state.current_state(),
                latest_snapshot_state_view.into_state_cache(),
            );
            let (updates_until_last_checkpoint, state_after_last_checkpoint) = calculator
                .calculate_for_write_sets_after_snapshot(last_checkpoint_index, &write_sets)?;

            // synchronously commit the snapshot at the last checkpoint here if not committed to disk yet.
            buffered_state.update(
                updates_until_last_checkpoint,
                state_after_last_checkpoint,
                true, /* sync_commit */
            )?;
        }

        info!(
            latest_snapshot_version = buffered_state.current_state().base_version,
            latest_snapshot_root_hash = buffered_state.current_state().base.root_hash(),
            latest_in_memory_version = buffered_state.current_state().current_version,
            latest_in_memory_root_hash = buffered_state.current_state().current.root_hash(),
            "StateStore initialization finished.",
        );
        Ok(buffered_state)
    }

    pub fn reset(&self) {
        *self.buffered_state.lock() = Self::create_buffered_state_from_latest_snapshot(
            &self.state_db,
            self.target_snapshot_size,
            false,
        )
        .expect("buffered state creation failed.");
    }

    pub fn buffered_state(&self) -> &Mutex<BufferedState> {
        &self.buffered_state
    }

    /// Returns the key, value pairs for a particular state key prefix at at desired version. This
    /// API can be used to get all resources of an account by passing the account address as the
    /// key prefix.
    pub fn get_values_by_key_prefix(
        &self,
        key_prefix: &StateKeyPrefix,
        desired_version: Version,
    ) -> Result<HashMap<StateKey, StateValue>> {
        let mut read_opts = ReadOptions::default();
        // Without this, iterators are not guaranteed a total order of all keys, but only keys for the same prefix.
        // For example,
        // aptos/abc|0
        // aptos/abc|1
        // aptos/abd|1
        // if we seek('aptos/'), and call next, we may not reach `aptos/abd/1` because the prefix extractor we adopted
        // here will stick with prefix `aptos/abc` and return `None` or any arbitrary result after visited all the
        // keys starting with `aptos/abc`.
        read_opts.set_total_order_seek(true);
        let mut iter = self.ledger_db.iter::<StateValueSchema>(read_opts)?;
        let mut result = HashMap::new();
        let mut prev_key = None;
        iter.seek(&(key_prefix))?;
        while let Some(((state_key, version), state_value_opt)) = iter.next().transpose()? {
            // In case the previous seek() ends on the same key with version 0.
            if Some(&state_key) == prev_key.as_ref() {
                continue;
            }
            // Cursor is currently at the first available version of the state key.
            // Check if the key_prefix is a valid prefix of the state_key we got from DB.
            if !key_prefix.is_prefix(&state_key)? {
                // No more keys matching the key_prefix, we can return the result.
                break;
            }

            if version > desired_version {
                iter.seek(&(state_key.clone(), desired_version))?;
                continue;
            }

            if let Some(state_value) = state_value_opt {
                result.insert(state_key.clone(), state_value);
            }
            // We don't allow fetching arbitrarily large number of values to be fetched as this can
            // potentially slowdown the DB.
            if result.len() > MAX_VALUES_TO_FETCH_FOR_KEY_PREFIX {
                return Err(anyhow!(
                    "Too many values requested for key_prefix {:?} - maximum allowed {:?}",
                    key_prefix,
                    MAX_VALUES_TO_FETCH_FOR_KEY_PREFIX
                ));
            }
            prev_key = Some(state_key.clone());
            // Seek to the next key - this can be done by seeking to the current key with version 0
            iter.seek(&(state_key, 0))?;
        }
        Ok(result)
    }

    /// Gets the proof that proves a range of accounts.
    pub fn get_value_range_proof(
        &self,
        rightmost_key: HashValue,
        version: Version,
    ) -> Result<SparseMerkleRangeProof> {
        self.state_merkle_db.get_range_proof(rightmost_key, version)
    }

    /// Put the `value_state_sets` into its own CF.
    pub fn put_value_sets(
        &self,
        value_state_sets: Vec<&HashMap<StateKey, Option<StateValue>>>,
        first_version: Version,
        expected_usage: StateStorageUsage,
        batch: &mut SchemaBatch,
    ) -> Result<()> {
        self.put_stats_and_indices(&value_state_sets, first_version, expected_usage, batch)?;

        let kv_batch = value_state_sets
            .iter()
            .enumerate()
            .flat_map(|(i, kvs)| {
                kvs.iter()
                    .map(move |(k, v)| ((k.clone(), first_version + i as Version), v.clone()))
            })
            .collect::<HashMap<_, _>>();
        add_kv_batch(batch, &kv_batch)
    }

    pub fn get_usage(&self, version: Option<Version>) -> Result<StateStorageUsage> {
        let _timer = OTHER_TIMERS_SECONDS
            .with_label_values(&["get_usage"])
            .start_timer();
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
        value_state_sets: &[&HashMap<StateKey, Option<StateValue>>],
        first_version: Version,
        expected_usage: StateStorageUsage,
        batch: &mut SchemaBatch,
    ) -> Result<()> {
        let _timer = OTHER_TIMERS_SECONDS
            .with_label_values(&["put_stats_and_indices"])
            .start_timer();

        let base_version = first_version.checked_sub(1);
        let mut usage = self.get_usage(base_version)?;
        let cache = Arc::new(Mutex::new(
            HashMap::<StateKey, (Version, Option<StateValue>)>::new(),
        ));

        if let Some(base_version) = base_version {
            let key_set = value_state_sets
                .iter()
                .flat_map(|value_state_set| value_state_set.iter())
                .map(|(key, _)| key)
                .collect::<HashSet<_>>();
            IO_POOL.scope(|s| {
                for key in key_set {
                    let cache = cache.clone();
                    s.spawn(move |_| {
                        let _timer = OTHER_TIMERS_SECONDS
                            .with_label_values(&["put_stats_and_indices__get_state_value"])
                            .start_timer();
                        let version_and_value = self
                            .state_db
                            .get_state_value_with_version_by_version(key, base_version)
                            .expect("Must succeed.");
                        if let Some((version, value)) = version_and_value {
                            cache.lock().insert(key.clone(), (version, Some(value)));
                        } else {
                            cache.lock().insert(key.clone(), (base_version, None));
                        }
                    });
                }
            });
        }

        // calculate total state size in bytes
        for (idx, kvs) in value_state_sets.iter().enumerate() {
            let version = first_version + idx as Version;

            for (key, value) in kvs.iter() {
                if let Some(value) = value {
                    usage.add_item(key.size() + value.size());
                } else {
                    // stale index of the tombstone at current version.
                    batch.put::<StaleStateValueIndexSchema>(
                        &StaleStateValueIndex {
                            stale_since_version: version,
                            version,
                            state_key: key.clone(),
                        },
                        &(),
                    )?;
                }

                let old_version_and_value_opt = if let Some((old_version, old_value_opt)) =
                    cache.lock().insert(key.clone(), (version, value.clone()))
                {
                    old_value_opt.map(|value| (old_version, value))
                } else {
                    None
                };

                if let Some((old_version, old_value)) = old_version_and_value_opt {
                    usage.remove_item(key.size() + old_value.size());
                    // stale index of the old value at its version.
                    batch.put::<StaleStateValueIndexSchema>(
                        &StaleStateValueIndex {
                            stale_since_version: version,
                            version: old_version,
                            state_key: key.clone(),
                        },
                        &(),
                    )?;
                }
            }

            STATE_ITEMS.set(usage.items() as i64);
            TOTAL_STATE_BYTES.set(usage.bytes() as i64);
            batch.put::<VersionDataSchema>(&version, &usage.into())?;
        }

        if !expected_usage.is_untracked() {
            ensure!(
                expected_usage == usage,
                "Calculated state db usage not expected. expected: {:?}, calculated: {:?}",
                expected_usage,
                usage,
            );
        }

        Ok(())
    }

    /// Merklize the results generated by `value_state_sets` to `batch` and return the result root
    /// hashes for each write set.
    #[cfg(test)]
    pub fn merklize_value_set(
        &self,
        value_set: Vec<(HashValue, Option<&(HashValue, StateKey)>)>,
        node_hashes: Option<&HashMap<aptos_types::nibble::nibble_path::NibblePath, HashValue>>,
        version: Version,
        base_version: Option<Version>,
    ) -> Result<HashValue> {
        let (batch, hash) = self.state_merkle_db.merklize_value_set(
            value_set,
            node_hashes,
            version,
            base_version,
            None, // previous epoch ending version
        )?;
        self.state_merkle_db.write_schemas(batch)?;
        Ok(hash)
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
        start_hashed_key: HashValue,
    ) -> Result<impl Iterator<Item = Result<(StateKey, StateValue)>> + Send + Sync> {
        let store = Arc::clone(self);
        Ok(JellyfishMerkleIterator::new(
            Arc::clone(&self.state_merkle_db),
            version,
            start_hashed_key,
        )?
        .map(move |res| match res {
            Ok((_hashed_key, (key, version))) => {
                Ok((key.clone(), store.expect_value_by_version(&key, version)?))
            }
            Err(err) => Err(err),
        }))
    }

    pub fn get_value_chunk_with_proof(
        self: &Arc<Self>,
        version: Version,
        first_index: usize,
        chunk_size: usize,
    ) -> Result<StateValueChunkWithProof> {
        let result_iter = JellyfishMerkleIterator::new_by_index(
            Arc::clone(&self.state_merkle_db),
            version,
            first_index,
        )?
        .take(chunk_size);
        let state_key_values: Vec<(StateKey, StateValue)> = result_iter
            .into_iter()
            .map(|res| {
                res.and_then(|(_, (key, version))| {
                    Ok((key.clone(), self.expect_value_by_version(&key, version)?))
                })
            })
            .collect::<Result<Vec<_>>>()?;
        ensure!(
            !state_key_values.is_empty(),
            AptosDbError::NotFound(format!("State chunk starting at {}", first_index)),
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
        )?))
    }

    /// Prune the stale state value schema generated between a range of version in (begin, end]
    pub fn prune_state_values(
        &self,
        begin: Version,
        end: Version,
        db_batch: &mut SchemaBatch,
    ) -> Result<()> {
        let mut iter = self
            .state_db
            .ledger_db
            .iter::<StaleStateValueIndexSchema>(ReadOptions::default())?;
        iter.seek(&begin)?;
        while let Some(item) = iter.next() {
            let (index, _) = item?;
            if index.stale_since_version > end {
                break;
            }
            // Prune the stale state value index itself first.
            db_batch.delete::<StaleStateValueIndexSchema>(&index)?;
            db_batch.delete::<StateValueSchema>(&(index.state_key, index.version))?;
        }
        for version in begin..end {
            db_batch.delete::<VersionDataSchema>(&version)?;
        }
        Ok(())
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
            .db
            .iter::<crate::jellyfish_merkle_node::JellyfishMerkleNodeSchema>(
            Default::default(),
        )?;
        iter.seek_to_first();
        let all_rows = iter.collect::<Result<Vec<_>>>()?;
        Ok(all_rows.into_iter().map(|(k, _v)| k).collect())
    }
}

impl StateValueWriter<StateKey, StateValue> for StateStore {
    fn write_kv_batch(
        &self,
        version: Version,
        node_batch: &StateValueBatch,
        progress: StateSnapshotProgress,
    ) -> Result<()> {
        let _timer = OTHER_TIMERS_SECONDS
            .with_label_values(&["state_value_writer_write_chunk"])
            .start_timer();
        let mut batch = SchemaBatch::new();
        add_kv_batch(&mut batch, node_batch)?;
        batch.put::<DbMetadataSchema>(
            &DbMetadataKey::StateSnapshotRestoreProgress(version),
            &DbMetadataValue::StateSnapshotProgress(progress),
        )?;
        self.ledger_db.write_schemas(batch)
    }

    fn write_usage(&self, version: Version, usage: StateStorageUsage) -> Result<()> {
        self.ledger_db
            .put::<VersionDataSchema>(&version, &usage.into())
    }

    fn get_progress(&self, version: Version) -> Result<Option<StateSnapshotProgress>> {
        Ok(self
            .ledger_db
            .get::<DbMetadataSchema>(&DbMetadataKey::StateSnapshotRestoreProgress(version))?
            .map(|v| v.expect_state_snapshot_progress()))
    }
}

fn add_kv_batch(batch: &mut SchemaBatch, kv_batch: &StateValueBatch) -> Result<()> {
    for (k, v) in kv_batch {
        batch.put::<StateValueSchema>(k, v)?;
    }
    Ok(())
}
