// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This file defines state store APIs that are related account state Merkle tree.

use anyhow::{anyhow, ensure, format_err, Result};
use aptos_crypto::{
    hash::{CryptoHash, SPARSE_MERKLE_PLACEHOLDER_HASH},
    HashValue,
};
use aptos_infallible::Mutex;
use aptos_jellyfish_merkle::{
    iterator::JellyfishMerkleIterator, restore::StateSnapshotRestore, StateValueWriter,
};
use aptos_logger::{debug, info};
use aptos_state_view::StateViewId;
#[cfg(test)]
use aptos_types::nibble::nibble_path::NibblePath;
use aptos_types::proof::SparseMerkleProofExt;
use aptos_types::{
    proof::{definition::LeafCount, SparseMerkleProof, SparseMerkleRangeProof},
    state_store::{
        state_key::StateKey,
        state_key_prefix::StateKeyPrefix,
        state_value::{StateValue, StateValueChunkWithProof},
    },
    transaction::Version,
};
use executor_types::in_memory_state_calculator::InMemoryStateCalculator;
use schemadb::{ReadOptions, SchemaBatch, DB};
use std::ops::Deref;
use std::{collections::HashMap, sync::Arc};
use storage_interface::{
    cached_state_view::CachedStateView, state_delta::StateDelta,
    sync_proof_fetcher::SyncProofFetcher, DbReader, StateSnapshotReceiver,
};

use crate::state_store::buffered_state::BufferedState;
use crate::{
    change_set::ChangeSet, schema::state_value::StateValueSchema, state_merkle_db::StateMerkleDb,
    AptosDbError, LedgerStore, TransactionStore,
};

pub(crate) mod buffered_state;
mod state_merkle_batch_committer;
mod state_snapshot_committer;
#[cfg(test)]
mod state_store_test;

type StateValueBatch = aptos_jellyfish_merkle::StateValueBatch<StateKey, StateValue>;

pub const MAX_VALUES_TO_FETCH_FOR_KEY_PREFIX: usize = 10_000;
// We assume TARGET_SNAPSHOT_INTERVAL_IN_VERSION > block size.
const MAX_WRITE_SETS_AFTER_SNAPSHOT: LeafCount = buffered_state::TARGET_SNAPSHOT_INTERVAL_IN_VERSION
    * (buffered_state::ASYNC_COMMIT_CHANNEL_BUFFER_SIZE + 2 + 1/*  Rendezvous channel */)
    * 2;

#[derive(Debug)]
pub(crate) struct StateDb {
    pub ledger_db: Arc<DB>,
    pub state_merkle_db: Arc<StateMerkleDb>,
}

#[derive(Debug)]
pub(crate) struct StateStore {
    state_db: Arc<StateDb>,
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
// want another trait, `StateReader`, which is a subset of `DbReaer` here but Rust does not support trait
// upcasting coercion for now. Should change it to a different trait once upcasting is stablized.
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

    /// Get the lastest state value of the given key up to the given version. Only used for testing for now
    /// but should replace the `get_value_with_proof_by_version` call for VM execution if just fetch the
    /// value without proof.
    fn get_state_value_by_version(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<Option<StateValue>> {
        let mut read_opts = ReadOptions::default();
        // We want `None` if the state_key changes in iteration.
        read_opts.set_prefix_same_as_start(true);
        let mut iter = self.ledger_db.iter::<StateValueSchema>(read_opts)?;
        iter.seek(&(state_key.clone(), version))?;
        iter.next()
            .transpose()?
            .map(|(_, state_value)| Ok(state_value))
            .transpose()
    }

    /// Returns the proof of the given state key and version.
    fn get_state_proof_by_version(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<SparseMerkleProof> {
        let (_, proof) = self.state_merkle_db.get_with_proof(state_key, version)?;
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
}

impl DbReader for StateStore {
    /// Returns the latest state snapshot strictly before `next_version` if any.
    fn get_state_snapshot_before(
        &self,
        next_version: Version,
    ) -> Result<Option<(Version, HashValue)>> {
        self.deref().get_state_snapshot_before(next_version)
    }

    /// Get the lastest state value of the given key up to the given version. Only used for testing for now
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
    fn get_state_proof_by_version(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<SparseMerkleProof> {
        self.deref().get_state_proof_by_version(state_key, version)
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
        target_snapshot_size: usize,
        hack_for_tests: bool,
    ) -> Self {
        let state_merkle_db = Arc::new(StateMerkleDb::new(state_merkle_db));
        let state_db = Arc::new(StateDb {
            ledger_db,
            state_merkle_db,
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
        let mut buffered_state = BufferedState::new(
            &state_db.state_merkle_db,
            StateDelta::new_at_checkpoint(latest_snapshot_root_hash, latest_snapshot_version),
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

        debug!(
            latest_version = buffered_state.current_state().current_version,
            root_hash = buffered_state.current_state().current.root_hash(),
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
        while let Some(((state_key, version), state_value)) = iter.next().transpose()? {
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

            result.insert(state_key.clone(), state_value);
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
        value_state_sets: Vec<&HashMap<StateKey, StateValue>>,
        first_version: Version,
        cs: &mut ChangeSet,
    ) -> Result<()> {
        let kv_batch = value_state_sets
            .iter()
            .enumerate()
            .flat_map(|(i, kvs)| {
                kvs.iter()
                    .map(move |(k, v)| ((k.clone(), first_version + i as Version), v.clone()))
            })
            .collect::<HashMap<_, _>>();
        add_kv_batch(&mut cs.batch, &kv_batch)
    }

    /// Merklize the results generated by `value_state_sets` to `batch` and return the result root
    /// hashes for each write set.
    #[cfg(test)]
    pub fn merklize_value_set(
        &self,
        value_set: Vec<(HashValue, &(HashValue, StateKey))>,
        node_hashes: Option<&HashMap<NibblePath, HashValue>>,
        version: Version,
        base_version: Option<Version>,
    ) -> Result<HashValue> {
        let (batch, hash) = self.state_merkle_db.merklize_value_set(
            value_set,
            node_hashes,
            version,
            base_version,
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

    pub fn get_snapshot_receiver(
        self: &Arc<Self>,
        version: Version,
        expected_root_hash: HashValue,
    ) -> Result<Box<dyn StateSnapshotReceiver<StateKey, StateValue>>> {
        Ok(Box::new(StateSnapshotRestore::new_overwrite(
            &self.state_merkle_db,
            self,
            version,
            expected_root_hash,
        )?))
    }
}

impl StateValueWriter<StateKey, StateValue> for StateStore {
    fn write_kv_batch(&self, node_batch: &StateValueBatch) -> Result<()> {
        let mut batch = SchemaBatch::new();
        add_kv_batch(&mut batch, node_batch)?;
        self.ledger_db.write_schemas(batch)
    }
}

fn add_kv_batch(batch: &mut SchemaBatch, kv_batch: &StateValueBatch) -> Result<()> {
    kv_batch
        .iter()
        .map(|(k, v)| batch.put::<StateValueSchema>(k, v))
        .collect::<Result<Vec<_>>>()?;

    // Add kv_batch
    Ok(())
}
