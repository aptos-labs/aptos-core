use crate::{error_if_too_many_requested, AccountAddress};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_storage_interface::{DbReader, DbWriter, ExecutedTrees, MAX_REQUEST_LIMIT};
use aptos_types::{
    access_path::AccessPath,
    account_config::{aptos_test_root_address, AccountResource, NewBlockEvent},
    contract_event::EventWithVersion,
    epoch_state::EpochState,
    event::{EventHandle, EventKey},
    ledger_info::LedgerInfoWithSignatures,
    proof::{
        AccumulatorConsistencyProof, SparseMerkleProofExt, TransactionAccumulatorProof,
        TransactionAccumulatorRangeProof, TransactionAccumulatorSummary,
        TransactionInfoListWithProof, TransactionInfoWithProof,
    },
    state_proof::StateProof,
    state_store::{
        state_key::StateKey,
        state_key_prefix::StateKeyPrefix,
        state_storage_usage::StateStorageUsage,
        state_value::{StateValue, StateValueChunkWithProof},
        table,
    },
    transaction::{
        Transaction, TransactionInfo, TransactionListWithProof, TransactionOutput,
        TransactionOutputListWithProof, TransactionToCommit, TransactionWithProof, Version,
    },
    write_set::WriteSet,
};
use move_core_types::move_resource::MoveStructType;

use anyhow::{format_err, Result};
use dashmap::DashMap;
use itertools::zip_eq;
use std::sync::Arc;

use crate::{
    gauged_api,
    metrics::{LEDGER_VERSION, NEXT_BLOCK_EPOCH},
    AptosDB,
};

pub struct FakeAptosDB {
    inner: AptosDB,
    txn_version_by_hash: Arc<DashMap<HashValue, Version>>,
    txn_by_version: Arc<DashMap<Version, Transaction>>,
    txn_info_by_version: Arc<DashMap<Version, TransactionInfo>>,
    account_seq_num: Arc<DashMap<AccountAddress, u64>>,
    ledger_commit_lock: std::sync::Mutex<()>,
}

impl FakeAptosDB {
    pub fn new(db: AptosDB) -> Self {
        Self {
            inner: db,
            txn_by_version: Arc::new(DashMap::new()),
            txn_version_by_hash: Arc::new(DashMap::new()),
            txn_info_by_version: Arc::new(DashMap::new()),
            account_seq_num: Arc::new(DashMap::new()),
            ledger_commit_lock: std::sync::Mutex::new(()),
        }
    }
}

impl DbWriter for FakeAptosDB {
    fn save_transactions(
        &self,
        txns_to_commit: &[TransactionToCommit],
        first_version: Version,
        base_state_version: Option<Version>,
        ledger_info_with_sigs: Option<&LedgerInfoWithSignatures>,
        sync_commit: bool,
        latest_in_memory_state: aptos_storage_interface::state_delta::StateDelta,
    ) -> Result<()> {
        gauged_api("save_transactions", || {
            // Executing and committing from more than one threads not allowed -- consensus and
            // state sync must hand over to each other after all pending execution and committing
            // complete.
            let _lock = self
                .ledger_commit_lock
                .try_lock()
                .expect("Concurrent committing detected.");

            if first_version == 0 {
                self.inner.save_transactions(
                    txns_to_commit,
                    first_version,
                    base_state_version,
                    ledger_info_with_sigs,
                    sync_commit,
                    latest_in_memory_state,
                )?;

                // let last_version = first_version + txns_to_commit.len() as u64 - 1;

                // zip_eq(first_version..=last_version, txns_to_commit).for_each(
                //     |(_, txn_to_commit)| {
                //         if let Transaction::GenesisTransaction(write_set) =
                //             txn_to_commit.transaction()
                //         {
                //             if let aptos_types::transaction::WriteSetPayload::Direct(change_set) =
                //                 write_set
                //             {
                //                 change_set.write_set().iter().for_each(|(key, _)| {
                //                     if let StateKey::AccessPath(path) = key {
                //                         println!("{{ key: {} }}, ", path);
                //                     }
                //                 });
                //                 println!("==============");
                //             }
                //         }
                //     },
                // );
            }

            // print!("save_transactions usr : ");

            let last_version = first_version + txns_to_commit.len() as u64 - 1;

            zip_eq(first_version..=last_version, txns_to_commit).try_for_each(
                |(ver, txn_to_commit)| -> Result<(), anyhow::Error> {
                    // let hash = txn_to_commit.transaction().hash();

                    self.txn_by_version
                        .insert(ver, txn_to_commit.transaction().clone());
                    self.txn_info_by_version
                        .insert(ver, txn_to_commit.transaction_info().clone());
                    self.txn_version_by_hash
                        .insert(txn_to_commit.transaction().hash(), ver);

                    if let Ok(user_txn) = txn_to_commit.transaction().as_signed_user_txn() {
                        self.account_seq_num
                            .entry(user_txn.sender())
                            .and_modify(|seq_num| {
                                *seq_num = std::cmp::max(user_txn.sequence_number() + 1, *seq_num);
                            })
                            .or_insert(user_txn.sequence_number());
                    }
                    Ok::<(), anyhow::Error>(())
                },
            )?;

            // println!("");

            // Once everything is successfully stored, update the latest in-memory ledger info.
            if let Some(x) = ledger_info_with_sigs {
                self.inner.ledger_store.set_latest_ledger_info(x.clone());

                LEDGER_VERSION.set(x.ledger_info().version() as i64);
                NEXT_BLOCK_EPOCH.set(x.ledger_info().next_block_epoch() as i64);
            }
            Ok(())
        })
    }

