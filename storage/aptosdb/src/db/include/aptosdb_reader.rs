// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_storage_interface::state_store::{
    state::State, state_summary::StateSummary, state_view::hot_state_view::HotStateView,
};
use aptos_types::{block_info::BlockHeight, transaction::IndexedTransactionSummary};

impl DbReader for AptosDB {
    fn get_persisted_state(&self) -> Result<(Arc<dyn HotStateView>, State)> {
        gauged_api("get_persisted_state", || {
            self.state_store.get_persisted_state()
        })
    }

    fn get_persisted_state_summary(&self) -> Result<StateSummary> {
        gauged_api("get_persisted_state_summary", || {
            self.state_store.get_persisted_state_summary()
        })
    }

    fn get_epoch_ending_ledger_infos(
        &self,
        start_epoch: u64,
        end_epoch: u64,
    ) -> Result<EpochChangeProof> {
        gauged_api("get_epoch_ending_ledger_infos", || {
            let (ledger_info_with_sigs, more) =
                Self::get_epoch_ending_ledger_infos(self, start_epoch, end_epoch)?;
            Ok(EpochChangeProof::new(ledger_info_with_sigs, more))
        })
    }

    fn get_prefixed_state_value_iterator(
        &self,
        key_prefix: &StateKeyPrefix,
        cursor: Option<&StateKey>,
        version: Version,
    ) -> Result<Box<dyn Iterator<Item = Result<(StateKey, StateValue)>> + '_>> {
        gauged_api("get_prefixed_state_value_iterator", || {
            ensure!(
                !self.state_kv_db.enabled_sharding(),
                "This API is not supported with sharded DB"
            );
            self.error_if_state_kv_pruned("StateValue", version)?;

            Ok(Box::new(
                self.state_store
                    .get_prefixed_state_value_iterator(key_prefix, cursor, version)?,
            )
                as Box<dyn Iterator<Item = Result<(StateKey, StateValue)>>>)
        })
    }

    fn get_transaction_auxiliary_data_by_version(
        &self,
        version: Version,
    ) -> Result<Option<TransactionAuxiliaryData>> {
        gauged_api("get_transaction_auxiliary_data_by_version", || {
            self.error_if_ledger_pruned("Transaction", version)?;
            self.ledger_db
                .transaction_auxiliary_data_db()
                .get_transaction_auxiliary_data(version)
        })
    }

    fn get_latest_ledger_info_option(&self) -> Result<Option<LedgerInfoWithSignatures>> {
        gauged_api("get_latest_ledger_info_option", || {
            Ok(self.ledger_db.metadata_db().get_latest_ledger_info_option())
        })
    }

    fn get_synced_version(&self) -> Result<Option<Version>> {
        gauged_api("get_synced_version", || {
            self.ledger_db.metadata_db().get_synced_version()
        })
    }

    fn get_pre_committed_version(&self) -> Result<Option<Version>> {
        gauged_api("get_pre_committed_version", || {
            Ok(self.state_store.current_state_locked().version())
        })
    }

    fn get_account_ordered_transaction(
        &self,
        address: AccountAddress,
        seq_num: u64,
        include_events: bool,
        ledger_version: Version,
    ) -> Result<Option<TransactionWithProof>> {
        gauged_api("get_account_transaction", || {
            ensure!(
                !self.state_kv_db.enabled_sharding(),
                "This API is not supported with sharded DB"
            );
            self.transaction_store
                .get_account_ordered_transaction_version(address, seq_num, ledger_version)?
                .map(|txn_version| {
                    self.get_transaction_with_proof(txn_version, ledger_version, include_events)
                })
                .transpose()
        })
    }

    fn get_account_ordered_transactions(
        &self,
        address: AccountAddress,
        start_seq_num: u64,
        limit: u64,
        include_events: bool,
        ledger_version: Version,
    ) -> Result<AccountOrderedTransactionsWithProof> {
        gauged_api("get_account_ordered_transactions", || {
            ensure!(
                !self.state_kv_db.enabled_sharding(),
                "This API is not supported with sharded DB"
            );
            error_if_too_many_requested(limit, MAX_REQUEST_LIMIT)?;

            let txns_with_proofs = self
                .transaction_store
                .get_account_ordered_transactions_iter(
                    address,
                    start_seq_num,
                    limit,
                    ledger_version,
                )?
                .map(|result| {
                    let (_seq_num, txn_version) = result?;
                    self.get_transaction_with_proof(txn_version, ledger_version, include_events)
                })
                .collect::<Result<Vec<_>>>()?;

            Ok(AccountOrderedTransactionsWithProof::new(txns_with_proofs))
        })
    }

    fn get_account_transaction_summaries(
        &self,
        address: AccountAddress,
        start_version: Option<u64>,
        end_version: Option<u64>,
        limit: u64,
        ledger_version: Version,
    ) -> Result<Vec<IndexedTransactionSummary>> {
        gauged_api("get_account_transaction_summaries", || {
            error_if_too_many_requested(limit, MAX_REQUEST_LIMIT)?;

            let txn_summaries_iter = self
                .transaction_store
                .get_account_transaction_summaries_iter(
                    address,
                    start_version,
                    end_version,
                    limit,
                    ledger_version,
                )?
                .map(|result| {
                    let (_version, txn_summary) = result?;
                    Ok(txn_summary)
                });

            if start_version.is_some() {
                txn_summaries_iter.collect::<Result<Vec<_>>>()
            } else {
                let txn_summaries = txn_summaries_iter.collect::<Result<Vec<_>>>()?;
                Ok(txn_summaries.into_iter().rev().collect::<Vec<_>>())
            }
        })
    }

    /// This API is best-effort in that it CANNOT provide absence proof.
    fn get_transaction_by_hash(
        &self,
        hash: HashValue,
        ledger_version: Version,
        fetch_events: bool,
    ) -> Result<Option<TransactionWithProof>> {
        gauged_api("get_transaction_by_hash", || {
            self.ledger_db
                .transaction_db()
                .get_transaction_version_by_hash(&hash, ledger_version)?
                .map(|v| self.get_transaction_with_proof(v, ledger_version, fetch_events))
                .transpose()
        })
    }

    /// Returns the transaction by version, delegates to `AptosDB::get_transaction_with_proof`.
    /// Returns an error if the provided version is not found.
    fn get_transaction_by_version(
        &self,
        version: Version,
        ledger_version: Version,
        fetch_events: bool,
    ) -> Result<TransactionWithProof> {
        gauged_api("get_transaction_by_version", || {
            self.get_transaction_with_proof(version, ledger_version, fetch_events)
        })
    }

    // ======================= State Synchronizer Internal APIs ===================================
    /// Returns batch of transactions for the purpose of synchronizing state to another node.
    ///
    /// If any version beyond ledger_version is requested, it is ignored.
    /// Returns an error if any version <= ledger_version is requested but not found.
    ///
    /// This is used by the State Synchronizer module internally.
    fn get_transactions(
        &self,
        start_version: Version,
        limit: u64,
        ledger_version: Version,
        fetch_events: bool,
    ) -> Result<TransactionListWithProof> {
        gauged_api("get_transactions", || {
            error_if_too_many_requested(limit, MAX_REQUEST_LIMIT)?;

            if start_version > ledger_version || limit == 0 {
                return Ok(TransactionListWithProof::new_empty());
            }
            self.error_if_ledger_pruned("Transaction", start_version)?;

            let limit = std::cmp::min(limit, ledger_version - start_version + 1);

            let txns = (start_version..start_version + limit)
                .map(|version| self.ledger_db.transaction_db().get_transaction(version))
                .collect::<Result<Vec<_>>>()?;
            let txn_infos = (start_version..start_version + limit)
                .map(|version| {
                    self.ledger_db
                        .transaction_info_db()
                        .get_transaction_info(version)
                })
                .collect::<Result<Vec<_>>>()?;
            let events = if fetch_events {
                Some(
                    (start_version..start_version + limit)
                        .map(|version| self.ledger_db.event_db().get_events_by_version(version))
                        .collect::<Result<Vec<_>>>()?,
                )
            } else {
                None
            };
            let proof = TransactionInfoListWithProof::new(
                self.ledger_db
                    .transaction_accumulator_db()
                    .get_transaction_range_proof(Some(start_version), limit, ledger_version)?,
                txn_infos,
            );

            Ok(TransactionListWithProof::new(
                txns,
                events,
                Some(start_version),
                proof,
            ))
        })
    }

    /// Get the first version that txn starts existent.
    fn get_first_txn_version(&self) -> Result<Option<Version>> {
        gauged_api("get_first_txn_version", || {
            Ok(Some(self.ledger_pruner.get_min_readable_version()))
        })
    }

    /// Get the first block version / height that will likely not be pruned soon.
    fn get_first_viable_block(&self) -> Result<(Version, BlockHeight)> {
        gauged_api("get_first_viable_block", || {
            let min_version = self.ledger_pruner.get_min_viable_version();
            if !self.skip_index_and_usage {
                let (block_version, index, _seq_num) = self
                    .event_store
                    .lookup_event_at_or_after_version(&new_block_event_key(), min_version)?
                    .ok_or_else(|| {
                        AptosDbError::NotFound(format!(
                            "NewBlockEvent at or after version {}",
                            min_version
                        ))
                    })?;
                let event = self
                    .event_store
                    .get_event_by_version_and_index(block_version, index)?;
                return Ok((block_version, event.expect_new_block_event()?.height()));
            }

            self.ledger_db
                .metadata_db()
                .get_block_height_at_or_after_version(min_version)
        })
    }

    /// Get the first version that write set starts existent.
    fn get_first_write_set_version(&self) -> Result<Option<Version>> {
        gauged_api("get_first_write_set_version", || {
            Ok(Some(self.ledger_pruner.get_min_readable_version()))
        })
    }

    /// Returns a batch of transactions for the purpose of synchronizing state to another node.
    ///
    /// If any version beyond ledger_version is requested, it is ignored.
    /// Returns an error if any version <= ledger_version is requested but not found.
    ///
    /// This is used by the State Synchronizer module internally.
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

            self.error_if_ledger_pruned("Transaction", start_version)?;

            let limit = std::cmp::min(limit, ledger_version - start_version + 1);

            let (txn_infos, txns_and_outputs) = (start_version..start_version + limit)
                .map(|version| {
                    let txn_info = self
                        .ledger_db
                        .transaction_info_db()
                        .get_transaction_info(version)?;
                    let events = self.ledger_db.event_db().get_events_by_version(version)?;
                    let write_set = self.ledger_db.write_set_db().get_write_set(version)?;
                    let txn = self.ledger_db.transaction_db().get_transaction(version)?;
                    let auxiliary_data = self
                        .ledger_db
                        .transaction_auxiliary_data_db()
                        .get_transaction_auxiliary_data(version)?
                        .unwrap_or_default();
                    let txn_output = TransactionOutput::new(
                        write_set,
                        events,
                        txn_info.gas_used(),
                        txn_info.status().clone().into(),
                        auxiliary_data,
                    );
                    Ok((txn_info, (txn, txn_output)))
                })
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .unzip();
            let proof = TransactionInfoListWithProof::new(
                self.ledger_db
                    .transaction_accumulator_db()
                    .get_transaction_range_proof(Some(start_version), limit, ledger_version)?,
                txn_infos,
            );

            Ok(TransactionOutputListWithProof::new(
                txns_and_outputs,
                Some(start_version),
                proof,
            ))
        })
    }

    /// TODO(bowu): Deprecate after internal index migration
    fn get_events(
        &self,
        event_key: &EventKey,
        start: u64,
        order: Order,
        limit: u64,
        ledger_version: Version,
    ) -> Result<Vec<EventWithVersion>> {
        gauged_api("get_events", || {
            self.get_events_by_event_key(event_key, start, order, limit, ledger_version)
        })
    }

    fn get_transaction_iterator(
        &self,
        start_version: Version,
        limit: u64,
    ) -> Result<Box<dyn Iterator<Item = Result<Transaction>> + '_>> {
        gauged_api("get_transaction_iterator", || {
            error_if_too_many_requested(limit, MAX_REQUEST_LIMIT)?;
            self.error_if_ledger_pruned("Transaction", start_version)?;

            let iter = self
                .ledger_db
                .transaction_db()
                .get_transaction_iter(start_version, limit as usize)?;
            Ok(Box::new(iter) as Box<dyn Iterator<Item = Result<Transaction>> + '_>)
        })
    }

    fn get_transaction_info_iterator(
        &self,
        start_version: Version,
        limit: u64,
    ) -> Result<Box<dyn Iterator<Item = Result<TransactionInfo>> + '_>> {
        gauged_api("get_transaction_info_iterator", || {
            error_if_too_many_requested(limit, MAX_REQUEST_LIMIT)?;
            self.error_if_ledger_pruned("Transaction", start_version)?;

            let iter = self
                .ledger_db
                .transaction_info_db()
                .get_transaction_info_iter(start_version, limit as usize)?;
            Ok(Box::new(iter) as Box<dyn Iterator<Item = Result<TransactionInfo>> + '_>)
        })
    }

    fn get_events_iterator(
        &self,
        start_version: Version,
        limit: u64,
    ) -> Result<Box<dyn Iterator<Item = Result<Vec<ContractEvent>>> + '_>> {
        gauged_api("get_events_iterator", || {
            error_if_too_many_requested(limit, MAX_REQUEST_LIMIT)?;
            self.error_if_ledger_pruned("Transaction", start_version)?;

            let iter = self
                .ledger_db
                .event_db()
                .get_events_by_version_iter(start_version, limit as usize)?;
            Ok(Box::new(iter)
                as Box<
                    dyn Iterator<Item = Result<Vec<ContractEvent>>> + '_,
                >)
        })
    }

    fn get_write_set_iterator(
        &self,
        start_version: Version,
        limit: u64,
    ) -> Result<Box<dyn Iterator<Item = Result<WriteSet>> + '_>> {
        gauged_api("get_write_set_iterator", || {
            error_if_too_many_requested(limit, MAX_REQUEST_LIMIT)?;
            self.error_if_ledger_pruned("Transaction", start_version)?;

            let iter = self
                .ledger_db
                .write_set_db()
                .get_write_set_iter(start_version, limit as usize)?;
            Ok(Box::new(iter) as Box<dyn Iterator<Item = Result<WriteSet>> + '_>)
        })
    }

    fn get_transaction_accumulator_range_proof(
        &self,
        first_version: Version,
        limit: u64,
        ledger_version: Version,
    ) -> Result<TransactionAccumulatorRangeProof> {
        gauged_api("get_transaction_accumulator_range_proof", || {
            self.error_if_ledger_pruned("Transaction", first_version)?;

            self.ledger_db
                .transaction_accumulator_db()
                .get_transaction_range_proof(Some(first_version), limit, ledger_version)
        })
    }

    /// Gets ledger info at specified version and ensures it's an epoch ending.
    fn get_epoch_ending_ledger_info(&self, version: u64) -> Result<LedgerInfoWithSignatures> {
        gauged_api("get_epoch_ending_ledger_info", || {
            self.ledger_db
                .metadata_db()
                .get_epoch_ending_ledger_info(version)
        })
    }

    fn get_state_proof_with_ledger_info(
        &self,
        known_version: u64,
        ledger_info_with_sigs: LedgerInfoWithSignatures,
    ) -> Result<StateProof> {
        gauged_api("get_state_proof_with_ledger_info", || {
            let ledger_info = ledger_info_with_sigs.ledger_info();
            ensure!(
                known_version <= ledger_info.version(),
                "Client known_version {} larger than ledger version {}.",
                known_version,
                ledger_info.version(),
            );
            let known_epoch = self.ledger_db.metadata_db().get_epoch(known_version)?;
            let end_epoch = ledger_info.next_block_epoch();
            let epoch_change_proof = if known_epoch < end_epoch {
                let (ledger_infos_with_sigs, more) =
                    self.get_epoch_ending_ledger_infos(known_epoch, end_epoch)?;
                EpochChangeProof::new(ledger_infos_with_sigs, more)
            } else {
                EpochChangeProof::new(vec![], /* more = */ false)
            };

            Ok(StateProof::new(ledger_info_with_sigs, epoch_change_proof))
        })
    }

    fn get_state_proof(&self, known_version: u64) -> Result<StateProof> {
        gauged_api("get_state_proof", || {
            let ledger_info_with_sigs = self.ledger_db.metadata_db().get_latest_ledger_info()?;
            self.get_state_proof_with_ledger_info(known_version, ledger_info_with_sigs)
        })
    }

    fn get_state_value_by_version(
        &self,
        state_store_key: &StateKey,
        version: Version,
    ) -> Result<Option<StateValue>> {
        gauged_api("get_state_value_by_version", || {
            self.error_if_state_kv_pruned("StateValue", version)?;

            self.state_store
                .get_state_value_by_version(state_store_key, version)
        })
    }

    fn get_state_value_with_version_by_version(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<Option<(Version, StateValue)>> {
        gauged_api("get_state_value_with_version_by_version", || {
            self.error_if_state_kv_pruned("StateValue", version)?;

            self.state_store
                .get_state_value_with_version_by_version(state_key, version)
        })
    }

    /// Returns the proof of the given state key and version.
    fn get_state_proof_by_version_ext(
        &self,
        key_hash: &HashValue,
        version: Version,
        root_depth: usize,
    ) -> Result<SparseMerkleProofExt> {
        gauged_api("get_state_proof_by_version_ext", || {
            self.error_if_state_merkle_pruned("State merkle", version)?;

            self.state_store
                .get_state_proof_by_version_ext(key_hash, version, root_depth)
        })
    }

    fn get_state_value_with_proof_by_version_ext(
        &self,
        key_hash: &HashValue,
        version: Version,
        root_depth: usize,
    ) -> Result<(Option<StateValue>, SparseMerkleProofExt)> {
        gauged_api("get_state_value_with_proof_by_version_ext", || {
            self.error_if_state_merkle_pruned("State merkle", version)?;

            self.state_store
                .get_state_value_with_proof_by_version_ext(key_hash, version, root_depth)
        })
    }

    fn get_latest_epoch_state(&self) -> Result<EpochState> {
        gauged_api("get_latest_epoch_state", || {
            let latest_ledger_info = self.ledger_db.metadata_db().get_latest_ledger_info()?;
            match latest_ledger_info.ledger_info().next_epoch_state() {
                Some(epoch_state) => Ok(epoch_state.clone()),
                None => self
                    .ledger_db
                    .metadata_db()
                    .get_epoch_state(latest_ledger_info.ledger_info().epoch()),
            }
        })
    }

    fn get_pre_committed_ledger_summary(&self) -> Result<LedgerSummary> {
        gauged_api("get_pre_committed_ledger_summary", || {
            let (state, state_summary) = self
                .state_store
                .current_state_locked()
                .to_state_and_summary();
            let num_txns = state.next_version();

            let frozen_subtrees = self
                .ledger_db
                .transaction_accumulator_db()
                .get_frozen_subtree_hashes(num_txns)?;
            let transaction_accumulator =
                Arc::new(InMemoryAccumulator::new(frozen_subtrees, num_txns)?);
            Ok(LedgerSummary {
                state,
                state_summary,
                transaction_accumulator,
            })
        })
    }

    fn get_block_timestamp(&self, version: u64) -> Result<u64> {
        gauged_api("get_block_timestamp", || {
            self.error_if_ledger_pruned("NewBlockEvent", version)?;
            let (_block_height, block_info) = self.get_raw_block_info_by_version(version)?;

            Ok(block_info.timestamp_usecs())
        })
    }

    // Returns latest `num_events` NewBlockEvents and their versions.
    // TODO(grao): Remove after DAG.
    fn get_latest_block_events(&self, num_events: usize) -> Result<Vec<EventWithVersion>> {
        gauged_api("get_latest_block_events", || {
            let latest_version = self.get_synced_version()?;
            if !self.skip_index_and_usage {
                return self.get_events(
                    &new_block_event_key(),
                    u64::MAX,
                    Order::Descending,
                    num_events as u64,
                    latest_version.unwrap_or(0),
                );
            }

            let db = self.ledger_db.metadata_db_arc();
            let mut iter = db.rev_iter::<BlockInfoSchema>()?;
            iter.seek_to_last();

            let mut events = Vec::with_capacity(num_events);
            for item in iter {
                let (_block_height, block_info) = item?;
                let first_version = block_info.first_version();
                if latest_version.as_ref().is_some_and(|v| first_version <= *v) {
                    let event = self
                        .ledger_db
                        .event_db()
                        .expect_new_block_event(first_version)?;
                    events.push(EventWithVersion::new(first_version, event));
                    if events.len() == num_events {
                        break;
                    }
                }
            }

            Ok(events)
        })
    }

    fn get_block_info_by_version(
        &self,
        version: Version,
    ) -> Result<(Version, Version, NewBlockEvent)> {
        gauged_api("get_block_info", || {
            self.error_if_ledger_pruned("NewBlockEvent", version)?;

            let (block_height, block_info) = self.get_raw_block_info_by_version(version)?;
            self.to_api_block_info(block_height, block_info)
        })
    }

    fn get_block_info_by_height(
        &self,
        block_height: u64,
    ) -> Result<(Version, Version, NewBlockEvent)> {
        gauged_api("get_block_info_by_height", || {
            let block_info = self.get_raw_block_info_by_height(block_height)?;
            self.to_api_block_info(block_height, block_info)
        })
    }

    fn get_last_version_before_timestamp(
        &self,
        timestamp: u64,
        ledger_version: Version,
    ) -> Result<Version> {
        gauged_api("get_last_version_before_timestamp", || {
            self.event_store
                .get_last_version_before_timestamp(timestamp, ledger_version)
        })
    }

    fn get_latest_state_checkpoint_version(&self) -> Result<Option<Version>> {
        gauged_api("get_latest_state_checkpoint_version", || {
            Ok(self
                .state_store
                .current_state_locked()
                .last_checkpoint()
                .version())
        })
    }

    fn get_state_snapshot_before(
        &self,
        next_version: Version,
    ) -> Result<Option<(Version, HashValue)>> {
        self.error_if_state_merkle_pruned("State merkle", next_version)?;
        gauged_api("get_state_snapshot_before", || {
            self.state_store.get_state_snapshot_before(next_version)
        })
    }

    fn get_accumulator_root_hash(&self, version: Version) -> Result<HashValue> {
        gauged_api("get_accumulator_root_hash", || {
            self.error_if_ledger_pruned("Transaction accumulator", version)?;
            self.ledger_db
                .transaction_accumulator_db()
                .get_root_hash(version)
        })
    }

    fn get_accumulator_consistency_proof(
        &self,
        client_known_version: Option<Version>,
        ledger_version: Version,
    ) -> Result<AccumulatorConsistencyProof> {
        gauged_api("get_accumulator_consistency_proof", || {
            self.error_if_ledger_pruned(
                "Transaction accumulator",
                client_known_version.unwrap_or(0),
            )?;
            self.ledger_db
                .transaction_accumulator_db()
                .get_consistency_proof(client_known_version, ledger_version)
        })
    }

    fn get_accumulator_summary(
        &self,
        ledger_version: Version,
    ) -> Result<TransactionAccumulatorSummary> {
        let num_txns = ledger_version + 1;
        let frozen_subtrees = self
            .ledger_db
            .transaction_accumulator_db()
            .get_frozen_subtree_hashes(num_txns)?;
        TransactionAccumulatorSummary::new(InMemoryAccumulator::new(frozen_subtrees, num_txns)?)
            .map_err(Into::into)
    }

    fn get_state_item_count(&self, version: Version) -> Result<usize> {
        gauged_api("get_state_item_count", || {
            self.error_if_state_merkle_pruned("State merkle", version)?;
            self.ledger_db
                .metadata_db()
                .get_usage(version)
                .map(|usage| usage.items())
        })
    }

    fn get_state_value_chunk_with_proof(
        &self,
        version: Version,
        first_index: usize,
        chunk_size: usize,
    ) -> Result<StateValueChunkWithProof> {
        gauged_api("get_state_value_chunk_with_proof", || {
            self.error_if_state_merkle_pruned("State merkle", version)?;
            self.state_store
                .get_value_chunk_with_proof(version, first_index, chunk_size)
        })
    }

    fn is_state_merkle_pruner_enabled(&self) -> Result<bool> {
        gauged_api("is_state_merkle_pruner_enabled", || {
            Ok(self
                .state_store
                .state_db
                .state_merkle_pruner
                .is_pruner_enabled())
        })
    }

    fn get_epoch_snapshot_prune_window(&self) -> Result<usize> {
        gauged_api("get_state_prune_window", || {
            Ok(self
                .state_store
                .state_db
                .epoch_snapshot_pruner
                .get_prune_window() as usize)
        })
    }

    fn is_ledger_pruner_enabled(&self) -> Result<bool> {
        gauged_api("is_ledger_pruner_enabled", || {
            Ok(self.ledger_pruner.is_pruner_enabled())
        })
    }

    fn get_ledger_prune_window(&self) -> Result<usize> {
        gauged_api("get_ledger_prune_window", || {
            Ok(self.ledger_pruner.get_prune_window() as usize)
        })
    }

    fn get_table_info(&self, handle: TableHandle) -> Result<TableInfo> {
        gauged_api("get_table_info", || {
            self.get_table_info_option(handle)?
                .ok_or_else(|| AptosDbError::NotFound(format!("TableInfo for {:?}", handle)))
        })
    }

    /// Returns whether the indexer DB has been enabled or not
    fn indexer_enabled(&self) -> bool {
        self.indexer.is_some()
    }

    fn get_state_storage_usage(&self, version: Option<Version>) -> Result<StateStorageUsage> {
        gauged_api("get_state_storage_usage", || {
            if let Some(v) = version {
                self.error_if_ledger_pruned("state storage usage", v)?;
            }
            self.state_store.get_usage(version)
        })
    }

    fn get_event_by_version_and_index(
        &self,
        version: Version,
        index: u64,
    ) -> Result<ContractEvent> {
        gauged_api("get_event_by_version_and_index", || {
            self.error_if_ledger_pruned("Event", version)?;
            self.event_store
                .get_event_by_version_and_index(version, index)
        })
    }
}

