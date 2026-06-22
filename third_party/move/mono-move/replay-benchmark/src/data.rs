// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Loads and decodes the on-disk dump (`<version>_txns` / `<version>_inputs`): the transaction and
//! its read-set. The on-disk structs are re-declared here and BCS-decoded, so only the structural
//! shape has to match the producer.

use anyhow::{bail, Context};
use aptos_types::{
    access_path::Path,
    state_store::{
        state_key::{inner::StateKeyInner, StateKey},
        state_slot::{StateSlot, StateSlotKind},
        state_storage_usage::StateStorageUsage,
        state_value::StateValue,
        StateViewResult, TStateView,
    },
    transaction::{
        user_transaction_context::{TransactionIndexKind, UserTransactionContext},
        EntryFunction, PersistedAuxiliaryInfo, Transaction, TransactionExecutableRef, Version,
    },
};
use move_core_types::{
    account_address::AccountAddress,
    language_storage::{ModuleId, StructTag},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    path::Path as FsPath,
    sync::Arc,
};

/// On-disk representation of a block of transactions, matching
/// `aptos_replay_benchmark::workload::TransactionBlock`.
#[derive(Serialize, Deserialize)]
pub struct TransactionBlock {
    /// The version of the first transaction in the block.
    pub begin_version: Version,
    /// Non-empty list of transactions in a block.
    pub transactions: Vec<Transaction>,
    /// Persisted auxiliary info for each transaction, aligned with `transactions`.
    #[serde(default = "Vec::new")]
    pub persisted_auxiliary_infos: Vec<PersistedAuxiliaryInfo>,
}

/// The complete read-set a block touched, matching `aptos_replay_benchmark::state_view::ReadSet`.
/// Keyed by [`StateKey`] (modules, resources, and resource groups all live here).
#[derive(Clone, Serialize, Deserialize)]
pub struct ReadSet {
    pub data: HashMap<StateKey, StateValue>,
}

impl TStateView for ReadSet {
    type Key = StateKey;

    fn next_version(&self) -> Version {
        0
    }

    fn get_state_slot(&self, state_key: &Self::Key) -> StateViewResult<StateSlot> {
        let slot = match self.data.get(state_key) {
            Some(state_value) => StateSlot::new(state_key.clone(), StateSlotKind::ColdOccupied {
                value_version: 0,
                value: state_value.clone(),
            }),
            None => StateSlot::new(state_key.clone(), StateSlotKind::ColdVacant),
        };
        Ok(slot)
    }

    fn get_usage(&self) -> StateViewResult<StateStorageUsage> {
        Ok(StateStorageUsage::new_untracked())
    }
}

/// A single Move resource, addressed by `(account, struct tag)`, with its BCS-encoded value.
pub struct StoredResource {
    pub address: AccountAddress,
    pub struct_tag: StructTag,
    pub blob: Vec<u8>,
}

impl ReadSet {
    /// Every module's `(id, bytecode)` present in the read-set.
    pub fn modules(&self) -> Vec<(ModuleId, Vec<u8>)> {
        let mut modules = vec![];
        for (key, value) in &self.data {
            if let Some(Path::Code(module_id)) = access_path_of(key) {
                modules.push((module_id, value.bytes().to_vec()));
            }
        }
        modules
    }

    /// Every individual resource present in the read-set. Resource *groups* are unpacked into
    /// their member resources, since the bare Move VMs read resources one type at a time and have
    /// no notion of resource groups.
    pub fn resources(&self) -> anyhow::Result<Vec<StoredResource>> {
        let mut resources = vec![];
        for (key, value) in &self.data {
            let (address, path) = match (address_of(key), access_path_of(key)) {
                (Some(address), Some(path)) => (address, path),
                _ => continue,
            };
            match path {
                Path::Code(_) => {},
                Path::Resource(struct_tag) => resources.push(StoredResource {
                    address,
                    struct_tag,
                    blob: value.bytes().to_vec(),
                }),
                Path::ResourceGroup(group_tag) => {
                    // A resource group is stored as a BCS map from member struct tag to the
                    // member's BCS-encoded value.
                    let members: BTreeMap<StructTag, Vec<u8>> = bcs::from_bytes(value.bytes())
                        .with_context(|| {
                            format!(
                                "Failed to decode resource group {} at {}",
                                group_tag.to_canonical_string(),
                                address
                            )
                        })?;
                    for (struct_tag, blob) in members {
                        resources.push(StoredResource {
                            address,
                            struct_tag,
                            blob,
                        });
                    }
                },
            }
        }
        Ok(resources)
    }
}

