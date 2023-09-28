// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::ProofReader;
use anyhow::{bail, Result};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_scratchpad::{FrozenSparseMerkleTree, SparseMerkleTree};
use aptos_storage_interface::{cached_state_view::StateCache, state_delta::StateDelta};
use aptos_types::{
    event::EventKey,
    on_chain_config,
    state_store::{
        create_empty_sharded_state_updates, state_key::StateKey,
        state_storage_usage::StateStorageUsage, state_value::StateValue, ShardedStateUpdates,
    },
    transaction::{Transaction, Version},
    write_set::{TransactionWrite, WriteOp, WriteSet},
};
use dashmap::DashMap;
use itertools::zip_eq;
use once_cell::sync::Lazy;
use std::collections::HashMap;

pub static NEW_EPOCH_EVENT_KEY: Lazy<EventKey> = Lazy::new(on_chain_config::new_epoch_event_key);

/// Helper class for calculating `InMemState` after a chunk or block of transactions are executed.
///
/// A new SMT is spawned in two situations:
///   1. a state checkpoint is encountered.
///   2. a transaction chunk or block ended (where `finish()` is called)
///
/// | ------------------------------------------ | -------------------------- |
/// |  (updates_between_checkpoint_and_latest)   |  (updates_after_latest)    |
/// \                                            \                            |
///  checkpoint SMT                               latest SMT                  |
///                                                                          /
///                                (creates checkpoint SMT on checkpoint txn)
///                                        (creates "latest SMT" on finish())
pub struct InMemoryStateCalculator {
    ///// These don't change during the calculation.
    // This makes sure all in-mem nodes seen while proofs were fetched stays in mem during the
    // calculation
    _frozen_base: FrozenSparseMerkleTree<StateValue>,
    proof_reader: ProofReader,

    //// These changes every time a new txn is added to the calculator.
    state_cache: DashMap<StateKey, Option<StateValue>>,
    next_version: Version,
    updates_after_latest: ShardedStateUpdates,
    usage: StateStorageUsage,

    //// These changes whenever make_checkpoint() or finish() happens.
    checkpoint: SparseMerkleTree<StateValue>,
    checkpoint_version: Option<Version>,
    // This doesn't need to be frozen since `_frozen_base` holds a ref to the oldest ancestor
    // already, but frozen SMT is used here anyway to avoid exposing the `batch_update()` interface
    // on the non-frozen SMT.
    latest: FrozenSparseMerkleTree<StateValue>,
    updates_between_checkpoint_and_latest: ShardedStateUpdates,
}

impl InMemoryStateCalculator {
    pub fn new(base: &StateDelta, state_cache: StateCache) -> Self {
        let StateCache {
            frozen_base,
            sharded_state_cache,
            proofs,
        } = state_cache;
        let StateDelta {
            base,
            base_version,
            current,
            current_version,
            updates_since_base,
        } = base.clone();

        // TODO(grao): Rethink the strategy for state sync, and optimize this.
        let state_cache = sharded_state_cache
            .iter()
            .flatten()
            .map(|entry| (entry.key().clone(), entry.value().1.clone()))
            .collect();

        Self {
            _frozen_base: frozen_base,
            proof_reader: ProofReader::new(proofs),

            state_cache,
            next_version: current_version.map_or(0, |v| v + 1),
            updates_after_latest: create_empty_sharded_state_updates(),
            usage: current.usage(),

            checkpoint: base,
            checkpoint_version: base_version,
            latest: current.freeze(),
            updates_between_checkpoint_and_latest: updates_since_base,
        }
    }

    fn make_checkpoint(&mut self) -> Result<HashValue> {
        // Update SMT.
        let smt_updates: Vec<_> = self
            .updates_after_latest
            .iter()
            .flatten()
            .map(|(key, value)| (key.hash(), value.as_ref()))
            .collect();
        let new_checkpoint =
            self.latest
                .batch_update(smt_updates, self.usage, &self.proof_reader)?;
        let root_hash = new_checkpoint.root_hash();

        // Move self to the new checkpoint.
        self.latest = new_checkpoint.clone();
        self.checkpoint = new_checkpoint.unfreeze();
        self.checkpoint_version = self.next_version.checked_sub(1);
        self.updates_between_checkpoint_and_latest = create_empty_sharded_state_updates();
        self.updates_after_latest = create_empty_sharded_state_updates();

        Ok(root_hash)
    }

