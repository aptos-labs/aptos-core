// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::collections::{hash_map, HashMap};

use anyhow::{anyhow, bail, Result};
use once_cell::sync::Lazy;

use crate::{ParsedTransactionOutput, ProofReader};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_state_view::account_with_state_cache::AsAccountWithStateCache;
use aptos_types::{
    account_config::CORE_CODE_ADDRESS,
    account_view::AccountView,
    epoch_state::EpochState,
    event::EventKey,
    on_chain_config,
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::{Transaction, TransactionPayload, Version},
    write_set::{WriteOp, WriteSet},
};
use scratchpad::{FrozenSparseMerkleTree, SparseMerkleTree};
use storage_interface::{cached_state_view::StateCache, state_delta::StateDelta};

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
    // This makes sure all in-mem nodes seen while proofs were fetched stays in mem during the
    // calculation
    _frozen_base: FrozenSparseMerkleTree<StateValue>,
    state_cache: HashMap<StateKey, Option<StateValue>>,
    proof_reader: ProofReader,

    checkpoint: SparseMerkleTree<StateValue>,
    checkpoint_version: Option<Version>,
    // This doesn't need to be frozen since `_frozen_base` holds a ref to the oldest ancestor
    // already, but frozen SMT is used here anyway to avoid exposing the `batch_update()` interface
    // on the non-frozen SMT.
    latest: FrozenSparseMerkleTree<StateValue>,

    next_version: Version,
    updates_between_checkpoint_and_latest: HashMap<StateKey, Option<StateValue>>,
    updates_after_latest: HashMap<StateKey, Option<StateValue>>,
}

impl InMemoryStateCalculator {
    pub fn new(base: &StateDelta, state_cache: StateCache) -> Self {
        let StateCache {
            frozen_base,
            state_cache,
            proofs,
        } = state_cache;
        let StateDelta {
            base,
            base_version,
            current,
            current_version,
            updates_since_base,
        } = base.clone();

        Self {
            _frozen_base: frozen_base,
            state_cache,
            proof_reader: ProofReader::new(proofs),
            checkpoint: base,
            checkpoint_version: base_version,
            latest: current.freeze(),
            next_version: current_version.map_or(0, |v| v + 1),
            updates_between_checkpoint_and_latest: updates_since_base,
            updates_after_latest: HashMap::new(),
        }
    }

    pub fn calculate_for_transaction_chunk(
        mut self,
        to_keep: &[(Transaction, ParsedTransactionOutput)],
        new_epoch: bool,
    ) -> Result<(
        Vec<HashMap<StateKey, Option<StateValue>>>,
        Vec<Option<HashValue>>,
        StateDelta,
        Option<EpochState>,
    )> {
        let mut state_updates_vec = Vec::new();
        let mut state_checkpoint_hashes = Vec::new();

        for (txn, txn_output) in to_keep {
            let (state_updates, state_checkpoint_hash) = self.add_transaction(txn, txn_output)?;
            state_updates_vec.push(state_updates);
            state_checkpoint_hashes.push(state_checkpoint_hash);
        }
        let (result_state, accounts) = self.finish()?;

        // Get the updated validator set from updated account state.
        let next_epoch_state = if new_epoch {
            Some(Self::parse_validator_set(&accounts)?)
        } else {
            None
        };

        Ok((
            state_updates_vec,
            state_checkpoint_hashes,
            result_state,
            next_epoch_state,
        ))
    }

    fn add_transaction(
        &mut self,
        txn: &Transaction,
        txn_output: &ParsedTransactionOutput,
    ) -> Result<(HashMap<StateKey, Option<StateValue>>, Option<HashValue>)> {
        let updated_state_kvs = process_write_set(
            Some(txn),
            &mut self.state_cache,
            txn_output.write_set().clone(),
        )?;
        self.updates_after_latest.extend(updated_state_kvs.clone());
        self.next_version += 1;

        if txn_output.is_reconfig() {
            Ok((updated_state_kvs, Some(self.make_checkpoint()?)))
        } else {
            match txn {
                Transaction::BlockMetadata(_) | Transaction::UserTransaction(_) => {
                    Ok((updated_state_kvs, None))
                }
                Transaction::GenesisTransaction(_) | Transaction::StateCheckpoint(_) => {
                    Ok((updated_state_kvs, Some(self.make_checkpoint()?)))
                }
            }
        }
    }

    fn make_checkpoint(&mut self) -> Result<HashValue> {
        // Update SMT.
        let smt_updates: Vec<_> = self
            .updates_after_latest
            .iter()
            .map(|(key, value)| (key.hash(), value.as_ref()))
            .collect();
        let new_checkpoint = self.latest.batch_update(smt_updates, &self.proof_reader)?;
        let root_hash = new_checkpoint.root_hash();

        // Move self to the new checkpoint.
        self.latest = new_checkpoint.clone();
        self.checkpoint = new_checkpoint.unfreeze();
        self.checkpoint_version = self.next_version.checked_sub(1);
        self.updates_between_checkpoint_and_latest = HashMap::new();
        self.updates_after_latest = HashMap::new();

        Ok(root_hash)
    }