/// Everything needed to benchmark a single entry-function transaction on both VMs.
pub struct BenchmarkInput {
    /// On-chain version of the transaction (for reporting).
    pub version: Version,
    /// The transaction sender; used to materialize the leading `&signer` argument(s).
    pub sender: AccountAddress,
    /// The entry function and its (BCS-encoded, non-signer) arguments.
    pub entry: EntryFunction,
    /// The transaction's user context (sender, gas, fee payer, secondary signers, entry payload,
    /// index), reconstructed from the transaction itself. Fed to the `transaction_context` native
    /// so the entry function sees the same context it did on chain (this matters: some functions
    /// abort early without it).
    pub user_context: UserTransactionContext,
    /// The transaction's chain id.
    pub chain_id: u8,
    /// The complete read-set the transaction's block touched. Shared across the block's
    /// transactions; a superset of any single transaction's reads, which is fine for replay.
    pub read_set: Arc<ReadSet>,
}

/// Loads the transaction blocks written by `download`.
pub fn load_transaction_blocks(path: impl AsRef<FsPath>) -> anyhow::Result<Vec<TransactionBlock>> {
    let bytes = std::fs::read(path.as_ref())
        .with_context(|| format!("Failed to read transactions file {:?}", path.as_ref()))?;
    bcs::from_bytes(&bytes).context("Failed to decode transaction blocks")
}

/// Loads the read-sets written by `initialize`. Index-aligned with the transaction blocks.
pub fn load_read_sets(path: impl AsRef<FsPath>) -> anyhow::Result<Vec<ReadSet>> {
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
        let read_set = Arc::new(read_set);
        let mut version = block.begin_version;
        for (i, txn) in block.transactions.iter().enumerate() {
            let aux_info = block.persisted_auxiliary_infos.get(i);
            if let Some((sender, entry, user_context, chain_id)) =
                parse_user_transaction(txn, aux_info)
            {
                inputs.push(BenchmarkInput {
                    version,
                    sender,
                    entry,
                    user_context,
                    chain_id,
                    read_set: Arc::clone(&read_set),
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

/// If `txn` is an entry-function user transaction, returns its sender, entry function, the
/// reconstructed [`UserTransactionContext`], and chain id. Everything is derived from the
/// transaction itself — no chain access required.
fn parse_user_transaction(
    txn: &Transaction,
    aux_info: Option<&PersistedAuxiliaryInfo>,
) -> Option<(AccountAddress, EntryFunction, UserTransactionContext, u8)> {
    let signed = match txn {
        Transaction::UserTransaction(signed) => signed,
        _ => return None,
    };
    let entry = match signed.executable_ref() {
        Ok(TransactionExecutableRef::EntryFunction(entry)) => entry.clone(),
        _ => return None,
    };

    let sender = signed.sender();
    let chain_id = signed.chain_id().id();
    let authenticator = signed.authenticator();
    let secondary_signers = authenticator.secondary_signer_addresses();
    let gas_payer = authenticator.fee_payer_address().unwrap_or(sender);
    let transaction_index_kind = match aux_info {
        Some(PersistedAuxiliaryInfo::V1 { transaction_index }) => {
            TransactionIndexKind::BlockExecution {
                transaction_index: *transaction_index,
            }
        },
        Some(PersistedAuxiliaryInfo::TimestampNotYetAssignedV1 { transaction_index }) => {
            TransactionIndexKind::ValidationOrSimulation {
                transaction_index: *transaction_index,
            }
        },
        Some(PersistedAuxiliaryInfo::None) | None => TransactionIndexKind::NotAvailable,
    };

    let user_context = UserTransactionContext::new(
        sender,
        secondary_signers,
        gas_payer,
        signed.max_gas_amount(),
        signed.gas_unit_price(),
        chain_id,
        Some(entry.as_entry_function_payload()),
        None,
        transaction_index_kind,
        false,
    );
    Some((sender, entry, user_context, chain_id))
}

/// The account address of a `StateKey`, when it is an access-path key.
fn address_of(key: &StateKey) -> Option<AccountAddress> {
    match key.inner() {
        StateKeyInner::AccessPath(ap) => Some(ap.address),
        _ => None,
    }
}

/// The structured `Path` (module / resource / resource group) of a `StateKey`, when it is an
/// access-path key.
fn access_path_of(key: &StateKey) -> Option<Path> {
    match key.inner() {
        StateKeyInner::AccessPath(ap) => Some(ap.get_path()),
        _ => None,
    }
}