    fn finish(mut self) -> Result<(StateDelta, HashMap<StateKey, StateValue>)> {
        let smt_updates: Vec<_> = self
            .updates_after_latest
            .iter()
            .flatten()
            .map(|(key, value)| (key.hash(), value.as_ref()))
            .collect();
        let latest = self
            .latest
            .batch_update(smt_updates, self.usage, &self.proof_reader)?;

        zip_eq(
            self.updates_between_checkpoint_and_latest.iter_mut(),
            self.updates_after_latest,
        )
        .for_each(|(base, delta)| {
            base.extend(delta);
        });

        let result_state = StateDelta::new(
            self.checkpoint,
            self.checkpoint_version,
            latest.unfreeze(),
            self.next_version.checked_sub(1),
            self.updates_between_checkpoint_and_latest,
        );

        Ok((
            result_state,
            self.state_cache
                .into_iter()
                .filter_map(|(k, v_opt)| v_opt.map(|v| (k, v)))
                .collect(),
        ))
    }

    pub fn calculate_for_write_sets_after_snapshot(
        mut self,
        last_checkpoint_index: Option<usize>,
        write_sets: &[WriteSet],
    ) -> Result<(Option<ShardedStateUpdates>, StateDelta)> {
        let idx_after_last_checkpoint = last_checkpoint_index.map_or(0, |idx| idx + 1);
        let updates_before_last_checkpoint = if idx_after_last_checkpoint != 0 {
            for write_set in write_sets[0..idx_after_last_checkpoint].iter() {
                let state_updates = process_write_set(
                    None,
                    &mut self.state_cache,
                    &mut self.usage,
                    (*write_set).clone(),
                )?;
                self.insert_to_latest_updates(state_updates);
                self.next_version += 1;
            }
            let updates = self.updates_after_latest.clone();
            self.make_checkpoint()?;
            Some(updates)
        } else {
            None
        };
        for write_set in write_sets[idx_after_last_checkpoint..].iter() {
            let state_updates = process_write_set(
                None,
                &mut self.state_cache,
                &mut self.usage,
                (*write_set).clone(),
            )?;
            self.insert_to_latest_updates(state_updates);
            self.next_version += 1;
        }
        let (result_state, _) = self.finish()?;
        Ok((updates_before_last_checkpoint, result_state))
    }

    fn insert_to_latest_updates(&mut self, state_updates: HashMap<StateKey, Option<StateValue>>) {
        state_updates.into_iter().for_each(|(k, v)| {
            self.updates_after_latest[k.get_shard_id() as usize].insert(k, v);
        });
    }
}

// Checks the write set is a subset of the read set.
// Updates the `state_cache` to reflect the latest value.
// Returns all state key-value pair touched.
pub fn process_write_set(
    transaction: Option<&Transaction>,
    state_cache: &mut DashMap<StateKey, Option<StateValue>>,
    usage: &mut StateStorageUsage,
    write_set: WriteSet,
) -> Result<HashMap<StateKey, Option<StateValue>>> {
    // Find all keys this transaction touches while processing each write op.
    write_set
        .into_iter()
        .map(|(state_key, write_op)| {
            process_state_key_write_op(transaction, state_cache, usage, state_key, write_op)
        })
        .collect::<Result<_>>()
}

fn process_state_key_write_op(
    transaction: Option<&Transaction>,
    state_cache: &mut DashMap<StateKey, Option<StateValue>>,
    usage: &mut StateStorageUsage,
    state_key: StateKey,
    write_op: WriteOp,
) -> Result<(StateKey, Option<StateValue>)> {
    let key_size = state_key.size();
    let state_value = write_op.as_state_value();
    if let Some(ref value) = state_value {
        usage.add_item(key_size + value.size())
    }
    let cached = state_cache.insert(state_key.clone(), state_value.clone());
    if let Some(old_value_opt) = cached {
        if let Some(old_value) = old_value_opt {
            usage.remove_item(key_size + old_value.size());
        }
    } else if let Some(txn) = transaction {
        ensure_txn_valid_for_vacant_entry(txn)?;
    }
    Ok((state_key, state_value))
}

fn ensure_txn_valid_for_vacant_entry(transaction: &Transaction) -> Result<()> {
    // Before writing to an account, VM should always read that account. So we
    // should not reach this code path. The exception is genesis transaction (and
    // maybe other writeset transactions).
    match transaction {
        Transaction::GenesisTransaction(_) => (),
        Transaction::BlockMetadata(_) | Transaction::UserTransaction(_) => {
            bail!("Write set should be a subset of read set.")
        },
        Transaction::StateCheckpoint(_) => {},
    }
    Ok(())
}
