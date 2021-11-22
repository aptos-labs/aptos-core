// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use std::{
    collections::{hash_map, HashMap, HashSet},
    convert::TryFrom,
    marker::PhantomData,
    sync::Arc,
};

use anyhow::{bail, ensure, format_err, Result};
use components::speculation_cache::SpeculationCache;
use fail::fail_point;

use diem_crypto::{
    hash::{CryptoHash, EventAccumulatorHasher, TransactionAccumulatorHasher},
    HashValue,
};
use diem_infallible::{RwLock, RwLockReadGuard};
use diem_logger::prelude::*;
use diem_state_view::{StateView, StateViewId};
use diem_types::{
    account_address::{AccountAddress, HashAccountAddress},
    account_state::AccountState,
    account_state_blob::AccountStateBlob,
    contract_event::ContractEvent,
    epoch_state::EpochState,
    on_chain_config,
    proof::accumulator::InMemoryAccumulator,
    protocol_spec::{DpnProto, ProtocolSpec},
    transaction::{
        Transaction, TransactionInfoTrait, TransactionOutput, TransactionPayload,
        TransactionStatus, TransactionToCommit,
    },
    write_set::{WriteOp, WriteSet, WriteSetMut},
};
use diem_vm::VMExecutor;
use executor_types::{Error, ExecutedTrees, ProcessedVMOutput, ProofReader, TransactionData};
use rayon::prelude::*;
use storage_interface::{
    default_protocol::DbReaderWriter, state_view::VerifiedStateView, TreeState,
};

use crate::metrics::DIEM_EXECUTOR_ERRORS;

#[cfg(any(test, feature = "fuzzing"))]
pub mod fuzzing;
mod logging;
pub mod metrics;
#[cfg(test)]
mod mock_vm;
#[cfg(test)]
mod tests;

pub mod block_executor_impl;
pub mod chunk_executor;
mod components;
pub mod db_bootstrapper;
mod transaction_replayer_impl;

/// `Executor` implements all functionalities the execution module needs to provide.
pub struct Executor<PS, V> {
    db: DbReaderWriter,
    cache: RwLock<SpeculationCache>,
    phantom: PhantomData<(PS, V)>,
}

