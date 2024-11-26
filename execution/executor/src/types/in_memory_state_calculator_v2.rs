// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::OTHER_TIMERS;
use anyhow::{ensure, Result};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_drop_helper::DropHelper;
use aptos_executor_types::{
    execution_output::ExecutionOutput, state_checkpoint_output::StateCheckpointOutput,
    transactions_with_output::TransactionsWithOutput, ProofReader,
};
use aptos_logger::info;
use aptos_metrics_core::TimerHelper;
use aptos_scratchpad::FrozenSparseMerkleTree;
use aptos_storage_interface::state_store::{
    sharded_state_update_refs::ShardedStateUpdateRefs,
    sharded_state_updates::ShardedStateUpdates,
    state_delta::StateDelta,
    state_view::cached_state_view::{ShardedStateCache, StateCache, StateCacheShard},
};
use aptos_types::{
    state_store::{
        state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
    },
    write_set::WriteSet,
};
use itertools::{zip_eq, Itertools};
use rayon::prelude::*;
use std::{ops::Deref, sync::Arc};

/// Helper class for calculating state changes after a block of transactions are executed.
pub struct InMemoryStateCalculatorV2 {}

impl InMemoryStateCalculatorV2 {
    pub fn calculate_for_transactions(
        execution_output: &ExecutionOutput,
        parent_state: &Arc<StateDelta>,
        known_state_checkpoints: Option<impl IntoIterator<Item = Option<HashValue>>>,
    ) -> Result<StateCheckpointOutput> {
        if execution_output.is_block {
            Self::validate_input_for_block(parent_state, &execution_output.to_commit)?;
        }

        Self::calculate_impl(
            parent_state,
            &execution_output.state_cache,
            execution_output.to_commit.state_update_refs(),
            execution_output.to_commit.last_checkpoint_index(),
            execution_output.is_block,
            known_state_checkpoints,
        )
    }

    pub fn calculate_for_write_sets_after_snapshot(
        parent_state: &Arc<StateDelta>,
        state_cache: &StateCache,
        last_checkpoint_index: Option<usize>,
        write_sets: &[WriteSet],
    ) -> Result<StateCheckpointOutput> {
        let state_update_refs =
            ShardedStateUpdateRefs::index_write_sets(write_sets, write_sets.len());

        Self::calculate_impl(
            parent_state,
            state_cache,
            &state_update_refs,
            last_checkpoint_index,
            false,
            Option::<Vec<_>>::None,
        )
    }

