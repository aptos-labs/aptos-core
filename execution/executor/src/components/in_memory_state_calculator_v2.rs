// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::APTOS_EXECUTOR_OTHER_TIMERS_SECONDS;
use anyhow::{anyhow, ensure, Result};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_executor_types::{ParsedTransactionOutput, ProofReader};
use aptos_experimental_runtimes::thread_manager::THREAD_MANAGER;
use aptos_logger::info;
use aptos_scratchpad::SparseMerkleTree;
use aptos_storage_interface::{
    cached_state_view::{ShardedStateCache, StateCache},
    state_delta::StateDelta,
};
use aptos_types::{
    account_address::AccountAddress,
    account_config::CORE_CODE_ADDRESS,
    account_view::AccountView,
    epoch_state::EpochState,
    state_store::{
        create_empty_sharded_state_updates, state_key::StateKey,
        state_storage_usage::StateStorageUsage, state_value::StateValue, ShardedStateUpdates,
    },
    transaction::{Transaction, Version},
    write_set::TransactionWrite,
};
use arr_macro::arr;
use bytes::Bytes;
use dashmap::DashMap;
use itertools::zip_eq;
use rayon::prelude::*;
use std::collections::HashMap;

struct CoreAccountStateView<'a> {
    base: &'a ShardedStateCache,
    updates: &'a ShardedStateUpdates,
}

impl<'a> CoreAccountStateView<'a> {
    pub fn new(base: &'a ShardedStateCache, updates: &'a ShardedStateUpdates) -> Self {
        Self { base, updates }
    }
}

impl<'a> AccountView for CoreAccountStateView<'a> {
    fn get_state_value(&self, state_key: &StateKey) -> Result<Option<Bytes>> {
        if let Some(v_opt) = self.updates[state_key.get_shard_id() as usize].get(state_key) {
            return Ok(v_opt.as_ref().map(StateValue::bytes).cloned());
        }
        if let Some(entry) = self.base[state_key.get_shard_id() as usize]
            .get(state_key)
            .as_ref()
        {
            let state_value = entry.value().1.as_ref();
            return Ok(state_value.map(StateValue::bytes).cloned());
        }
        Ok(None)
    }

    fn get_account_address(&self) -> Result<Option<AccountAddress>> {
        Ok(Some(CORE_CODE_ADDRESS))
    }
}

/// Helper class for calculating state changes after a block of transactions are executed.
pub struct InMemoryStateCalculatorV2 {}