impl<PS, V> Executor<PS, V>
where
    PS: ProtocolSpec,
    V: VMExecutor,
{
    pub fn committed_block_id(&self) -> HashValue {
        self.cache.read().committed_block_id()
    }

    /// Constructs an `Executor`.
    pub fn new(db: DbReaderWriter) -> Self {
        let startup_info = db
            .reader
            .get_startup_info()
            .expect("Shouldn't fail")
            .expect("DB not bootstrapped.");

        Self {
            db,
            cache: RwLock::new(SpeculationCache::new_with_startup_info(startup_info)),
            phantom: PhantomData,
        }
    }

    fn reset_cache(&self) -> Result<(), Error> {
        let startup_info = self
            .db
            .reader
            .get_startup_info()?
            .ok_or_else(|| format_err!("DB not bootstrapped."))?;
        *self.cache.write() = SpeculationCache::new_with_startup_info(startup_info);
        Ok(())
    }

    pub fn new_on_unbootstrapped_db(db: DbReaderWriter, tree_state: TreeState) -> Self {
        Self {
            db,
            cache: RwLock::new(SpeculationCache::new_for_db_bootstrapping(tree_state)),
            phantom: PhantomData,
        }
    }

    /// Post-processing of what the VM outputs. Returns the entire block's output.
    fn process_vm_outputs(
        transactions: &[Transaction],
        vm_outputs: Vec<TransactionOutput>,
        // the one used by the vm during the execution which generated `vm_output`
        state_view: VerifiedStateView<DpnProto>,
        parent_transaction_accumulator: &Arc<InMemoryAccumulator<TransactionAccumulatorHasher>>,
    ) -> Result<ProcessedVMOutput> {
        let (mut account_to_state, account_to_proof, parent_state) =
            state_view.unpack_after_execution();
        // The data of each individual transaction. For convenience purpose, even for the
        // transactions that will be discarded, we will compute its in-memory Sparse Merkle Tree
        // (it will be identical to the previous one).
        let mut txn_data = vec![];
        // The hash of each individual PS::TransactionInfo object. This will not include the
        // transactions that will be discarded, since they do not go into the transaction
        // accumulator.
        let mut txn_info_hashes = vec![];

        let proof_reader = ProofReader::new(account_to_proof);
        let new_epoch_event_key = on_chain_config::new_epoch_event_key();

        let new_epoch_marker = vm_outputs
            .iter()
            .enumerate()
            .find(|(_, output)| {
                output
                    .events()
                    .iter()
                    .any(|event| *event.key() == new_epoch_event_key)
            })
            // Off by one for exclusive index.
            .map(|(idx, _)| idx + 1);
        let transaction_count = new_epoch_marker.unwrap_or(vm_outputs.len());

        let txn_states = itertools::zip_eq(vm_outputs.iter(), transactions.iter())
            .take(transaction_count)
            .map(|(vm_output, txn)| {
                process_write_set(txn, &mut account_to_state, vm_output.write_set().clone())
            })
            .collect::<Result<Vec<_>>>()?;
        let txn_blobs = txn_states
            .par_iter()
            .with_min_len(100)
            .map(|account_to_state| {
                account_to_state
                    .iter()
                    .map(|(addr, state)| Ok((*addr, AccountStateBlob::try_from(state)?)))
                    .collect::<Result<HashMap<_, _>>>()
            })
            .collect::<Result<Vec<_>>>()?;

        let (roots_with_node_hashes, current_state_tree) = parent_state
            .serial_update(
                txn_blobs
                    .iter()
                    .map(|m| {
                        m.iter()
                            .map(|(account, value)| (account.hash(), value))
                            .collect::<Vec<_>>()
                    })
                    .collect(),
                &proof_reader,
            )
            .map_err(|e| format_err!("Failed to update state tree. err: {:?}", e))?;

        for ((vm_output, txn), ((state_tree_hash, new_node_hashes), blobs)) in itertools::zip_eq(
            itertools::zip_eq(vm_outputs.into_iter(), transactions.iter()).take(transaction_count),
            itertools::zip_eq(roots_with_node_hashes, txn_blobs),
        ) {
            let event_tree = {
                let event_hashes: Vec<_> =
                    vm_output.events().iter().map(CryptoHash::hash).collect();
                InMemoryAccumulator::<EventAccumulatorHasher>::from_leaves(&event_hashes)
            };

            let txn_info_hash = match vm_output.status() {
                TransactionStatus::Keep(status) => {
                    ensure!(
                        !vm_output.write_set().is_empty(),
                        "Transaction with empty write set should be discarded.",
                    );
                    // Compute hash for the PS::TransactionInfo object. We need the hash of the
                    // transaction itself, the state root hash as well as the event root hash.
                    let txn_info_hash = PS::TransactionInfo::new(
                        txn.hash(),
                        state_tree_hash,
                        event_tree.root_hash(),
                        vm_output.gas_used(),
                        status.clone(),
                    )
                    .hash();
                    txn_info_hashes.push(txn_info_hash);
                    Some(txn_info_hash)
                }
                TransactionStatus::Discard(status) => {
                    if !vm_output.write_set().is_empty() || !vm_output.events().is_empty() {
                        error!(
                            "Discarded transaction has non-empty write set or events. \
                             Transaction: {:?}. Status: {:?}.",
                            txn, status,
                        );
                        DIEM_EXECUTOR_ERRORS.inc();
                    }
                    None
                }
                TransactionStatus::Retry => None,
            };

            txn_data.push(TransactionData::new(
                blobs,
                new_node_hashes,
                vm_output.write_set().clone(),
                vm_output.events().to_vec(),
                vm_output.status().clone(),
                state_tree_hash,
                Arc::new(event_tree),
                vm_output.gas_used(),
                txn_info_hash,
            ));
        }

        // check for change in validator set
        let next_epoch_state = if new_epoch_marker.is_some() {
            // Pad the rest of transactions
            txn_data.resize(
                transactions.len(),
                TransactionData::new(
                    HashMap::new(),
                    HashMap::new(),
                    WriteSetMut::new(vec![])
                        .freeze()
                        .expect("generated write sets should always be valid"),
                    vec![],
                    TransactionStatus::Retry,
                    current_state_tree.root_hash(),
                    Arc::new(InMemoryAccumulator::<EventAccumulatorHasher>::default()),
                    0,
                    None,
                ),
            );

            let validator_set = account_to_state
                .get(&on_chain_config::config_address())
                .map(|state| {
                    state
                        .get_validator_set()?
                        .ok_or_else(|| format_err!("ValidatorSet does not exist"))
                })
                .ok_or_else(|| format_err!("ValidatorSet account does not exist"))??;
            let configuration = account_to_state
                .get(&on_chain_config::config_address())
                .map(|state| {
                    state
                        .get_configuration_resource()?
                        .ok_or_else(|| format_err!("Configuration does not exist"))
                })
                .ok_or_else(|| format_err!("Association account does not exist"))??;
            Some(EpochState {
                epoch: configuration.epoch(),
                verifier: (&validator_set).into(),
            })
        } else {
            None
        };

        let current_transaction_accumulator =
            parent_transaction_accumulator.append(&txn_info_hashes);

        Ok(ProcessedVMOutput::new(
            txn_data,
            ExecutedTrees::new_copy(
                current_state_tree.unfreeze(),
                Arc::new(current_transaction_accumulator),
            ),
            next_epoch_state,
        ))
    }

    fn get_executed_trees_from_lock(
        cache: &RwLockReadGuard<SpeculationCache>,
        block_id: HashValue,
    ) -> Result<ExecutedTrees, Error> {
        let executed_trees = if block_id == cache.committed_block_id() {
            cache.committed_trees().clone()
        } else {
            cache
                .get_block(&block_id)?
                .lock()
                .output()
                .executed_trees()
                .clone()
        };

        Ok(executed_trees)
    }

    fn get_executed_state_view_from_lock<'a>(
        &self,
        cache: &RwLockReadGuard<SpeculationCache>,
        id: StateViewId,
        executed_trees: &'a ExecutedTrees,
    ) -> VerifiedStateView<DpnProto> {
        VerifiedStateView::new(
            id,
            Arc::clone(&self.db.reader),
            cache.committed_trees().version(),
            cache.committed_trees().state_root(),
            executed_trees.state_tree().clone(),
        )
    }

    fn get_executed_trees(&self, block_id: HashValue) -> Result<ExecutedTrees, Error> {
        let read_lock = self.cache.read();
        Self::get_executed_trees_from_lock(&read_lock, block_id)
    }

    fn get_executed_state_view(
        &self,
        id: StateViewId,
        executed_trees: &ExecutedTrees,
    ) -> VerifiedStateView<DpnProto> {
        let read_lock = self.cache.read();
        self.get_executed_state_view_from_lock(&read_lock, id, executed_trees)
    }

    fn replay_transactions_impl(
        &self,
        first_version: u64,
        transactions: Vec<Transaction>,
        transaction_outputs: Option<Vec<TransactionOutput>>,
        transaction_infos: Vec<PS::TransactionInfo>,
    ) -> Result<(
        ProcessedVMOutput,
        Vec<TransactionToCommit>,
        Vec<ContractEvent>,
        Vec<Transaction>,
        Vec<PS::TransactionInfo>,
    )> {
        let read_lock = self.cache.read();
        // Construct a StateView and pass the transactions to VM.
        let state_view = VerifiedStateView::new(
            StateViewId::ChunkExecution { first_version },
            Arc::clone(&self.db.reader),
            read_lock.synced_trees().version(),
            read_lock.synced_trees().state_root(),
            read_lock.synced_trees().state_tree().clone(),
        );

        fail_point!("executor::vm_execute_chunk", |_| {
            Err(anyhow::anyhow!("Injected error in execute_chunk"))
        });

        let transactions_executed = transaction_outputs.is_none();
        let vm_outputs = if let Some(outputs) = transaction_outputs {
            ensure!(
                transactions.len() == outputs.len(),
                "the number of transactions {} doesn't \
                 match the number of transactions outputs {}.",
                transactions.len(),
                outputs.len()
            );
            for (access_path, _) in outputs.iter().map(|o| o.write_set()).flatten() {
                state_view.get(access_path)?;
            }
            outputs
        } else {
            V::execute_block(transactions.clone(), &state_view)?
        };

        // Since other validators have committed these transactions, their status should all be
        // TransactionStatus::Keep.
        for (index, output) in vm_outputs.iter().enumerate() {
            if let TransactionStatus::Discard(status_code) = output.status() {
                let bail_error_message = format!(
                    "Syncing a transaction that should be discarded! Transaction version: {:?}, status code: {:?}.",
                    first_version + index as u64, status_code
                );
                error!("{}", bail_error_message);
                info!("Discarded transaction: {:?}", transactions[index]);
                info!("Discarded transaction output: {:?}", output);
                info!("Transactions were executed: {:?}", transactions_executed);
                bail!(bail_error_message);
            }
        }

        let output = Self::process_vm_outputs(
            &transactions,
            vm_outputs,
            state_view,
            read_lock.synced_trees().txn_accumulator(),
        )?;

        // Since we have verified the proofs, we just need to verify that each PS::TransactionInfo
        // object matches what we have computed locally.
        let mut txns_to_commit = vec![];
        let mut events = vec![];
        let mut seen_retry = false;
        let mut txns_to_retry = vec![];
        let mut txn_infos_to_retry = vec![];
        for ((txn, txn_data), (i, txn_info)) in itertools::zip_eq(
            itertools::zip_eq(transactions, output.transaction_data()),
            transaction_infos.into_iter().enumerate(),
        ) {
            let recorded_status = match txn_data.status() {
                TransactionStatus::Keep(recorded_status) => recorded_status.clone(),
                status @ TransactionStatus::Discard(_) => bail!(
                    "The transaction at version {}, got the status of 'Discard': {:?}",
                    first_version
                        .checked_add(i as u64)
                        .ok_or_else(|| format_err!("version + i overflows"))?,
                    status
                ),
                TransactionStatus::Retry => {
                    seen_retry = true;
                    txns_to_retry.push(txn);
                    txn_infos_to_retry.push(txn_info);
                    continue;
                }
            };
            assert!(!seen_retry);
            let generated_txn_info = PS::TransactionInfo::new(
                txn.hash(),
                txn_data.state_root_hash(),
                txn_data.event_root_hash(),
                txn_data.gas_used(),
                recorded_status.clone(),
            );
            ensure!(
                txn_info == generated_txn_info,
                "txn_info do not match for {}-th transaction in chunk.\nChunk txn_info: {}\nProof txn_info: {}",
                i, generated_txn_info, txn_info
            );
            txns_to_commit.push(TransactionToCommit::new(
                txn,
                txn_data.account_blobs().clone(),
                Some(txn_data.jf_node_hashes().clone()),
                txn_data.write_set().clone(),
                txn_data.events().to_vec(),
                txn_data.gas_used(),
                recorded_status,
            ));
            events.append(&mut txn_data.events().to_vec());
        }

        Ok((
            output,
            txns_to_commit,
            events,
            txns_to_retry,
            txn_infos_to_retry,
        ))
    }
}