    fn get_state_snapshot_receiver(
        &self,
        version: Version,
        expected_root_hash: HashValue,
    ) -> Result<Box<dyn aptos_storage_interface::StateSnapshotReceiver<StateKey, StateValue>>> {
        self.inner
            .get_state_snapshot_receiver(version, expected_root_hash)
    }

    fn finalize_state_snapshot(
        &self,
        version: Version,
        output_with_proof: TransactionOutputListWithProof,
        ledger_infos: &[LedgerInfoWithSignatures],
    ) -> Result<()> {
        self.inner
            .finalize_state_snapshot(version, output_with_proof, ledger_infos)
    }
}

impl DbReader for FakeAptosDB {
    fn get_epoch_ending_ledger_infos(
        &self,
        start_epoch: u64,
        end_epoch: u64,
    ) -> Result<aptos_types::epoch_change::EpochChangeProof> {
        (&self.inner as &dyn DbReader).get_epoch_ending_ledger_infos(start_epoch, end_epoch)
    }

    fn get_transactions(
        &self,
        start_version: Version,
        batch_size: u64,
        ledger_version: Version,
        fetch_events: bool,
    ) -> Result<TransactionListWithProof> {
        self.inner
            .get_transactions(start_version, batch_size, ledger_version, fetch_events)
    }

    fn get_gas_prices(
        &self,
        start_version: Version,
        limit: u64,
        ledger_version: Version,
    ) -> Result<Vec<u64>> {
        self.inner
            .get_gas_prices(start_version, limit, ledger_version)
    }

    fn get_transaction_by_hash(
        &self,
        hash: HashValue,
        _ledger_version: Version,
        _fetch_events: bool,
    ) -> Result<Option<TransactionWithProof>> {
        // println!("get_transaction_by_hash {hash:#x}");
        // println!(
        //     "txn_version_by_hash({hash:#x}): {:?}",
        //     self.txn_version_by_hash.contains_key(&hash)
        // );
        self.txn_version_by_hash
            .get(&hash)
            .as_deref()
            .map(|version| {
                let txn_info = self
                    .txn_info_by_version
                    .get(version)
                    .ok_or_else(|| format_err!("No transaction info at version {}", version,))?
                    .clone();
                let txn = self
                    .txn_by_version
                    .get(version)
                    .ok_or_else(|| format_err!("No transaction at version {}", version))?
                    .clone();

                let txn_info_with_proof = TransactionInfoWithProof::new(
                    TransactionAccumulatorProof::new(vec![]),
                    txn_info,
                );

                Ok(TransactionWithProof::new(
                    version.clone(),
                    txn,
                    None,
                    txn_info_with_proof,
                ))
            })
            .transpose()
    }

    fn get_transaction_by_version(
        &self,
        version: Version,
        ledger_version: Version,
        fetch_events: bool,
    ) -> Result<TransactionWithProof> {
        self.inner
            .get_transaction_by_version(version, ledger_version, fetch_events)
    }

    fn get_first_txn_version(&self) -> Result<Option<Version>> {
        self.inner.get_first_txn_version()
    }

    fn get_first_viable_txn_version(&self) -> Result<Version> {
        self.inner.get_first_viable_txn_version()
    }

