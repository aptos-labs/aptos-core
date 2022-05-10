// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::components::apply_chunk_output::ParsedTransactionOutput;
use anyhow::{anyhow, bail, Result};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_state_view::account_with_state_cache::AsAccountWithStateCache;
use aptos_types::{
    account_view::AccountView,
    epoch_state::EpochState,
    event::EventKey,
    nibble::nibble_path::NibblePath,
    on_chain_config,
    proof::{accumulator::InMemoryAccumulator, definition::LeafCount},
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::{Transaction, TransactionPayload, Version},
    write_set::{WriteOp, WriteSet},
};
use executor_types::{ExecutedTrees, ProofReader};
use once_cell::sync::Lazy;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use scratchpad::{FrozenSparseMerkleTree, SparseMerkleTree, StateStoreStatus};
use std::{
    collections::{hash_map, HashMap, HashSet},
    sync::Arc,
};
use storage_interface::{
    in_memory_state::InMemoryState, verified_state_view::StateCache, DbReader, TreeState,
};

pub trait IntoLedgerView {
    fn into_ledger_view(self, db: &Arc<dyn DbReader>) -> Result<ExecutedTrees>;
}

impl IntoLedgerView for TreeState {
    fn into_ledger_view(self, _db: &Arc<dyn DbReader>) -> Result<ExecutedTrees> {
        let checkpoint_num_txns = self.num_transactions;
        let checkpoint =
            InMemoryState::new_at_checkpoint(self.state_root_hash, checkpoint_num_txns);

        let transaction_accumulator = Arc::new(InMemoryAccumulator::new(
            self.ledger_frozen_subtree_hashes,
            self.num_transactions,
        )?);

        Ok(ExecutedTrees::new(checkpoint, transaction_accumulator))
    }
}

pub static NEW_EPOCH_EVENT_KEY: Lazy<EventKey> = Lazy::new(on_chain_config::new_epoch_event_key);

/// Helper class for calculating `InMemState` after a chunk or block of transactions are executed.
///
/// A new SMT is spawned in two situations:
///   1. a state checkpoint is encountered.
///   2. a transaction chunk or block ended (where `finish()` is called)
///
/// | ------------------------------------------ | -------------------------- |
/// |  (updated_between_checkpoint_and_latest)   |  (updated_after_latest)    |
/// \                                            \                            |
///  checkpoint SMT                               latest SMT                  |
///                                                                          /
///                                (creates checkpoint SMT on checkpoint txn)
///                                        (creates "latest SMT" on finish())
pub(crate) struct InMemoryStateCalculator {
    // This makes sure all in-mem nodes seen while proofs were fetched stays in mem during the
    // calculation
    _frozen_base: FrozenSparseMerkleTree<StateValue>,
    state_cache: HashMap<StateKey, StateValue>,
    proof_reader: ProofReader,

    checkpoint: SparseMerkleTree<StateValue>,
    checkpoint_num_transactions: LeafCount,
    // This doesn't need to be frozen since `_frozen_base` holds a ref to the oldest ancestor
    // already, but frozen SMT is used here anyway to avoid exposing the `batch_update()` interface
    // on the non-frozen SMT.
    latest: FrozenSparseMerkleTree<StateValue>,

    next_version: Version,
    updated_between_checkpoint_and_latest: HashSet<StateKey>,
    updated_after_latest: HashSet<StateKey>,
}

impl InMemoryStateCalculator {
    pub fn new(base: &InMemoryState, state_cache: StateCache, next_version: Version) -> Self {
        let StateCache {
            frozen_base,
            state_cache,
            proofs,
        } = state_cache;
        let InMemoryState {
            checkpoint,
            checkpoint_num_transactions,
            current,
            updated_since_checkpoint,
        } = base.clone();

        Self {
            _frozen_base: frozen_base,
            state_cache,
            proof_reader: ProofReader::new(proofs),
            checkpoint,
            checkpoint_num_transactions,
            latest: current.freeze(),
            next_version,
            updated_between_checkpoint_and_latest: updated_since_checkpoint,
            updated_after_latest: HashSet::new(),
        }
    }