impl AptosDB {
    /// Returns ledger infos reflecting epoch bumps starting with the given epoch. If there are no
    /// more than `MAX_NUM_EPOCH_ENDING_LEDGER_INFO` results, this function returns all of them,
    /// otherwise the first `MAX_NUM_EPOCH_ENDING_LEDGER_INFO` results are returned and a flag
    /// (when true) will be used to indicate the fact that there is more.
    fn get_epoch_ending_ledger_infos(
        &self,
        start_epoch: u64,
        end_epoch: u64,
    ) -> Result<(Vec<LedgerInfoWithSignatures>, bool)> {
        self.get_epoch_ending_ledger_infos_impl(
            start_epoch,
            end_epoch,
            MAX_NUM_EPOCH_ENDING_LEDGER_INFO,
        )
    }

    fn get_epoch_ending_ledger_infos_impl(
        &self,
        start_epoch: u64,
        end_epoch: u64,
        limit: usize,
    ) -> Result<(Vec<LedgerInfoWithSignatures>, bool)> {
        ensure!(
            start_epoch <= end_epoch,
            "Bad epoch range [{}, {})",
            start_epoch,
            end_epoch,
        );
        // Note that the latest epoch can be the same with the current epoch (in most cases), or
        // current_epoch + 1 (when the latest ledger_info carries next validator set)

        let latest_epoch = self
            .ledger_db
            .metadata_db()
            .get_latest_ledger_info()?
            .ledger_info()
            .next_block_epoch();
        ensure!(
            end_epoch <= latest_epoch,
            "Unable to provide epoch change ledger info for still open epoch. asked upper bound: {}, last sealed epoch: {}",
            end_epoch,
            latest_epoch - 1,  // okay to -1 because genesis LedgerInfo has .next_block_epoch() == 1
        );

        let (paging_epoch, more) = if end_epoch - start_epoch > limit as u64 {
            (start_epoch + limit as u64, true)
        } else {
            (end_epoch, false)
        };

        let lis = self
            .ledger_db
            .metadata_db()
            .get_epoch_ending_ledger_info_iter(start_epoch, paging_epoch)?
            .collect::<Result<Vec<_>>>()?;

        ensure!(
            lis.len() == (paging_epoch - start_epoch) as usize,
            "DB corruption: missing epoch ending ledger info for epoch {}",
            lis.last()
                .map(|li| li.ledger_info().next_block_epoch() - 1)
                .unwrap_or(start_epoch),
        );
        Ok((lis, more))
    }