/// For all accounts modified by this transaction, find the previous blob and update it based
/// on the write set. Returns the blob value of all these accounts.
pub fn process_write_set(
    transaction: &Transaction,
    account_to_state: &mut HashMap<AccountAddress, AccountState>,
    write_set: WriteSet,
) -> Result<HashMap<AccountAddress, AccountState>> {
    let mut updated_blobs = HashMap::new();

    // Find all addresses this transaction touches while processing each write op.
    let mut addrs = HashSet::new();
    for (access_path, write_op) in write_set.into_iter() {
        let address = access_path.address;
        let path = access_path.path;
        match account_to_state.entry(address) {
            hash_map::Entry::Occupied(mut entry) => {
                update_account_state(entry.get_mut(), path, write_op);
            }
            hash_map::Entry::Vacant(entry) => {
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
                }

                let mut account_state = Default::default();
                update_account_state(&mut account_state, path, write_op);
                entry.insert(account_state);
            }
        }
        addrs.insert(address);
    }

    for addr in addrs {
        let account_state = account_to_state.get(&addr).expect("Address should exist.");
        updated_blobs.insert(addr, account_state.clone());
    }

    Ok(updated_blobs)
}

fn update_account_state(account_state: &mut AccountState, path: Vec<u8>, write_op: WriteOp) {
    match write_op {
        WriteOp::Value(new_value) => account_state.insert(path, new_value),
        WriteOp::Deletion => account_state.remove(&path),
    };
}
