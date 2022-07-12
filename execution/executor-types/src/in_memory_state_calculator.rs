// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::collections::{hash_map, HashMap};

use anyhow::{anyhow, bail, Result};
use once_cell::sync::Lazy;

use crate::{ParsedTransactionOutput, ProofReader};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_state_view::account_with_state_cache::AsAccountWithStateCache;
use aptos_types::{
    account_view::AccountView,
    epoch_state::EpochState,
    event::EventKey,
    on_chain_config,
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::{Transaction, TransactionPayload, Version},
    write_set::{WriteOp, WriteSet},
};
use scratchpad::{FrozenSparseMerkleTree, SparseMerkleTree};
use storage_interface::{cached_state_view::StateCache, in_memory_state::InMemoryState};

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
    state_cache: HashMap<StateKey, StateValue>,
    proof_reader: ProofReader,

    checkpoint: SparseMerkleTree<StateValue>,
    checkpoint_version: Option<Version>,
    // This doesn't need to be frozen since `_frozen_base` holds a ref to the oldest ancestor
    // already, but frozen SMT is used here anyway to avoid exposing the `batch_update()` interface
    // on the non-frozen SMT.
    latest: FrozenSparseMerkleTree<StateValue>,

    next_version: Version,
    updates_between_checkpoint_and_latest: HashMap<StateKey, StateValue>,
    updates_after_latest: HashMap<StateKey, StateValue>,
}

impl InMemoryStateCalculator {
    pub fn new(base: &InMemoryState, state_cache: StateCache) -> Self {
        let StateCache {
            frozen_base,
            state_cache,
            proofs,
        } = state_cache;
        let InMemoryState {
            checkpoint,
            checkpoint_version,
            current,
            current_version,
            updated_since_checkpoint,
        } = base.clone();

        Self {
            _frozen_base: frozen_base,
            state_cache,
            proof_reader: ProofReader::new(proofs),
            checkpoint,
            checkpoint_version,
            latest: current.freeze(),
            next_version: current_version.map_or(0, |v| v + 1),
            updates_between_checkpoint_and_latest: updated_since_checkpoint,
            updates_after_latest: HashMap::new(),
        }
    }

    pub fn calculate_for_transaction_chunk(
        mut self,
        to_keep: &[(Transaction, ParsedTransactionOutput)],
        new_epoch: bool,
    ) -> Result<(
        Vec<HashMap<StateKey, StateValue>>,
        Vec<Option<HashValue>>,
        InMemoryState,
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
    ) -> Result<(HashMap<StateKey, StateValue>, Option<HashValue>)> {
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
            .map(|(key, value)| (key.hash(), value))
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
        let on_chain_config_address = on_chain_config::config_address();
        let account_state_view = state_cache.as_account_with_state_cache(&on_chain_config_address);
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

    fn finish(mut self) -> Result<(InMemoryState, HashMap<StateKey, StateValue>)> {
        let smt_updates: Vec<_> = self
            .updates_after_latest
            .iter()
            .map(|(key, value)| (key.hash(), value))
            .collect();
        let latest = self.latest.batch_update(smt_updates, &self.proof_reader)?;

        self.updates_between_checkpoint_and_latest
            .extend(self.updates_after_latest);

        let result_state = InMemoryState::new(
            self.checkpoint,
            self.checkpoint_version,
            latest.unfreeze(),
            self.next_version.checked_sub(1),
            self.updates_between_checkpoint_and_latest,
        );

        Ok((result_state, self.state_cache))
    }

    pub fn calculate_for_write_sets_after_checkpoint(
        mut self,
        write_sets: &[WriteSet],
    ) -> Result<InMemoryState> {
        for write_set in write_sets {
            let state_updates =
                process_write_set(None, &mut self.state_cache, (*write_set).clone())?;
            self.updates_after_latest.extend(state_updates.into_iter());
            self.next_version += 1;
        }
        let (result_state, _) = self.finish()?;
        Ok(result_state)
    }
}

// Checks the write set is a subset of the read set.
// Updates the `state_cache` to reflect the latest value.
// Returns all state key-value pair touched.
pub fn process_write_set(
    transaction: Option<&Transaction>,
    state_cache: &mut HashMap<StateKey, StateValue>,
    write_set: WriteSet,
) -> Result<HashMap<StateKey, StateValue>> {
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
    state_cache: &mut HashMap<StateKey, StateValue>,
    state_key: StateKey,
    write_op: WriteOp,
) -> Result<(StateKey, StateValue)> {
    let state_value = match write_op {
        WriteOp::Value(new_value) => StateValue::from(new_value),
        WriteOp::Deletion => StateValue::empty(),
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
