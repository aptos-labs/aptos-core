// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{components::chunk_output::ChunkOutput, metrics::APTOS_EXECUTOR_ERRORS};
use anyhow::{anyhow, bail, ensure, Result};
use aptos_crypto::{
    hash::{CryptoHash, EventAccumulatorHasher, TransactionAccumulatorHasher},
    HashValue,
};
use aptos_logger::error;
use aptos_types::{
    access_path::AccessPath,
    account_address::AccountAddress,
    account_state::AccountState,
    account_state_blob::AccountStateBlob,
    contract_event::ContractEvent,
    epoch_state::EpochState,
    event::EventKey,
    nibble::nibble_path::NibblePath,
    on_chain_config,
    proof::accumulator::InMemoryAccumulator,
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::{
        Transaction, TransactionInfo, TransactionOutput, TransactionPayload, TransactionStatus,
    },
    write_set::{WriteOp, WriteSet},
};
use executor_types::{ExecutedChunk, ExecutedTrees, ProofReader, TransactionData};
use once_cell::sync::Lazy;
use scratchpad::SparseMerkleTree;
use std::{
    collections::{hash_map, HashMap, HashSet},
    convert::TryFrom,
    iter::repeat,
    ops::Deref,
    sync::Arc,
};
use storage_interface::{verified_state_view::StateCache, DbReader, TreeState};

pub struct ApplyChunkOutput;

impl ApplyChunkOutput {
    pub fn apply(
        chunk_output: ChunkOutput,
        base_accumulator: &Arc<InMemoryAccumulator<TransactionAccumulatorHasher>>,
    ) -> Result<(ExecutedChunk, Vec<Transaction>, Vec<Transaction>)> {
        let ChunkOutput {
            state_cache,
            transactions,
            transaction_outputs,
        } = chunk_output;

        // Separate transactions with different VM statuses.
        let (new_epoch, status, to_keep, to_discard, to_retry) =
            Self::sort_transactions(transactions, transaction_outputs)?;

        // Apply the write set, get the latest state.
        let (state_store_update, roots_with_node_hashes, result_state, next_epoch_state) =
            Self::apply_write_set(state_cache, new_epoch, &to_keep)?;

        // Calculate TransactionData and TransactionInfo, i.e. the ledger history diff.
        let (to_commit, transaction_info_hashes) =
            Self::assemble_ledger_diff(to_keep, state_store_update, roots_with_node_hashes);

        Ok((
            ExecutedChunk {
                status,
                to_commit,
                result_view: ExecutedTrees::new_copy(
                    result_state,
                    Arc::new(base_accumulator.append(&transaction_info_hashes)),
                ),
                next_epoch_state,
                ledger_info: None,
            },
            to_discard,
            to_retry,
        ))
    }

    fn sort_transactions(
        mut transactions: Vec<Transaction>,
        transaction_outputs: Vec<TransactionOutput>,
    ) -> Result<(
        bool,
        Vec<TransactionStatus>,
        Vec<(Transaction, ParsedTransactionOutput)>,
        Vec<Transaction>,
        Vec<Transaction>,
    )> {
        let num_txns = transactions.len();
        let mut transaction_outputs: Vec<ParsedTransactionOutput> =
            transaction_outputs.into_iter().map(Into::into).collect();
        // N.B. off-by-1 intentionally, for exclusive index
        let new_epoch_marker = transaction_outputs
            .iter()
            .position(|o| o.is_reconfig())
            .map(|idx| idx + 1);

        // Transactions after the epoch ending are all to be retried.
        let to_retry = if let Some(pos) = new_epoch_marker {
            transaction_outputs.drain(pos..);
            transactions.drain(pos..).collect()
        } else {
            vec![]
        };

        // N.B. Transaction status after the epoch marker are ignored and set to Retry forcibly.
        let status = transaction_outputs
            .iter()
            .map(|t| t.status())
            .cloned()
            .chain(repeat(TransactionStatus::Retry))
            .take(num_txns)
            .collect();

        // Separate transactions with the Keep status out.
        let (to_keep, to_discard) =
            itertools::zip_eq(transactions.into_iter(), transaction_outputs.into_iter())
                .partition::<Vec<(Transaction, ParsedTransactionOutput)>, _>(|(_, o)| {
                    matches!(o.status(), TransactionStatus::Keep(_))
                });

        // Sanity check transactions with the Discard status:
        let to_discard = to_discard
            .into_iter()
            .map(|(t, o)| {
                // In case a new status other than Retry, Keep and Discard is added:
                if !matches!(o.status(), TransactionStatus::Discard(_)) {
                    error!("Status other than Retry, Keep or Discard; Transaction discarded.");
                }
                // VM shouldn't have output anything for discarded transactions, log if it did.
                if !o.write_set().is_empty() || !o.events().is_empty() {
                    error!(
                        "Discarded transaction has non-empty write set or events. \
                     Transaction: {:?}. Status: {:?}.",
                        t,
                        o.status(),
                    );
                    APTOS_EXECUTOR_ERRORS.inc();
                }
                Ok(t)
            })
            .collect::<Result<Vec<_>>>()?;

        Ok((
            new_epoch_marker.is_some(),
            status,
            to_keep,
            to_discard,
            to_retry,
        ))
    }