    fn calculate_impl(
        parent_state: &Arc<StateDelta>,
        state_cache: &StateCache,
        state_update_refs: &ShardedStateUpdateRefs,
        last_checkpoint_index: Option<usize>,
        is_block: bool,
        known_state_checkpoints: Option<impl IntoIterator<Item = Option<HashValue>>>,
    ) -> Result<StateCheckpointOutput> {
        let StateCache {
            // This makes sure all in-mem nodes seen while proofs were fetched stays in mem during the
            // calculation
            frozen_base,
            sharded_state_cache,
            proofs,
        } = state_cache;
        assert!(frozen_base.smt.is_the_same(&parent_state.current));

        // TODO(aldenhu): use maps of refs instead of cloning the state kvs
        let (updates_before_last_checkpoint, updates_after_last_checkpoint) =
            Self::calculate_updates(state_update_refs, last_checkpoint_index);
        // TODO(aldenhu): calculate on the checkpoint as well, and don't need to combine
        let mut _all_updates_owned: Option<DropHelper<ShardedStateUpdates>> = None;
        let all_updates = if updates_after_last_checkpoint.all_shards_empty() {
            &updates_before_last_checkpoint
        } else if updates_before_last_checkpoint.all_shards_empty() {
            &updates_after_last_checkpoint
        } else {
            let _timer = OTHER_TIMERS.timer_with(&["calculate_all_updates"]);
            let mut all_updates = updates_before_last_checkpoint.clone();
            all_updates.clone_merge(&updates_after_last_checkpoint);
            _all_updates_owned = Some(DropHelper::new(all_updates));
            _all_updates_owned.as_ref().expect("Just set").deref()
        };

        let usage = Self::calculate_usage(
            parent_state.current.usage(),
            sharded_state_cache,
            all_updates,
        );

        let first_version = parent_state.current_version.map_or(0, |v| v + 1);
        let num_txns = state_update_refs.num_versions;
        let proof_reader = ProofReader::new(proofs);
        let latest_checkpoint = if let Some(index) = last_checkpoint_index {
            Self::make_checkpoint(
                parent_state.current.freeze(&frozen_base.base_smt),
                &updates_before_last_checkpoint,
                if index == num_txns - 1 {
                    usage
                } else {
                    StateStorageUsage::new_untracked()
                },
                &proof_reader,
            )?
        } else {
            // If there is no checkpoint in this chunk, the latest checkpoint will be the existing
            // one.
            parent_state.base.freeze(&frozen_base.base_smt)
        };

        let mut latest_checkpoint_version = parent_state.base_version;
        let mut state_checkpoint_hashes = known_state_checkpoints
            .map_or_else(|| vec![None; num_txns], |v| v.into_iter().collect());
        ensure!(
            state_checkpoint_hashes.len() == num_txns,
            "Bad number of known hashes."
        );
        if let Some(index) = last_checkpoint_index {
            if let Some(h) = state_checkpoint_hashes[index] {
                ensure!(
                    h == latest_checkpoint.root_hash(),
                    "Last checkpoint not expected."
                );
            } else {
                state_checkpoint_hashes[index] = Some(latest_checkpoint.root_hash());
            }
            latest_checkpoint_version = Some(first_version + index as u64);
        }

        let current_version = first_version + num_txns as u64 - 1;
        // We need to calculate the SMT at the end of the chunk, if it is not already calculated.
        let current_tree = if last_checkpoint_index == Some(num_txns - 1) {
            latest_checkpoint.smt.clone()
        } else {
            ensure!(!is_block, "Block must have the checkpoint at the end.");
            // The latest tree is either the last checkpoint in current chunk, or the tree at the
            // end of previous chunk if there is no checkpoint in the current chunk.
            let latest_tree = if last_checkpoint_index.is_some() {
                latest_checkpoint.clone()
            } else {
                parent_state.current.freeze(&frozen_base.base_smt)
            };
            Self::make_checkpoint(
                latest_tree,
                &updates_after_last_checkpoint,
                usage,
                &proof_reader,
            )?
            .smt
        };

        let updates_since_latest_checkpoint = if last_checkpoint_index.is_some() {
            updates_after_last_checkpoint
        } else {
            let mut updates_since_latest_checkpoint =
                parent_state.updates_since_base.deref().clone();
            zip_eq(
                updates_since_latest_checkpoint.shards.iter_mut(),
                updates_after_last_checkpoint.shards,
            )
            .for_each(|(base, delta)| base.extend(delta));
            updates_since_latest_checkpoint
        };

        info!(
            "last_checkpoint_index {last_checkpoint_index:?}, result_state: {latest_checkpoint_version:?} {:?} {:?} {current_version} {:?} {:?}",
            latest_checkpoint.root_hash(),
            latest_checkpoint.usage(),
            current_tree.root_hash(),
            current_tree.usage(),
        );

        let result_state = StateDelta::new(
            latest_checkpoint.smt,
            latest_checkpoint_version,
            current_tree,
            Some(current_version),
            updates_since_latest_checkpoint,
        );

        Ok(StateCheckpointOutput::new(
            parent_state.clone(),
            Arc::new(result_state),
            last_checkpoint_index.map(|_| updates_before_last_checkpoint),
            state_checkpoint_hashes,
        ))
    }

    fn calculate_updates(
        state_update_refs: &ShardedStateUpdateRefs,
        last_checkpoint_index: Option<usize>,
    ) -> (ShardedStateUpdates, ShardedStateUpdates) {
        let _timer = OTHER_TIMERS.timer_with(&["calculate_updates"]);

        let mut shard_iters = state_update_refs
            .shards
            .iter()
            .map(|shard| shard.iter())
            .collect::<Vec<_>>();
        let mut before_last_checkpoint = ShardedStateUpdates::new_empty();
        let mut after_last_checkpoint = ShardedStateUpdates::new_empty();

        // TODO(aldenhu): no need to par_iter() if no need to clone.
        if let Some(last_checkpoint_index) = last_checkpoint_index {
            shard_iters
                .par_iter_mut()
                .zip_eq(before_last_checkpoint.shards.par_iter_mut())
                .for_each(|(shard_iter, shard_updates)| {
                    shard_updates.extend(
                        shard_iter
                            // n.b. take_while_ref so that in the next step we can process the rest of the entries from the iters.
                            .take_while_ref(|(idx, _k, _v)| *idx <= last_checkpoint_index)
                            .map(|(_idx, k, v)| ((*k).clone(), v.cloned())),
                    )
                });
        }

        let num_txns = state_update_refs.num_versions;
        if num_txns != 0 && last_checkpoint_index != Some(num_txns - 1) {
            shard_iters
                .par_iter_mut()
                .zip_eq(after_last_checkpoint.shards.par_iter_mut())
                .for_each(|(shard_iter, shard_updates)| {
                    shard_updates.extend(shard_iter.map(|(_idx, k, v)| ((*k).clone(), v.cloned())))
                });
        }

        (before_last_checkpoint, after_last_checkpoint)
    }

