// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::APTOS_EXECUTOR_OTHER_TIMERS_SECONDS;
use anyhow::{anyhow, ensure, Result};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_executor_types::{ParsedTransactionOutput, ProofReader};
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
    transaction::Transaction,
    write_set::TransactionWrite,
};
use arr_macro::arr;
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
    fn get_state_value(&self, state_key: &StateKey) -> Result<Option<Vec<u8>>> {
        if let Some(v_opt) = self.updates[state_key.get_shard_id() as usize].get(state_key) {
            return Ok(v_opt.as_ref().map(|x| x.bytes().to_vec()));
        }
        if let Some(entry) = self.base[state_key.get_shard_id() as usize]
            .get(state_key)
            .as_ref()
        {
            let state_value = entry.value().1.as_ref();
            return Ok(state_value.map(|x| x.bytes().to_vec()));
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
    pub fn calculate_for_transaction_block(
        base: &StateDelta,
        state_cache: StateCache,
        to_keep: &[(Transaction, ParsedTransactionOutput)],
        new_epoch: bool,
    ) -> Result<(
        Vec<ShardedStateUpdates>,
        Vec<Option<HashValue>>,
        StateDelta,
        Option<EpochState>,
        ShardedStateUpdates,
        ShardedStateCache,
    )> {
        ensure!(!to_keep.is_empty(), "Empty block is not allowed.");
        ensure!(
            base.base_version == base.current_version,
            "Base version {:?} is different from current_version {:?}, cannot calculate state.",
            base.base_version,
            base.current_version,
        );
        base.updates_since_base.iter().try_for_each(|shard| {
            ensure!(
                shard.is_empty(),
                "Updates is not empty, cannot calculate state."
            );
            Ok(())
        })?;

        let num_txns = to_keep.len();
        for (i, (txn, txn_output)) in to_keep.iter().enumerate() {
            ensure!(
                Self::need_checkpoint(txn, txn_output) ^ (i != num_txns - 1),
                "Checkpoint is allowed iff it's the last txn in the block. index: {i}, is_last: {}, txn: {txn:?}, is_reconfig: {}",
                i == num_txns - 1,
                txn_output.is_reconfig()
            );
        }

        let StateCache {
            // This makes sure all in-mem nodes seen while proofs were fetched stays in mem during the
            // calculation
            frozen_base: _,
            sharded_state_cache,
            proofs,
        } = state_cache;

        let state_updates_vec = Self::get_sharded_state_updates(to_keep);
        let updates: ShardedStateUpdates = Self::calculate_block_state_updates(&state_updates_vec);
        let latest_checkpoint = base.current.clone();
        let usage =
            Self::calculate_usage(latest_checkpoint.usage(), &sharded_state_cache, &updates);

        let next_epoch_state = if new_epoch {
            Some(Self::get_epoch_state(&sharded_state_cache, &updates)?)
        } else {
            None
        };

        let (new_checkpoint, new_checkpoint_version) = {
            let _timer = APTOS_EXECUTOR_OTHER_TIMERS_SECONDS
                .with_label_values(&["make_checkpoint"])
                .start_timer();
            let latest_checkpoint_version = base.current_version;
            let new_checkpoint_version =
                Some(latest_checkpoint_version.map_or(0, |v| v + 1) + num_txns as u64 - 1);
            let new_checkpoint = Self::make_checkpoint(
                latest_checkpoint,
                &updates,
                usage,
                ProofReader::new(proofs),
            )?;
            (new_checkpoint, new_checkpoint_version)
        };

        let state_checkpoint_hashes = std::iter::repeat(None)
            .take(num_txns - 1)
            .chain([Some(new_checkpoint.root_hash())])
            .collect();

        let result_state = StateDelta::new(
            new_checkpoint.clone(),
            new_checkpoint_version,
            new_checkpoint,
            new_checkpoint_version,
            create_empty_sharded_state_updates(),
        );

        Ok((
            state_updates_vec,
            state_checkpoint_hashes,
            result_state,
            next_epoch_state,
            updates,
            sharded_state_cache,
        ))
    }

    fn need_checkpoint(txn: &Transaction, txn_output: &ParsedTransactionOutput) -> bool {
        if txn_output.is_reconfig() {
            return true;
        }
        match txn {
            Transaction::BlockMetadata(_) | Transaction::UserTransaction(_) => false,
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

    fn calculate_block_state_updates(
        state_updates_vec: &[ShardedStateUpdates],
    ) -> ShardedStateUpdates {
        let _timer = APTOS_EXECUTOR_OTHER_TIMERS_SECONDS
            .with_label_values(&["calculate_block_state_updates"])
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

    fn calculate_usage(
        old_usage: StateStorageUsage,
        sharded_state_cache: &ShardedStateCache,
        updates: &ShardedStateUpdates,
    ) -> StateStorageUsage {
        let _timer = APTOS_EXECUTOR_OTHER_TIMERS_SECONDS
            .with_label_values(&["calculate_usage"])
            .start_timer();
        if old_usage.is_untracked() {
            return StateStorageUsage::new_untracked();
        }
        let (items_delta, bytes_delta) = updates
            .par_iter()
            .enumerate()
            .map(|(i, shard_updates)| {
                let mut items_delta = 0i64;
                let mut bytes_delta = 0i64;
                for (k, v) in shard_updates {
                    let key_size = k.size();
                    if let Some(ref value) = v {
                        items_delta += 1;
                        bytes_delta += (key_size + value.size()) as i64;
                    }
                    if let Some(old_entry) = sharded_state_cache[i].get(k) {
                        if let (_, Some(old_v)) = old_entry.value() {
                            items_delta -= 1;
                            bytes_delta -= (key_size + old_v.size()) as i64;
                        }
                    }
                }
                (items_delta, bytes_delta)
            })
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
        proof_reader: ProofReader,
    ) -> Result<SparseMerkleTree<StateValue>> {
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
                .batch_update(smt_updates, usage, &proof_reader)?;
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
}