    fn apply_write_set(
        state_cache: StateCache,
        new_epoch: bool,
        to_keep: &[(Transaction, ParsedTransactionOutput)],
    ) -> Result<(
        Vec<HashMap<StateKey, StateValue>>,
        Vec<(HashValue, HashMap<NibblePath, HashValue>)>,
        SparseMerkleTree<StateValue>,
        Option<EpochState>,
    )> {
        let StateCache {
            frozen_base,
            mut accounts,
            mut state_cache,
            proofs,
        } = state_cache;

        // Apply write sets to account states in the AccountCache, resulting in new account states.
        let state_store_updates = to_keep
            .iter()
            .map(|(t, o)| {
                process_write_set(t, &mut accounts, &mut state_cache, o.write_set().clone())
            })
            .collect::<Result<Vec<_>>>()?;

        // Apply new account states to the base state tree, resulting in updated state tree.
        let (roots_with_node_hashes, result_state) = frozen_base
            .serial_update(
                Self::state_store_updates_to_smt_updates(&state_store_updates),
                &ProofReader::new(proofs),
            )
            .map_err(|e| anyhow!("Failed to update state tree. err: {:?}", e))?;
        // Release ASAP the ref to the base SMT to allow old in-mem nodes to be dropped,
        // now that we don't require access to them.
        let result_state = result_state.unfreeze();

        // Get the updated validator set from updated account state.
        let next_epoch_state = if new_epoch {
            Some(Self::parse_validator_set(&accounts)?)
        } else {
            None
        };

        Ok((
            state_store_updates,
            roots_with_node_hashes,
            result_state,
            next_epoch_state,
        ))
    }