    pub fn calculate_for_transaction_chunk(
        mut self,
        to_keep: &[(Transaction, ParsedTransactionOutput)],
        new_epoch: bool,
    ) -> Result<(
        Vec<HashMap<StateKey, StateValue>>,
        Vec<HashMap<NibblePath, HashValue>>,
        Vec<Option<HashValue>>,
        InMemoryState,
        Option<EpochState>,
    )> {
        let mut state_updates_vec = Vec::new();
        let mut new_node_hashes_vec = Vec::new();
        let mut state_checkpoint_hashes = Vec::new();

        for (txn, txn_output) in to_keep {
            let (state_updates, new_node_hashes, state_checkpoint_hash) =
                self.add_transaction(txn, txn_output)?;
            state_updates_vec.push(state_updates);
            new_node_hashes_vec.push(new_node_hashes);
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
            new_node_hashes_vec,
            state_checkpoint_hashes,
            result_state,
            next_epoch_state,
        ))
    }

    fn add_transaction(
        &mut self,
        txn: &Transaction,
        txn_output: &ParsedTransactionOutput,
    ) -> Result<(
        HashMap<StateKey, StateValue>,
        HashMap<NibblePath, HashValue>,
        Option<HashValue>,
    )> {
        let updated_state_keys = process_write_set(
            Some(txn),
            &mut self.state_cache,
            txn_output.write_set().clone(),
        )?;
        self.updated_after_latest
            .extend(updated_state_keys.into_iter());
        self.next_version += 1;

        if txn_output.is_reconfig() {
            self.checkpoint()
        } else {
            match txn {
                Transaction::BlockMetadata(_) | Transaction::UserTransaction(_) => {
                    // TODO(aldenhu): replace with: Ok((HashMap::new(), HashMap::new(), None))
                    self.checkpoint()
                }
                Transaction::GenesisTransaction(_) | Transaction::StateCheckpoint => {
                    self.checkpoint()
                }
            }
        }
    }

    fn checkpoint(
        &mut self,
    ) -> Result<(
        HashMap<StateKey, StateValue>,
        HashMap<NibblePath, HashValue>,
        Option<HashValue>,
    )> {
        // Update SMT.
        let updates_after_latest = self.updates_after_latest()?;
        let smt_updates: Vec<_> = updates_after_latest
            .iter()
            .map(|(key, value)| (key.hash(), value))
            .collect();
        let new_checkpoint = self.latest.batch_update(smt_updates, &self.proof_reader)?;
        let new_node_hashes =
            new_checkpoint.new_node_hashes_since(&self.checkpoint.clone().freeze());
        let root_hash = new_checkpoint.root_hash();

        // Calculate the set of state items that got changed since last checkpoint.
        let updated_between_checkpoint_and_latest: Vec<_> = self
            .updated_between_checkpoint_and_latest
            .difference(&self.updated_after_latest)
            .map(|key| match self.latest.get(key.hash()) {
                StateStoreStatus::ExistsInScratchPad(value) => Ok((key.clone(), value)),
                _ => Err(anyhow!(
                    "Pending state after checkpoint missing. key: {:?}",
                    key,
                )),
            })
            .collect::<Result<_>>()?;

        let mut state_updates = updates_after_latest;
        state_updates.extend(updated_between_checkpoint_and_latest.into_iter());

        // Move self to the new checkpoint.
        self.latest = new_checkpoint.clone();
        self.checkpoint = new_checkpoint.unfreeze();
        self.checkpoint_num_transactions = self.next_version;
        self.updated_between_checkpoint_and_latest = HashSet::new();
        self.updated_after_latest = HashSet::new();

        Ok((state_updates, new_node_hashes, Some(root_hash)))
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

    fn updates_after_latest(&self) -> Result<HashMap<StateKey, StateValue>> {
        self.updated_after_latest
            .iter()
            .collect::<Vec<_>>()
            .par_iter()
            .with_min_len(100)
            .map(|key| {
                Ok((
                    (**key).clone(),
                    self.state_cache
                        .get(key)
                        .ok_or_else(|| anyhow!("State value should exist."))?
                        .clone(),
                ))
            })
            .collect::<Result<_>>()
    }

    fn finish(self) -> Result<(InMemoryState, HashMap<StateKey, StateValue>)> {
        let updates_after_latest = self.updates_after_latest()?;
        let smt_updates: Vec<_> = updates_after_latest
            .iter()
            .map(|(key, value)| (key.hash(), value))
            .collect();
        let latest = self.latest.batch_update(smt_updates, &self.proof_reader)?;

        let mut updated_since_checkpoint = self.updated_between_checkpoint_and_latest;
        updated_since_checkpoint.extend(updates_after_latest.keys().cloned());

        let result_state = InMemoryState::new(
            self.checkpoint,
            self.checkpoint_num_transactions,
            latest.unfreeze(),
            updated_since_checkpoint,
        );

        Ok((result_state, self.state_cache))
    }

    #[allow(dead_code)]
    pub fn calculate_for_write_sets_after_checkpoint(
        mut self,
        write_sets: &[&WriteSet],
    ) -> Result<InMemoryState> {
        for write_set in write_sets {
            let state_updates =
                process_write_set(None, &mut self.state_cache, (*write_set).clone())?;
            self.updated_after_latest.extend(state_updates.into_iter());
            self.next_version += 1;
        }
        let (result_state, _) = self.finish()?;
        Ok(result_state)
    }
}

// Checks the write set is a subset of the read set.
// Updates the `state_cache` to reflect the latest value.
// Returns all state keys touched.
pub fn process_write_set(
    transaction: Option<&Transaction>,
    state_cache: &mut HashMap<StateKey, StateValue>,
    write_set: WriteSet,
) -> Result<HashSet<StateKey>> {
    // Find all keys this transaction touches while processing each write op.
    let mut updated_keys = HashSet::new();
    for (state_key, write_op) in write_set.into_iter() {
        process_state_key_write_op(
            transaction,
            state_cache,
            &mut updated_keys,
            state_key,
            write_op,
        )?;
    }

    Ok(updated_keys)
}

fn process_state_key_write_op(
    transaction: Option<&Transaction>,
    state_cache: &mut HashMap<StateKey, StateValue>,
    updated_keys: &mut HashSet<StateKey>,
    state_key: StateKey,
    write_op: WriteOp,
) -> Result<()> {
    match state_cache.entry(state_key.clone()) {
        hash_map::Entry::Occupied(mut entry) => {
            match write_op {
                WriteOp::Value(new_value) => entry.insert(StateValue::from(new_value)),
                WriteOp::Deletion => entry.insert(StateValue::empty()),
            };
        }
        hash_map::Entry::Vacant(entry) => {
            if let Some(txn) = transaction {
                ensure_txn_valid_for_vacant_entry(txn)?;
            }
            match write_op {
                WriteOp::Value(new_value) => entry.insert(StateValue::from(new_value)),
                WriteOp::Deletion => entry.insert(StateValue::empty()),
            };
        }
    }
    updated_keys.insert(state_key);
    Ok(())
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
        Transaction::StateCheckpoint => {}
    }
    Ok(())
}