    /// Returns the transaction with proof for a given version, or error if the transaction is not
    /// found.
    fn get_transaction_with_proof(
        &self,
        version: Version,
        ledger_version: Version,
        fetch_events: bool,
    ) -> Result<TransactionWithProof> {
        self.error_if_ledger_pruned("Transaction", version)?;

        let proof = self
            .ledger_db
            .transaction_info_db()
            .get_transaction_info_with_proof(
                version,
                ledger_version,
                self.ledger_db.transaction_accumulator_db(),
            )?;

        let transaction = self.ledger_db.transaction_db().get_transaction(version)?;

        // If events were requested, also fetch those.
        let events = if fetch_events {
            Some(self.ledger_db.event_db().get_events_by_version(version)?)
        } else {
            None
        };

        Ok(TransactionWithProof {
            version,
            transaction,
            events,
            proof,
        })
    }

    /// TODO(bowu): Deprecate after internal index migration
    fn get_events_by_event_key(
        &self,
        event_key: &EventKey,
        start_seq_num: u64,
        order: Order,
        limit: u64,
        ledger_version: Version,
    ) -> Result<Vec<EventWithVersion>> {
        ensure!(
            !self.state_kv_db.enabled_sharding(),
            "This API is deprecated for sharded DB"
        );
        error_if_too_many_requested(limit, MAX_REQUEST_LIMIT)?;
        let get_latest = order == Order::Descending && start_seq_num == u64::MAX;

        let cursor = if get_latest {
            // Caller wants the latest, figure out the latest seq_num.
            // In the case of no events on that path, use 0 and expect empty result below.
            self.event_store
                .get_latest_sequence_number(ledger_version, event_key)?
                .unwrap_or(0)
        } else {
            start_seq_num
        };

        // Convert requested range and order to a range in ascending order.
        let (first_seq, real_limit) = get_first_seq_num_and_limit(order, cursor, limit)?;

        // Query the index.
        let mut event_indices = self.event_store.lookup_events_by_key(
            event_key,
            first_seq,
            real_limit,
            ledger_version,
        )?;

        // When descending, it's possible that user is asking for something beyond the latest
        // sequence number, in which case we will consider it a bad request and return an empty
        // list.
        // For example, if the latest sequence number is 100, and the caller is asking for 110 to
        // 90, we will get 90 to 100 from the index lookup above. Seeing that the last item
        // is 100 instead of 110 tells us 110 is out of bound.
        if order == Order::Descending {
            if let Some((seq_num, _, _)) = event_indices.last() {
                if *seq_num < cursor {
                    event_indices = Vec::new();
                }
            }
        }

        let mut events_with_version = event_indices
            .into_iter()
            .map(|(seq, ver, idx)| {
                let event = self.event_store.get_event_by_version_and_index(ver, idx)?;
                let v0 = match &event {
                    ContractEvent::V1(event) => event,
                    ContractEvent::V2(_) => bail!("Unexpected module event"),
                };
                ensure!(
                    seq == v0.sequence_number(),
                    "Index broken, expected seq:{}, actual:{}",
                    seq,
                    v0.sequence_number()
                );
                Ok(EventWithVersion::new(ver, event))
            })
            .collect::<Result<Vec<_>>>()?;
        if order == Order::Descending {
            events_with_version.reverse();
        }

        Ok(events_with_version)
    }

    /// TODO(jill): deprecate Indexer once Indexer Async V2 is ready
    fn get_table_info_option(&self, handle: TableHandle) -> Result<Option<TableInfo>> {
        match &self.indexer {
            Some(indexer) => indexer.get_table_info(handle),
            None => bail!("Indexer not enabled."),
        }
    }
}