    fn state_store_updates_to_smt_updates(
        account_blobs: &[HashMap<StateKey, StateValue>],
    ) -> Vec<Vec<(HashValue, &StateValue)>> {
        account_blobs
            .iter()
            .map(|m| {
                m.iter()
                    .map(|(key, value)| (key.hash(), value))
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    fn parse_validator_set(accounts: &HashMap<AccountAddress, AccountState>) -> Result<EpochState> {
        let validator_set = accounts
            .get(&on_chain_config::config_address())
            .map(|state| {
                state
                    .get_validator_set()?
                    .ok_or_else(|| anyhow!("ValidatorSet does not exist"))
            })
            .ok_or_else(|| anyhow!("ValidatorSet account does not exist"))??;
        let configuration = accounts
            .get(&on_chain_config::config_address())
            .map(|state| {
                state
                    .get_configuration_resource()?
                    .ok_or_else(|| anyhow!("Configuration does not exist"))
            })
            .ok_or_else(|| anyhow!("Association account does not exist"))??;

        Ok(EpochState {
            epoch: configuration.epoch(),
            verifier: (&validator_set).into(),
        })
    }

    fn assemble_ledger_diff(
        to_keep: Vec<(Transaction, ParsedTransactionOutput)>,
        state_updates: Vec<HashMap<StateKey, StateValue>>,
        roots_with_node_hashes: Vec<(HashValue, HashMap<NibblePath, HashValue>)>,
    ) -> (Vec<(Transaction, TransactionData)>, Vec<HashValue>) {
        let mut to_commit = vec![];
        let mut txn_info_hashes = vec![];
        for ((txn, txn_output), ((state_tree_hash, new_node_hashes), state_store_update)) in
            itertools::zip_eq(
                to_keep,
                itertools::zip_eq(roots_with_node_hashes, state_updates),
            )
        {
            let (write_set, events, reconfig_events, gas_used, status) = txn_output.unpack();
            let event_tree = {
                let event_hashes: Vec<_> = events.iter().map(CryptoHash::hash).collect();
                InMemoryAccumulator::<EventAccumulatorHasher>::from_leaves(&event_hashes)
            };

            let txn_info = match &status {
                TransactionStatus::Keep(status) => TransactionInfo::new(
                    txn.hash(),
                    state_tree_hash,
                    event_tree.root_hash(),
                    gas_used,
                    status.clone(),
                ),
                _ => unreachable!("Transaction sorted by status already."),
            };

            let txn_info_hash = txn_info.hash();
            txn_info_hashes.push(txn_info_hash);
            to_commit.push((
                txn,
                TransactionData::new(
                    state_store_update,
                    new_node_hashes,
                    write_set,
                    events,
                    reconfig_events,
                    status,
                    Arc::new(event_tree),
                    gas_used,
                    txn_info,
                    txn_info_hash,
                ),
            ))
        }
        (to_commit, txn_info_hashes)
    }
}

pub fn ensure_no_discard(to_discard: Vec<Transaction>) -> Result<()> {
    ensure!(to_discard.is_empty(), "Syncing discarded transactions");
    Ok(())
}

pub fn ensure_no_retry(to_retry: Vec<Transaction>) -> Result<()> {
    ensure!(to_retry.is_empty(), "Chunk crosses epoch boundary.",);
    Ok(())
}

fn process_access_path_write_op(
    transaction: &Transaction,
    account_to_state: &mut HashMap<AccountAddress, AccountState>,
    addresses: &mut HashSet<StateKey>,
    access_path: AccessPath,
    write_op: WriteOp,
) -> Result<()> {
    let address = access_path.address;
    let path = access_path.path;
    match account_to_state.entry(address) {
        hash_map::Entry::Occupied(mut entry) => {
            update_account_state(entry.get_mut(), path, write_op);
        }
        hash_map::Entry::Vacant(entry) => {
            ensure_txn_valid_for_vacant_entry(transaction)?;
            let mut account_state = Default::default();
            update_account_state(&mut account_state, path, write_op);
            entry.insert(account_state);
        }
    }
    addresses.insert(StateKey::AccountAddressKey(address));
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

fn process_state_key_write_op(
    transaction: &Transaction,
    state_cache: &mut HashMap<StateKey, StateValue>,
    addresses: &mut HashSet<StateKey>,
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
            ensure_txn_valid_for_vacant_entry(transaction)?;
            match write_op {
                WriteOp::Value(new_value) => entry.insert(StateValue::from(new_value)),
                WriteOp::Deletion => entry.insert(StateValue::empty()),
            };
        }
    }
    addresses.insert(state_key);
    Ok(())
}

/// For all accounts modified by this transaction, find the previous blob and update it based
/// on the write set. Returns the blob value of all these accounts.
pub fn process_write_set(
    transaction: &Transaction,
    account_to_state: &mut HashMap<AccountAddress, AccountState>,
    state_cache: &mut HashMap<StateKey, StateValue>,
    write_set: WriteSet,
) -> Result<HashMap<StateKey, StateValue>> {
    let mut state_updates = HashMap::new();

    // Find all addresses this transaction touches while processing each write op.
    let mut updated_keys = HashSet::new();
    for (state_key, write_op) in write_set.into_iter() {
        match &state_key {
            StateKey::AccessPath(access_path) => process_access_path_write_op(
                transaction,
                account_to_state,
                &mut updated_keys,
                access_path.clone(),
                write_op,
            )?,
            StateKey::AccountAddressKey(_) => {
                bail!("Account address state key is not expected in write set")
            }
            // For now, we only support write set with access path, this needs to be updated once
            // we support table items
            StateKey::Raw(_) => process_state_key_write_op(
                transaction,
                state_cache,
                &mut updated_keys,
                state_key,
                write_op,
            )?,
        }
    }

    for state_key in updated_keys {
        match state_key {
            StateKey::AccountAddressKey(address) => {
                let account_state = account_to_state
                    .get(&address)
                    .expect("Address should exist.");
                state_updates.insert(
                    state_key,
                    StateValue::from(AccountStateBlob::try_from(account_state)?),
                );
            }
            _ => {
                let state_value = state_cache
                    .get(&state_key)
                    .expect("State value should exist.");
                state_updates.insert(state_key, state_value.clone());
            }
        }
    }

    Ok(state_updates)
}

fn update_account_state(account_state: &mut AccountState, path: Vec<u8>, write_op: WriteOp) {
    match write_op {
        WriteOp::Value(new_value) => account_state.insert(path, new_value),
        WriteOp::Deletion => account_state.remove(&path),
    };
}

pub trait IntoLedgerView {
    fn into_ledger_view(self, db: &Arc<dyn DbReader>) -> Result<ExecutedTrees>;
}

impl IntoLedgerView for TreeState {
    fn into_ledger_view(self, _db: &Arc<dyn DbReader>) -> Result<ExecutedTrees> {
        Ok(ExecutedTrees::new(
            self.account_state_root_hash,
            self.ledger_frozen_subtree_hashes,
            self.num_transactions,
        ))
    }
}

static NEW_EPOCH_EVENT_KEY: Lazy<EventKey> = Lazy::new(on_chain_config::new_epoch_event_key);

struct ParsedTransactionOutput {
    output: TransactionOutput,
    reconfig_events: Vec<ContractEvent>,
}

impl From<TransactionOutput> for ParsedTransactionOutput {
    fn from(output: TransactionOutput) -> Self {
        let reconfig_events = output
            .events()
            .iter()
            .filter(|e| *e.key() == *NEW_EPOCH_EVENT_KEY)
            .cloned()
            .collect();
        Self {
            output,
            reconfig_events,
        }
    }
}

impl Deref for ParsedTransactionOutput {
    type Target = TransactionOutput;

    fn deref(&self) -> &Self::Target {
        &self.output
    }
}

impl ParsedTransactionOutput {
    fn is_reconfig(&self) -> bool {
        !self.reconfig_events.is_empty()
    }

    pub fn unpack(
        self,
    ) -> (
        WriteSet,
        Vec<ContractEvent>,
        Vec<ContractEvent>,
        u64,
        TransactionStatus,
    ) {
        let Self {
            output,
            reconfig_events,
        } = self;
        let (write_set, events, gas_used, status) = output.unpack();

        (write_set, events, reconfig_events, gas_used, status)
    }
}
