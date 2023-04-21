// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, ensure, Result};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_executor_types::{ParsedTransactionOutput, ProofReader};
use aptos_scratchpad::SparseMerkleTree;
use aptos_storage_interface::{cached_state_view::StateCache, state_delta::StateDelta};
use aptos_types::{
    account_address::AccountAddress,
    account_config::CORE_CODE_ADDRESS,
    account_view::AccountView,
    epoch_state::EpochState,
    state_store::{
        state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
    },
    transaction::Transaction,
    write_set::TransactionWrite,
};
use arr_macro::arr;
use dashmap::DashMap;
use rayon::prelude::*;
use std::collections::HashMap;

type ShardedStates = [DashMap<StateKey, Option<StateValue>>; 16];

struct CoreAccountStateView<'a> {
    base: &'a ShardedStates,
    updates: &'a HashMap<&'a StateKey, &'a Option<StateValue>>,
}

impl<'a> CoreAccountStateView<'a> {
    pub fn new(
        base: &'a ShardedStates,
        updates: &'a HashMap<&'a StateKey, &'a Option<StateValue>>,
    ) -> Self {
        Self { base, updates }
    }
}

impl<'a> AccountView for CoreAccountStateView<'a> {
    fn get_state_value(&self, state_key: &StateKey) -> Result<Option<Vec<u8>>> {
        if let Some(v_opt) = self.updates.get(state_key) {
            return Ok(v_opt.as_ref().map(|x| x.bytes().to_vec()));
        }
        if let Some(v_opt) = self.base[state_key.get_shard_id() as usize]
            .get(state_key)
            .as_ref()
        {
            return Ok(v_opt.as_ref().map(|x| x.bytes().to_vec()));
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
        Vec<HashMap<StateKey, Option<StateValue>>>,
        Vec<Option<HashValue>>,
        StateDelta,
        Option<EpochState>,
    )> {
        ensure!(!to_keep.is_empty(), "Empty block is not allowed.");
        ensure!(
            base.base_version == base.current_version,
            "Base version {:?} is different from current_version {:?}, cannot calculate state.",
            base.base_version,
            base.current_version,
        );
        ensure!(
            base.updates_since_base.is_empty(),
            "Updates is not empty, cannot calculate state."
        );

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
            state_cache,
            proofs,
        } = state_cache;

        let sharded_state_cache = arr![DashMap::new(); 16];
        // TODO(grao): Move this logic to an earlier stage.
        let _: Vec<_> = state_cache
            .into_iter()
            .map(|(k, v)| {
                sharded_state_cache[k.get_shard_id() as usize].insert(k, v);
            })
            .collect();

        let state_updates_vec = Self::get_state_updates(to_keep);

        // TODO(grao): Shard this HashMap.
        let updates: HashMap<&StateKey, &Option<StateValue>> =
            state_updates_vec.iter().flatten().collect();

        let latest_checkpoint = base.current.clone();
        let latest_checkpoint_version = base.current_version;
        let mut usage = latest_checkpoint.usage();
        for (k, v) in &updates {
            let key_size = k.size();
            if let Some(ref value) = v {
                usage.add_item(key_size + value.size())
            }
            if let Some(old_v_opt) = sharded_state_cache[k.get_shard_id() as usize].get(k) {
                if let Some(old_v) = old_v_opt.as_ref() {
                    usage.remove_item(key_size + old_v.size());
                }
            }
        }

        let next_epoch_state = if new_epoch {
            Some(Self::get_epoch_state(&sharded_state_cache, &updates)?)
        } else {
            None
        };

        let new_checkpoint_version =
            Some(latest_checkpoint_version.map_or(0, |v| v + 1) + num_txns as u64 - 1);
        let new_checkpoint =
            Self::make_checkpoint(latest_checkpoint, updates, usage, ProofReader::new(proofs))?;
        let state_checkpoint_hashes = std::iter::repeat(None)
            .take(num_txns - 1)
            .chain([Some(new_checkpoint.root_hash())])
            .collect();

        let result_state = StateDelta::new(
            new_checkpoint.clone(),
            new_checkpoint_version,
            new_checkpoint,
            new_checkpoint_version,
            HashMap::new(),
        );

        Ok((
            state_updates_vec,
            state_checkpoint_hashes,
            result_state,
            next_epoch_state,
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

    // TODO(grao): Produce sharded output.
    fn get_state_updates(
        to_keep: &[(Transaction, ParsedTransactionOutput)],
    ) -> Vec<HashMap<StateKey, Option<StateValue>>> {
        to_keep
            .par_iter()
            .map(|(_, txn_output)| {
                txn_output
                    .write_set()
                    .iter()
                    .map(|(state_key, write_op)| (state_key.clone(), write_op.as_state_value()))
                    .collect()
            })
            .collect()
    }

    fn make_checkpoint(
        latest_checkpoint: SparseMerkleTree<StateValue>,
        updates: HashMap<&StateKey, &Option<StateValue>>,
        usage: StateStorageUsage,
        proof_reader: ProofReader,
    ) -> Result<SparseMerkleTree<StateValue>> {
        // Update SMT.
        let smt_updates: Vec<_> = updates
            .iter()
            .map(|(key, value)| (key.hash(), value.as_ref()))
            .collect();
        let new_checkpoint =
            latest_checkpoint
                .freeze()
                .batch_update(smt_updates, usage, &proof_reader)?;
        Ok(new_checkpoint.unfreeze())
    }

    fn get_epoch_state(
        base: &[DashMap<StateKey, Option<StateValue>>; 16],
        updates: &HashMap<&StateKey, &Option<StateValue>>,
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