    fn get_first_write_set_version(&self) -> Result<Option<Version>> {
        self.inner.get_first_write_set_version()
    }

    fn get_transaction_outputs(
        &self,
        start_version: Version,
        limit: u64,
        ledger_version: Version,
    ) -> Result<TransactionOutputListWithProof> {
        gauged_api("get_transactions_outputs", || {
            error_if_too_many_requested(limit, MAX_REQUEST_LIMIT)?;

            if start_version > ledger_version || limit == 0 {
                return Ok(TransactionOutputListWithProof::new_empty());
            }

            let limit = std::cmp::min(limit, ledger_version - start_version + 1);

            let (txn_infos, txns_and_outputs) = (start_version..start_version + limit)
                .map(|version| {
                    let txn_info = self
                        .txn_info_by_version
                        .get(&version)
                        .ok_or_else(|| format_err!("No transaction info at version {}", version,))?
                        .clone();
                    let events = vec![];
                    let write_set = WriteSet::default();
                    let txn = self
                        .txn_by_version
                        .get(&version)
                        .ok_or_else(|| format_err!("No transaction at version {}", version,))?
                        .clone();
                    let txn_output = TransactionOutput::new(
                        write_set,
                        events,
                        txn_info.gas_used(),
                        txn_info.status().clone().into(),
                    );
                    Ok((txn_info, (txn, txn_output)))
                })
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .unzip();
            let proof = TransactionInfoListWithProof::new(
                TransactionAccumulatorRangeProof::new_empty(),
                txn_infos,
            );

            Ok(TransactionOutputListWithProof::new(
                txns_and_outputs,
                Some(start_version),
                proof,
            ))
        })
    }

    fn get_events(
        &self,
        event_key: &aptos_types::event::EventKey,
        start: u64,
        order: aptos_storage_interface::Order,
        limit: u64,
        ledger_version: Version,
    ) -> Result<Vec<EventWithVersion>> {
        self.inner
            .get_events(event_key, start, order, limit, ledger_version)
    }

    fn get_block_timestamp(&self, version: Version) -> Result<u64> {
        self.inner.get_block_timestamp(version)
    }

    fn get_next_block_event(&self, version: Version) -> Result<(Version, NewBlockEvent)> {
        self.inner.get_next_block_event(version)
    }

    fn get_block_info_by_version(
        &self,
        version: Version,
    ) -> Result<(Version, Version, NewBlockEvent)> {
        self.inner.get_block_info_by_version(version)
    }

    fn get_block_info_by_height(&self, height: u64) -> Result<(Version, Version, NewBlockEvent)> {
        self.inner.get_block_info_by_height(height)
    }

    fn get_last_version_before_timestamp(
        &self,
        timestamp: u64,
        ledger_version: Version,
    ) -> Result<Version> {
        self.inner
            .get_last_version_before_timestamp(timestamp, ledger_version)
    }

    fn get_latest_epoch_state(&self) -> Result<EpochState> {
        self.inner.get_latest_epoch_state()
    }

