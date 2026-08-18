// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Loads and decodes the on-disk dump (`<version>_txns` / `<version>_inputs`): the transaction and
//! its read-set. The on-disk structs are re-declared here and BCS-decoded, so only the structural
//! shape has to match the producer.

use anyhow::{bail, Context};
use aptos_transaction_simulation::InMemoryStateStore;
use aptos_types::{
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::{
        PersistedAuxiliaryInfo, SignedTransaction, Transaction, TransactionBlock,
        TransactionExecutableRef, Version,
    },
};
use std::{collections::HashMap, path::Path as FsPath, sync::Arc};

/// Everything needed to benchmark a single entry-function transaction on both VMs.
pub struct BenchmarkInput {
    /// On-chain version of the transaction (for reporting).
    pub version: Version,
    /// The transaction to replay.
    pub txn: SignedTransaction,
    /// The transaction's on-chain auxiliary info (its index in the block).
    pub aux_info: PersistedAuxiliaryInfo,
    /// The state the transaction executes against.
    pub state: Arc<InMemoryStateStore>,
}

impl BenchmarkInput {
    /// The entry function's `module::function<type args>`, for reporting.
    pub fn function_label(&self) -> String {
        match self.txn.executable_ref() {
            Ok(TransactionExecutableRef::EntryFunction(entry)) => {
                let mut label = format!(
                    "{}::{}",
                    entry.module().short_str_lossless(),
                    entry.function()
                );
                if !entry.ty_args().is_empty() {
                    let ty_args = entry
                        .ty_args()
                        .iter()
                        .map(|t| t.to_canonical_string())
                        .collect::<Vec<_>>()
                        .join(", ");
                    label.push_str(&format!("<{}>", ty_args));
                }
                label
            },
            _ => "<not an entry function>".to_string(),
        }
    }
}

/// Loads the transaction blocks written by `download`.
fn load_transaction_blocks(path: impl AsRef<FsPath>) -> anyhow::Result<Vec<TransactionBlock>> {
    let bytes = std::fs::read(path.as_ref())
        .with_context(|| format!("Failed to read transactions file {:?}", path.as_ref()))?;
    bcs::from_bytes(&bytes).context("Failed to decode transaction blocks")
}

/// Loads the read-sets written by `capture`. Index-aligned with the transaction blocks.
pub fn load_read_sets(
    path: impl AsRef<FsPath>,
) -> anyhow::Result<Vec<HashMap<StateKey, StateValue>>> {
    let bytes = std::fs::read(path.as_ref())
        .with_context(|| format!("Failed to read inputs file {:?}", path.as_ref()))?;
    bcs::from_bytes(&bytes).context("Failed to decode read-sets")
}

/// Loads both files and produces one [`BenchmarkInput`] per entry-function user transaction found,
/// pairing each block with its read-set by index.
pub fn load_inputs(
    transactions_file: impl AsRef<FsPath>,
    inputs_file: impl AsRef<FsPath>,
) -> anyhow::Result<Vec<BenchmarkInput>> {
    let blocks = load_transaction_blocks(transactions_file)?;
    let read_sets = load_read_sets(inputs_file)?;
    if blocks.len() != read_sets.len() {
        bail!(
            "Number of transaction blocks ({}) does not match number of read-sets ({}); the \
            transactions and inputs files were likely produced from different runs.",
            blocks.len(),
            read_sets.len(),
        );
    }

    let mut inputs = vec![];
    for (block, read_set) in blocks.into_iter().zip(read_sets) {
        let state = InMemoryStateStore::new_with_state_values(read_set);
        crate::gas::make_gas_free(&state)?;
        let state = Arc::new(state);
        let mut version = block.begin_version;
        for (i, txn) in block.transactions.iter().enumerate() {
            if let Some(txn) = as_entry_function_transaction(txn) {
                let aux_info = block
                    .persisted_auxiliary_infos
                    .get(i)
                    .copied()
                    .unwrap_or(PersistedAuxiliaryInfo::None);
                inputs.push(BenchmarkInput {
                    version,
                    txn,
                    aux_info,
                    state: Arc::clone(&state),
                });
            }
            version += 1;
        }
    }
    Ok(inputs)
}

/// Loads every `<version>_txns` / `<version>_inputs` pair found in `dir` and concatenates the
/// resulting benchmark inputs. Useful for benchmarking a whole batch of downloaded transactions.
pub fn load_inputs_from_dir(dir: impl AsRef<FsPath>) -> anyhow::Result<Vec<BenchmarkInput>> {
    let dir = dir.as_ref();
    let mut txns_files = std::fs::read_dir(dir)
        .with_context(|| format!("Failed to read data directory {:?}", dir))?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with("_txns"))
        })
        .collect::<Vec<_>>();
    txns_files.sort();
    if txns_files.is_empty() {
        bail!("No `<version>_txns` files found in {:?}", dir);
    }

    let mut inputs = vec![];
    for txns_file in txns_files {
        let name = txns_file.file_name().and_then(|n| n.to_str()).unwrap();
        let prefix = name.strip_suffix("_txns").unwrap();
        let inputs_file = dir.join(format!("{}_inputs", prefix));
        if !inputs_file.exists() {
            bail!(
                "Missing inputs file for {:?} (expected {:?})",
                txns_file,
                inputs_file
            );
        }
        inputs.extend(load_inputs(&txns_file, &inputs_file)?);
    }
    Ok(inputs)
}

/// If `txn` is an entry-function user transaction, returns it. Both executors
/// derive everything else (signers, gas, session identity) from the
/// transaction themselves.
fn as_entry_function_transaction(txn: &Transaction) -> Option<SignedTransaction> {
    let signed = match txn {
        Transaction::UserTransaction(signed) => signed,
        _ => return None,
    };
    matches!(
        signed.executable_ref(),
        Ok(TransactionExecutableRef::EntryFunction(_))
    )
    .then(|| signed.clone())
}