impl InMemoryStateCalculatorV2 {
    pub fn calculate_for_transactions(
        base: &StateDelta,
        state_cache: StateCache,
        to_keep: &[(Transaction, ParsedTransactionOutput)],
        new_epoch: bool,
        is_block: bool,
    ) -> Result<(
        Vec<ShardedStateUpdates>,
        Vec<Option<HashValue>>,
        StateDelta,
        Option<EpochState>,
        ShardedStateUpdates,
        ShardedStateCache,
    )> {
        if is_block {
            Self::validate_input_for_block(base, to_keep)?;
        }

        let StateCache {
            // This makes sure all in-mem nodes seen while proofs were fetched stays in mem during the
            // calculation
            frozen_base,
            sharded_state_cache,
            proofs,
        } = state_cache;

        let num_txns = to_keep.len();
        // TODO(grao): Revisit if we really need to support empty chunk.
        if num_txns == 0 {
            return Ok((
                vec![],
                vec![],
                base.clone(),
                None,
                create_empty_sharded_state_updates(),
                sharded_state_cache,
            ));
        }

        let state_updates_vec = Self::get_sharded_state_updates(to_keep);

        // If there are multiple checkpoints in the chunk, we only calculate the SMT (and its root
        // hash) for the last one.
        let last_checkpoint_index = to_keep
            .iter()
            .enumerate()
            .rev()
            .find(|(_, (txn, txn_output))| Self::need_checkpoint(txn, txn_output))
            .map(|(index, _)| index);

        let (updates_before_last_checkpoint, updates_after_last_checkpoint) =
            if let Some(index) = last_checkpoint_index {
                (
                    Self::calculate_updates(&state_updates_vec[..=index]),
                    Self::calculate_updates(&state_updates_vec[index + 1..]),
                )
            } else {
                (
                    create_empty_sharded_state_updates(),
                    Self::calculate_updates(&state_updates_vec),
                )
            };

        let next_epoch_state = if new_epoch {
            // Assumes chunk doesn't cross epoch boundary here.
            ensure!(
                last_checkpoint_index == Some(num_txns - 1),
                "The last txn must be a reconfig for epoch change."
            );
            Some(Self::get_epoch_state(
                &sharded_state_cache,
                &updates_before_last_checkpoint,
            )?)
        } else {
            None
        };

        let usage = Self::calculate_usage(base.current.usage(), &sharded_state_cache, &[
            &updates_before_last_checkpoint,
            &updates_after_last_checkpoint,
        ]);

        let first_version = base.current_version.map_or(0, |v| v + 1);
        let proof_reader = ProofReader::new(proofs);
        let latest_checkpoint = if let Some(index) = last_checkpoint_index {
            Self::make_checkpoint(
                base.current.clone(),
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
            base.base.clone()
        };

        let mut latest_checkpoint_version = base.base_version;
        let mut state_checkpoint_hashes = vec![None; num_txns];
        if let Some(index) = last_checkpoint_index {
            state_checkpoint_hashes[index] = Some(latest_checkpoint.root_hash());
            latest_checkpoint_version = Some(first_version + index as u64);
        }

        let current_version = first_version + num_txns as u64 - 1;
        // We need to calculate the SMT at the end of the chunk, if it is not already calculated.
        let current_tree = if last_checkpoint_index == Some(num_txns - 1) {
            latest_checkpoint.clone()
        } else {
            ensure!(!is_block, "Block must have the checkpoint at the end.");
            // The latest tree is either the last checkpoint in current chunk, or the tree at the
            // end of previous chunk if there is no checkpoint in the current chunk.
            let latest_tree = if last_checkpoint_index.is_some() {
                latest_checkpoint.clone()
            } else {
                base.current.clone()
            };
            Self::make_checkpoint(
                latest_tree,
                &updates_after_last_checkpoint,
                usage,
                &proof_reader,
            )?
        };

        THREAD_MANAGER.get_non_exe_cpu_pool().spawn(move || {
            drop(frozen_base);
        });

        let updates_since_latest_checkpoint = if last_checkpoint_index.is_some() {
            updates_after_last_checkpoint
        } else {
            let mut updates_since_latest_checkpoint = base.updates_since_base.clone();
            zip_eq(
                updates_since_latest_checkpoint.iter_mut(),
                updates_after_last_checkpoint,
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
            latest_checkpoint.clone(),
            latest_checkpoint_version,
            current_tree,
            Some(current_version),
            updates_since_latest_checkpoint,
        );

        Ok((
            state_updates_vec,
            state_checkpoint_hashes,
            result_state,
            next_epoch_state,
            updates_before_last_checkpoint,
            sharded_state_cache,
        ))
    }

    fn need_checkpoint(txn: &Transaction, txn_output: &ParsedTransactionOutput) -> bool {
        if txn_output.is_reconfig() {
            return true;
        }
        match txn {
            Transaction::BlockMetadataTransaction(_) | Transaction::UserTransaction(_) => false,
            Transaction::GenesisTransaction(_) | Transaction::StateCheckpoint(_) => true,
        }
    }

    fn get_sharded_state_updates(
        to_keep: &[(Transaction, ParsedTransactionOutput)],
    ) -> Vec<ShardedStateUpdates> {
        let _timer = APTOS_EXECUTOR_OTHER_TIMERS_SECONDS
            .with_label_values(&["get_sharded_state_updates"])
            .start_timer();
        to_keep
            .par_iter()
            .map(|(_, txn_output)| {
                let mut updates = arr![HashMap::new(); 16];
                txn_output
                    .write_set()
                    .iter()
                    .for_each(|(state_key, write_op)| {
                        updates[state_key.get_shard_id() as usize]
                            .insert(state_key.clone(), write_op.as_state_value());
                    });
                updates
            })
            .collect()
    }

    fn calculate_updates(state_updates_vec: &[ShardedStateUpdates]) -> ShardedStateUpdates {
        let _timer = APTOS_EXECUTOR_OTHER_TIMERS_SECONDS
            .with_label_values(&["calculate_updates"])
            .start_timer();
        let mut updates: ShardedStateUpdates = create_empty_sharded_state_updates();
        updates
            .par_iter_mut()
            .enumerate()
            .for_each(|(i, per_shard_update)| {
                per_shard_update.extend(
                    state_updates_vec
                        .iter()
                        .flat_map(|hms| &hms[i])
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect::<Vec<_>>(),
                )
            });
        updates
    }

    fn add_to_delta(
        k: &StateKey,
        v: &Option<StateValue>,
        state_cache: &DashMap<StateKey, (Option<Version>, Option<StateValue>)>,
        items_delta: &mut i64,
        bytes_delta: &mut i64,
    ) {
        let key_size = k.size();
        if let Some(ref value) = v {
            *items_delta += 1;
            *bytes_delta += (key_size + value.size()) as i64;
        }
        if let Some(old_entry) = state_cache.get(k) {
            if let (_, Some(old_v)) = old_entry.value() {
                *items_delta -= 1;
                *bytes_delta -= (key_size + old_v.size()) as i64;
            }
        }
    }

    fn calculate_usage(
        old_usage: StateStorageUsage,
        sharded_state_cache: &ShardedStateCache,
        updates: &[&ShardedStateUpdates; 2],
    ) -> StateStorageUsage {
        let _timer = APTOS_EXECUTOR_OTHER_TIMERS_SECONDS
            .with_label_values(&["calculate_usage"])
            .start_timer();
        if old_usage.is_untracked() {
            return StateStorageUsage::new_untracked();
        }
        let (items_delta, bytes_delta) = updates[0]
            .par_iter()
            .zip_eq(updates[1].par_iter())
            .enumerate()
            .map(
                |(i, (shard_updates_before_checkpoint, shard_updates_after_checkpoint))| {
                    let mut items_delta = 0i64;
                    let mut bytes_delta = 0i64;
                    let num_updates_before_checkpoint = shard_updates_before_checkpoint.len();
                    for (index, (k, v)) in shard_updates_before_checkpoint
                        .iter()
                        .chain(shard_updates_after_checkpoint.iter())
                        .enumerate()
                    {
                        // Ignore updates before the checkpoint if there is an update for the same
                        // key after the checkpoint.
                        if index < num_updates_before_checkpoint
                            && shard_updates_after_checkpoint.contains_key(k)
                        {
                            continue;
                        }
                        Self::add_to_delta(
                            k,
                            v,
                            &sharded_state_cache[i],
                            &mut items_delta,
                            &mut bytes_delta,
                        );
                    }
                    (items_delta, bytes_delta)
                },
            )
            .reduce(
                || (0i64, 0i64),
                |(items_now, bytes_now), (items_delta, bytes_delta)| {
                    (items_now + items_delta, bytes_now + bytes_delta)
                },
            );
        StateStorageUsage::new(
            (old_usage.items() as i64 + items_delta) as usize,
            (old_usage.bytes() as i64 + bytes_delta) as usize,
        )
    }

    fn make_checkpoint(
        latest_checkpoint: SparseMerkleTree<StateValue>,
        updates: &ShardedStateUpdates,
        usage: StateStorageUsage,
        proof_reader: &ProofReader,
    ) -> Result<SparseMerkleTree<StateValue>> {
        let _timer = APTOS_EXECUTOR_OTHER_TIMERS_SECONDS
            .with_label_values(&["make_checkpoint"])
            .start_timer();

        // Update SMT.
        //
        // TODO(grao): Consider use the sharded updates directly instead of flatten.
        let smt_updates: Vec<_> = updates
            .iter()
            .flatten()
            .map(|(key, value)| (key.hash(), value.as_ref()))
            .collect();
        let new_checkpoint =
            latest_checkpoint
                .freeze()
                .batch_update(smt_updates, usage, proof_reader)?;
        Ok(new_checkpoint.unfreeze())
    }

    fn get_epoch_state(
        base: &ShardedStateCache,
        updates: &ShardedStateUpdates,
    ) -> Result<EpochState> {
        let core_account_view = CoreAccountStateView::new(base, updates);
        let validator_set = core_account_view
            .get_validator_set()?
            .ok_or_else(|| anyhow!("ValidatorSet not touched on epoch change"))?;
        let configuration = core_account_view
            .get_configuration_resource()?
            .ok_or_else(|| anyhow!("Configuration resource not touched on epoch change"))?;

        Ok(EpochState {
            epoch: configuration.epoch(),
            verifier: (&validator_set).into(),
        })
    }

    fn validate_input_for_block(
        base: &StateDelta,
        to_keep: &[(Transaction, ParsedTransactionOutput)],
    ) -> Result<()> {
        let num_txns = to_keep.len();
        ensure!(num_txns != 0, "Empty block is not allowed.");
        ensure!(
            base.base_version == base.current_version,
            "Base version {:?} is different from current_version {:?}, cannot calculate state.",
            base.base_version,
            base.current_version,
        );

        base.updates_since_base.iter().try_for_each(|shard| {
            ensure!(
                shard.is_empty(),
                "Updates is not empty, cannot calculate state for block."
            );
            Ok(())
        })?;

        for (i, (txn, txn_output)) in to_keep.iter().enumerate() {
            ensure!(
                Self::need_checkpoint(txn, txn_output) ^ (i != num_txns - 1),
                "Checkpoint is allowed iff it's the last txn in the block. index: {i}, is_last: {}, txn: {txn:?}, is_reconfig: {}",
                i == num_txns - 1,
                txn_output.is_reconfig()
            );
        }
        Ok(())
    }
}