    fn add_to_delta(
        _k: &StateKey,
        _v: &Option<&StateValue>,
        _state_cache: &StateCacheShard,
        _items_delta: &mut i64,
        _bytes_delta: &mut i64,
    ) {
        todo!()
        /* FIXME(aldenhu)
        let key_size = k.size();
        if let Some(value) = v {
            *items_delta += 1;
            *bytes_delta += (key_size + value.size()) as i64;
        }

        // n.b. all updated state items must be read and recorded in the state cache,
        // otherwise we can't calculate the correct usage.
        let old_entry = state_cache.get(k).expect("Must cache read");
        if let (_, Some(old_v)) = old_entry.value() {
            *items_delta -= 1;
            *bytes_delta -= (key_size + old_v.size()) as i64;
        }
         */
    }

    fn calculate_usage(
        old_usage: StateStorageUsage,
        sharded_state_cache: &ShardedStateCache,
        updates: &ShardedStateUpdates,
    ) -> StateStorageUsage {
        let _timer = OTHER_TIMERS.timer_with(&["calculate_usage"]);

        if old_usage.is_untracked() {
            return StateStorageUsage::new_untracked();
        }

        let (items_delta, bytes_delta) = sharded_state_cache
            .shards
            .par_iter()
            .zip_eq(updates.shards.par_iter())
            .map(|(cache, updates)| {
                let mut items_delta = 0;
                let mut bytes_delta = 0;
                updates.iter().for_each(|(key, value)| {
                    Self::add_to_delta(
                        key,
                        &value.as_ref(),
                        cache,
                        &mut items_delta,
                        &mut bytes_delta,
                    )
                });
                (items_delta, bytes_delta)
            })
            .reduce(|| (0, 0), |(i1, b1), (i2, b2)| (i1 + i2, b1 + b2));

        StateStorageUsage::new(
            (old_usage.items() as i64 + items_delta) as usize,
            (old_usage.bytes() as i64 + bytes_delta) as usize,
        )
    }

    fn make_checkpoint(
        latest_checkpoint: FrozenSparseMerkleTree<StateValue>,
        updates: &ShardedStateUpdates,
        usage: StateStorageUsage,
        proof_reader: &ProofReader,
    ) -> Result<FrozenSparseMerkleTree<StateValue>> {
        let _timer = OTHER_TIMERS.timer_with(&["make_checkpoint"]);

        // Update SMT.
        //
        // TODO(aldenhu): avoid collecting into vec
        let smt_updates: Vec<_> = {
            let _timer = OTHER_TIMERS.timer_with(&["make_smt_updates"]);
            updates
                .shards
                .iter()
                .flatten()
                .map(|(key, value)| (key.hash(), value.as_ref()))
                .collect()
        };
        let new_checkpoint = {
            let _timer = OTHER_TIMERS.timer_with(&["smt_batch_update"]);
            latest_checkpoint.batch_update(smt_updates, usage, proof_reader)?
        };
        Ok(new_checkpoint)
    }

    fn validate_input_for_block(
        base: &StateDelta,
        to_commit: &TransactionsWithOutput,
    ) -> Result<()> {
        let num_txns = to_commit.len();
        ensure!(num_txns != 0, "Empty block is not allowed.");
        ensure!(
            base.base_version == base.current_version,
            "Block base state is not a checkpoint. base_version {:?}, current_version {:?}",
            base.base_version,
            base.current_version,
        );
        ensure!(
            base.updates_since_base.all_shards_empty(),
            "Base state is corrupted, updates_since_base is not empty at a checkpoint."
        );

        Ok(())
    }
}