    fn parse_validator_set(state_cache: &HashMap<StateKey, StateValue>) -> Result<EpochState> {
        let account_state_view = state_cache.as_account_with_state_cache(&CORE_CODE_ADDRESS);
        let validator_set = account_state_view
            .get_validator_set()?
            .ok_or_else(|| anyhow!("ValidatorSet not touched on epoch change"))?;
        let configuration = account_state_view
            .get_configuration_resource()?
            .ok_or_else(|| anyhow!("Configuration resource not touched on epoch change"))?;

        Ok(EpochState {
            epoch: configuration.epoch(),
            verifier: (&validator_set).into(),
        })
    }

    fn finish(mut self) -> Result<(StateDelta, HashMap<StateKey, StateValue>)> {
        let smt_updates: Vec<_> = self
            .updates_after_latest
            .iter()
            .map(|(key, value)| (key.hash(), value))
            .collect();
        let latest = self.latest.batch_update(smt_updates, &self.proof_reader)?;

        self.updates_between_checkpoint_and_latest
            .extend(self.updates_after_latest);

        let result_state = StateDelta::new(
            self.checkpoint,
            self.checkpoint_version,
            latest.unfreeze(),
            self.next_version.checked_sub(1),
            self.updates_between_checkpoint_and_latest,
        );

        Ok((result_state, self.state_cache))
    }

    pub fn calculate_for_write_sets_after_snapshot(
        mut self,
        last_checkpoint_index: Option<usize>,
        write_sets: &[WriteSet],
    ) -> Result<(Option<HashMap<StateKey, StateValue>>, StateDelta)> {
        let idx_after_last_checkpoint = last_checkpoint_index.map_or(0, |idx| idx + 1);
        let updates_before_last_checkpoint = if idx_after_last_checkpoint != 0 {
            for write_set in write_sets[0..idx_after_last_checkpoint].iter() {
                let state_updates =
                    process_write_set(None, &mut self.state_cache, (*write_set).clone())?;
                self.updates_after_latest.extend(state_updates.into_iter());
                self.next_version += 1;
            }
            let updates = self.updates_after_latest.clone();
            self.make_checkpoint()?;
            Some(updates)
        } else {
            None
        };
        for write_set in write_sets[idx_after_last_checkpoint..].iter() {
            let state_updates =
                process_write_set(None, &mut self.state_cache, (*write_set).clone())?;
            self.updates_after_latest.extend(state_updates.into_iter());
            self.next_version += 1;
        }
        let (result_state, _) = self.finish()?;
        Ok((updates_before_last_checkpoint, result_state))
    }
}

// Checks the write set is a subset of the read set.
// Updates the `state_cache` to reflect the latest value.
// Returns all state key-value pair touched.
pub fn process_write_set(
    transaction: Option<&Transaction>,
    state_cache: &mut HashMap<StateKey, Option<StateValue>>,
    write_set: WriteSet,
) -> Result<HashMap<StateKey, Option<StateValue>>> {
    // Find all keys this transaction touches while processing each write op.
    write_set
        .into_iter()
        .map(|(state_key, write_op)| {
            process_state_key_write_op(transaction, state_cache, state_key, write_op)
        })
        .collect::<Result<_>>()
}

fn process_state_key_write_op(
    transaction: Option<&Transaction>,
    state_cache: &mut HashMap<StateKey, Option<StateValue>>,
    state_key: StateKey,
    write_op: WriteOp,
) -> Result<(StateKey, Option<StateValue>)> {
    let state_value = match write_op {
        WriteOp::Value(new_value) => Some(StateValue::from(new_value)),
        WriteOp::Deletion => None,
    };
    match state_cache.entry(state_key.clone()) {
        hash_map::Entry::Occupied(mut entry) => {
            entry.insert(state_value.clone());
        }
        hash_map::Entry::Vacant(entry) => {
            if let Some(txn) = transaction {
                ensure_txn_valid_for_vacant_entry(txn)?;
            }
            entry.insert(state_value.clone());
        }
    }
    Ok((state_key, state_value))
}

fn ensure_txn_valid_for_vacant_entry(transaction: &Transaction) -> Result<()> {
    // Before writing to an account, VM should always read that account. So we
    // should not reach this code path. The exception is genesis transaction (and
    // maybe other writeset transactions).
    match transaction {
        Transaction::GenesisTransaction(_) => (),
        Transaction::BlockMetadata(_) => {
            bail!("Write set should be a subset of read set.")
        }
        Transaction::UserTransaction(txn) => match txn.payload() {
            TransactionPayload::ModuleBundle(_)
            | TransactionPayload::Script(_)
            | TransactionPayload::ScriptFunction(_) => {
                bail!("Write set should be a subset of read set.")
            }
            TransactionPayload::WriteSet(_) => (),
        },
        Transaction::StateCheckpoint(_) => {}
    }
    Ok(())
}