    fn get_prefixed_state_value_iterator(
        &self,
        key_prefix: &StateKeyPrefix,
        cursor: Option<&StateKey>,
        version: Version,
    ) -> Result<Box<dyn Iterator<Item = anyhow::Result<(StateKey, StateValue)>> + '_>> {
        self.inner
            .get_prefixed_state_value_iterator(key_prefix, cursor, version)
    }

    fn get_latest_ledger_info_option(&self) -> Result<Option<LedgerInfoWithSignatures>> {
        self.inner.get_latest_ledger_info_option()
    }

    fn get_latest_state_checkpoint_version(&self) -> Result<Option<Version>> {
        self.inner.get_latest_state_checkpoint_version()
    }

    fn get_state_snapshot_before(
        &self,
        next_version: Version,
    ) -> Result<Option<(Version, HashValue)>> {
        self.inner.get_state_snapshot_before(next_version)
    }

    fn get_account_transaction(
        &self,
        address: aptos_types::PeerId,
        seq_num: u64,
        include_events: bool,
        ledger_version: Version,
    ) -> Result<Option<TransactionWithProof>> {
        self.inner
            .get_account_transaction(address, seq_num, include_events, ledger_version)
    }

    fn get_account_transactions(
        &self,
        address: aptos_types::PeerId,
        seq_num: u64,
        limit: u64,
        include_events: bool,
        ledger_version: Version,
    ) -> Result<aptos_types::transaction::AccountTransactionsWithProof> {
        self.inner
            .get_account_transactions(address, seq_num, limit, include_events, ledger_version)
    }

    fn get_state_proof_with_ledger_info(
        &self,
        known_version: u64,
        ledger_info: LedgerInfoWithSignatures,
    ) -> Result<StateProof> {
        self.inner
            .get_state_proof_with_ledger_info(known_version, ledger_info)
    }

    fn get_state_proof(&self, known_version: u64) -> Result<StateProof> {
        self.inner.get_state_proof(known_version)
    }

    fn get_state_value_by_version(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<Option<StateValue>> {
        let access_path = AccessPath::try_from(state_key.clone())?;
        let account_address = access_path.address;
        let struct_tag = access_path.get_struct_tag();

        if (account_address != aptos_test_root_address() && account_address != AccountAddress::ONE)
            && struct_tag.is_some()
            && struct_tag.unwrap() == AccountResource::struct_tag()
        {
            // println!("getting non-root account");
            let seq_num = match self.account_seq_num.get(&account_address).as_deref() {
                Some(seq_num) => *seq_num,
                None => {
                    self.account_seq_num.insert(account_address, 1);
                    1
                }
            };
            let account = AccountResource::new(
                seq_num,
                vec![],
                EventHandle::new(EventKey::new(0, account_address), 0),
                EventHandle::new(EventKey::new(1, account_address), 0),
            );
            let bytes = bcs::to_bytes(&account)?;
            Ok(Some(StateValue::new(bytes)))
        } else {
            // println!("getting root account from db");
            self.inner.get_state_value_by_version(state_key, version)
        }
    }

    fn get_state_proof_by_version_ext(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<SparseMerkleProofExt> {
        self.inner
            .get_state_proof_by_version_ext(state_key, version)
    }

    fn get_state_value_with_proof_by_version_ext(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<(Option<StateValue>, SparseMerkleProofExt)> {
        self.inner
            .get_state_value_with_proof_by_version_ext(state_key, version)
    }

    fn get_latest_executed_trees(&self) -> Result<ExecutedTrees> {
        self.inner.get_latest_executed_trees()
    }

    fn get_epoch_ending_ledger_info(&self, known_version: u64) -> Result<LedgerInfoWithSignatures> {
        self.inner.get_epoch_ending_ledger_info(known_version)
    }

    fn get_latest_transaction_info_option(
        &self,
    ) -> Result<Option<(Version, aptos_types::transaction::TransactionInfo)>> {
        self.inner.get_latest_transaction_info_option()
    }

    fn get_accumulator_root_hash(&self, _version: Version) -> Result<HashValue> {
        Ok(HashValue::zero())
    }

    fn get_accumulator_consistency_proof(
        &self,
        client_known_version: Option<Version>,
        ledger_version: Version,
    ) -> Result<AccumulatorConsistencyProof> {
        self.inner
            .get_accumulator_consistency_proof(client_known_version, ledger_version)
    }

    fn get_accumulator_summary(
        &self,
        ledger_version: Version,
    ) -> Result<TransactionAccumulatorSummary> {
        self.inner.get_accumulator_summary(ledger_version)
    }

    fn get_state_leaf_count(&self, version: Version) -> Result<usize> {
        self.inner.get_state_leaf_count(version)
    }

    fn get_state_value_chunk_with_proof(
        &self,
        version: Version,
        start_idx: usize,
        chunk_size: usize,
    ) -> Result<StateValueChunkWithProof> {
        self.inner
            .get_state_value_chunk_with_proof(version, start_idx, chunk_size)
    }

    fn is_state_pruner_enabled(&self) -> Result<bool> {
        self.inner.is_state_pruner_enabled()
    }

    fn get_epoch_snapshot_prune_window(&self) -> Result<usize> {
        self.inner.get_epoch_snapshot_prune_window()
    }

    fn is_ledger_pruner_enabled(&self) -> Result<bool> {
        self.inner.is_ledger_pruner_enabled()
    }

    fn get_ledger_prune_window(&self) -> Result<usize> {
        self.inner.get_ledger_prune_window()
    }

    fn get_table_info(&self, handle: table::TableHandle) -> Result<table::TableInfo> {
        self.inner.get_table_info(handle)
    }

    fn indexer_enabled(&self) -> bool {
        self.inner.indexer_enabled()
    }

    fn get_state_storage_usage(&self, version: Option<Version>) -> Result<StateStorageUsage> {
        self.inner.get_state_storage_usage(version)
    }
}
