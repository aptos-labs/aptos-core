// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use itertools::Itertools;

impl DbWriter for AptosDB {
    fn pre_commit_ledger(
        &self,
        txns_to_commit: &[TransactionToCommit],
        first_version: Version,
        base_state_version: Option<Version>,
        sync_commit: bool,
        latest_in_memory_state: StateDelta,
        state_updates_until_last_checkpoint: Option<ShardedStateUpdates>,
        sharded_state_cache: Option<&ShardedStateCache>,
    ) -> Result<()> {
        gauged_api("pre_commit_ledger", || {
            // Pre-committing and committing in concurrency is allowed but not pre-committing at the
            // same time from multiple threads, the same for committing.
            // Consensus and state sync must hand over to each other after all pending execution and
            // committing complete.
            let _lock = self
                .pre_commit_lock
                .try_lock()
                .expect("Concurrent committing detected.");
            let _timer = OTHER_TIMERS_SECONDS.timer_with(&["pre_commit_ledger"]);

            latest_in_memory_state.current.log_generation("db_save");

            self.pre_commit_validation(
                txns_to_commit,
                first_version,
                base_state_version,
                &latest_in_memory_state,
            )?;
            let last_version = first_version + txns_to_commit.len() as u64 - 1;

            let _new_root_hash = self.calculate_and_commit_ledger_and_state_kv(
                txns_to_commit,
                first_version,
                latest_in_memory_state.current.usage(),
                sharded_state_cache,
                self.skip_index_and_usage,
            )?;

            let _timer = OTHER_TIMERS_SECONDS.timer_with(&["save_transactions__others"]);
            {
                let mut buffered_state = self.state_store.buffered_state().lock();

                let _timer = OTHER_TIMERS_SECONDS.timer_with(&["buffered_state___update"]);
                buffered_state.update(
                    state_updates_until_last_checkpoint,
                    latest_in_memory_state,
                    sync_commit || txns_to_commit.last().unwrap().is_reconfig(),
                )?;
            }
            self.ledger_db.metadata_db().set_pre_committed_version(last_version);
            Ok(())
        })
    }

    fn commit_ledger(
        &self,
        version: Version,
        ledger_info_with_sigs: Option<&LedgerInfoWithSignatures>,
        txns_to_commit: Option<&[TransactionToCommit]>,
    ) -> Result<()> {
        gauged_api("commit_ledger", || {
            // Pre-committing and committing in concurrency is allowed but not pre-committing at the
            // same time from multiple threads, the same for committing.
            // Consensus and state sync must hand over to each other after all pending execution and
            // committing complete.
            let _lock = self
                .commit_lock
                .try_lock()
                .expect("Concurrent committing detected.");
            let _timer = OTHER_TIMERS_SECONDS.timer_with(&["commit_ledger"]);

            let old_committed_ver = self.get_and_check_commit_range(version)?;

            let ledger_batch = SchemaBatch::new();
            // Write down LedgerInfo if provided.
            if let Some(li) = ledger_info_with_sigs {
                self.check_and_put_ledger_info(version, li, &ledger_batch)?;
            }
            // Write down commit progress
            ledger_batch.put::<DbMetadataSchema>(
                &DbMetadataKey::OverallCommitProgress,
                &DbMetadataValue::Version(version),
            )?;
            self.ledger_db.metadata_db().write_schemas(ledger_batch)?;

            // Notify the pruners, invoke the indexer, and update in-memory ledger info.
            self.post_commit(
                old_committed_ver,
                version,
                ledger_info_with_sigs,
                txns_to_commit,
            )
        })
    }


    fn get_state_snapshot_receiver(
        &self,
        version: Version,
        expected_root_hash: HashValue,
    ) -> Result<Box<dyn StateSnapshotReceiver<StateKey, StateValue>>> {
        gauged_api("get_state_snapshot_receiver", || {
            self.state_store
                .get_snapshot_receiver(version, expected_root_hash)
        })
    }

    fn finalize_state_snapshot(
        &self,
        version: Version,
        output_with_proof: TransactionOutputListWithProof,
        ledger_infos: &[LedgerInfoWithSignatures],
    ) -> Result<()> {
        gauged_api("finalize_state_snapshot", || {
            // Ensure the output with proof only contains a single transaction output and info
            let num_transaction_outputs = output_with_proof.transactions_and_outputs.len();
            let num_transaction_infos = output_with_proof.proof.transaction_infos.len();
            ensure!(
                num_transaction_outputs == 1,
                "Number of transaction outputs should == 1, but got: {}",
                num_transaction_outputs
            );
            ensure!(
                num_transaction_infos == 1,
                "Number of transaction infos should == 1, but got: {}",
                num_transaction_infos
            );

            // TODO(joshlind): include confirm_or_save_frozen_subtrees in the change set
            // bundle below.

            // Update the merkle accumulator using the given proof
            let frozen_subtrees = output_with_proof
                .proof
                .ledger_info_to_transaction_infos_proof
                .left_siblings();
            restore_utils::confirm_or_save_frozen_subtrees(
                self.ledger_db.transaction_accumulator_db_raw(),
                version,
                frozen_subtrees,
                None,
            )?;

            // Create a single change set for all further write operations
            let mut ledger_db_batch = LedgerDbSchemaBatches::new();
            let mut sharded_kv_batch = new_sharded_kv_schema_batch();
            let state_kv_metadata_batch = SchemaBatch::new();
            // Save the target transactions, outputs, infos and events
            let (transactions, outputs): (Vec<Transaction>, Vec<TransactionOutput>) =
                output_with_proof
                    .transactions_and_outputs
                    .into_iter()
                    .unzip();
            let events = outputs
                .clone()
                .into_iter()
                .map(|output| output.events().to_vec())
                .collect::<Vec<_>>();
            let wsets: Vec<WriteSet> = outputs
                .into_iter()
                .map(|output| output.write_set().clone())
                .collect();
            let transaction_infos = output_with_proof.proof.transaction_infos;
            // We should not save the key value since the value is already recovered for this version
            restore_utils::save_transactions(
                self.state_store.clone(),
                self.ledger_db.clone(),
                version,
                &transactions,
                &transaction_infos,
                &events,
                wsets,
                Some((
                    &mut ledger_db_batch,
                    &mut sharded_kv_batch,
                    &state_kv_metadata_batch,
                )),
                false,
            )?;

            // Save the epoch ending ledger infos
            restore_utils::save_ledger_infos(
                self.ledger_db.metadata_db(),
                ledger_infos,
                Some(&mut ledger_db_batch.ledger_metadata_db_batches),
            )?;

            ledger_db_batch
                .ledger_metadata_db_batches
                .put::<DbMetadataSchema>(
                    &DbMetadataKey::LedgerCommitProgress,
                    &DbMetadataValue::Version(version),
                )?;
            ledger_db_batch
                .ledger_metadata_db_batches
                .put::<DbMetadataSchema>(
                    &DbMetadataKey::OverallCommitProgress,
                    &DbMetadataValue::Version(version),
                )?;

            // Apply the change set writes to the database (atomically) and update in-memory state
            //
            // state kv and SMT should use shared way of committing.
            self.ledger_db.write_schemas(ledger_db_batch)?;

            self.ledger_pruner.save_min_readable_version(version)?;
            self.state_store
                .state_merkle_pruner
                .save_min_readable_version(version)?;
            self.state_store
                .epoch_snapshot_pruner
                .save_min_readable_version(version)?;
            self.state_store
                .state_kv_pruner
                .save_min_readable_version(version)?;

            restore_utils::update_latest_ledger_info(self.ledger_db.metadata_db(), ledger_infos)?;
            self.ledger_db.metadata_db().set_pre_committed_version(version);
            self.state_store.reset();

            Ok(())
        })
    }
}

impl AptosDB {
    fn pre_commit_validation(
        &self,
        txns_to_commit: &[TransactionToCommit],
        first_version: Version,
        base_state_version: Option<Version>,
        latest_in_memory_state: &StateDelta,
    ) -> Result<()> {
        let _timer = OTHER_TIMERS_SECONDS
            .with_label_values(&["save_transactions_validation"])
            .start_timer();

        let num_txns = txns_to_commit.len() as u64;
        ensure!(
            num_txns > 0,
            "txns_to_commit is empty, nothing to save.",
        );
        let last_version = first_version + num_txns - 1;
        ensure!(
            Some(last_version) == latest_in_memory_state.current_version,
            "the last_version {:?} to commit doesn't match the current_version {:?} in latest_in_memory_state",
            last_version,
            latest_in_memory_state.current_version.expect("Must exist"),
        );

        let num_transactions_in_db = self.get_pre_committed_version()?.map_or(0, |v| v + 1);
        {
            let buffered_state = self.state_store.buffered_state().lock();
            ensure!(
                base_state_version == buffered_state.current_state().base_version,
                "base_state_version {:?} does not equal to the base_version {:?} in buffered state with current version {:?}",
                base_state_version,
                buffered_state.current_state().base_version,
                buffered_state.current_state().current_version,
            );

            // Ensure the incoming committing requests are always consecutive and the version in
            // buffered state is consistent with that in db.
            let next_version_in_buffered_state = buffered_state
                .current_state()
                .current_version
                .map(|version| version + 1)
                .unwrap_or(0);
            ensure!(num_transactions_in_db == first_version && num_transactions_in_db == next_version_in_buffered_state,
                "The first version passed in ({}), the next version in buffered state ({}) and the next version in db ({}) are inconsistent.",
                first_version,
                next_version_in_buffered_state,
                num_transactions_in_db,
            );
        }

        Ok(())
    }

    fn calculate_and_commit_ledger_and_state_kv(
        &self,
        txns_to_commit: &[TransactionToCommit],
        first_version: Version,
        expected_state_db_usage: StateStorageUsage,
        sharded_state_cache: Option<&ShardedStateCache>,
        skip_index_and_usage: bool,
    ) -> Result<HashValue> {
        let _timer = OTHER_TIMERS_SECONDS
            .with_label_values(&["save_transactions__work"])
            .start_timer();
        let mut new_root_hash = HashValue::zero();
        THREAD_MANAGER.get_non_exe_cpu_pool().scope(|s| {
            // TODO(grao): Write progress for each of the following databases, and handle the
            // inconsistency at the startup time.
            //
            // TODO(grao): Consider propagating the error instead of panic, if necessary.
            s.spawn(|_| {
                self.commit_events(txns_to_commit, first_version, skip_index_and_usage)
                    .unwrap()
            });
            s.spawn(|_| {
                self.ledger_db
                    .write_set_db()
                    .commit_write_sets(txns_to_commit, first_version)
                    .unwrap()
            });
            s.spawn(|_| {
                self.ledger_db
                    .transaction_db()
                    .commit_transactions(txns_to_commit, first_version, skip_index_and_usage)
                    .unwrap()
            });
            s.spawn(|_| {
                self.commit_state_kv_and_ledger_metadata(
                    txns_to_commit,
                    first_version,
                    expected_state_db_usage,
                    sharded_state_cache,
                    skip_index_and_usage,
                )
                .unwrap()
            });
            s.spawn(|_| {
                self.commit_transaction_infos(txns_to_commit, first_version)
                    .unwrap()
            });
            s.spawn(|_| {
                new_root_hash = self
                    .commit_transaction_accumulator(txns_to_commit, first_version)
                    .unwrap()
            });
            s.spawn(|_| {
                self.commit_transaction_auxiliary_data(txns_to_commit, first_version)
                    .unwrap()
            });
        });

        Ok(new_root_hash)
    }

    fn commit_state_kv_and_ledger_metadata(
        &self,
        txns_to_commit: &[TransactionToCommit],
        first_version: Version,
        expected_state_db_usage: StateStorageUsage,
        sharded_state_cache: Option<&ShardedStateCache>,
        skip_index_and_usage: bool,
    ) -> Result<()> {
        if txns_to_commit.is_empty() {
            return Ok(());
        }
        let _timer = OTHER_TIMERS_SECONDS
            .with_label_values(&["commit_state_kv_and_ledger_metadata"])
            .start_timer();
        let state_updates_vec = txns_to_commit
            .iter()
            .map(|txn_to_commit| txn_to_commit.state_updates())
            .collect::<Vec<_>>();

        let ledger_metadata_batch = SchemaBatch::new();
        let sharded_state_kv_batches = new_sharded_kv_schema_batch();
        let state_kv_metadata_batch = SchemaBatch::new();

        // TODO(grao): Make state_store take sharded state updates.
        self.state_store.put_value_sets(
            state_updates_vec,
            first_version,
            expected_state_db_usage,
            sharded_state_cache,
            &ledger_metadata_batch,
            &sharded_state_kv_batches,
            // Always put in state value index for now.
            // TODO(grao): remove after APIs migrated off the DB to the indexer.
            self.state_store.state_kv_db.enabled_sharding(),
            skip_index_and_usage,
            txns_to_commit
                .iter()
                .rposition(|txn| txn.has_state_checkpoint_hash()),
        )?;

        // Write block index if event index is skipped.
        if skip_index_and_usage {
            for (i, txn) in txns_to_commit.iter().enumerate() {
                for event in txn.events() {
                    if let Some(event_key) = event.event_key() {
                        if *event_key == new_block_event_key() {
                            let version = first_version + i as Version;
                            LedgerMetadataDb::put_block_info(
                                version,
                                event,
                                &ledger_metadata_batch,
                            )?;
                        }
                    }
                }
            }
        }

        let last_version = first_version + txns_to_commit.len() as u64 - 1;
        ledger_metadata_batch
            .put::<DbMetadataSchema>(
                &DbMetadataKey::LedgerCommitProgress,
                &DbMetadataValue::Version(last_version),
            )
            .unwrap();

        let _timer = OTHER_TIMERS_SECONDS
            .with_label_values(&["commit_state_kv_and_ledger_metadata___commit"])
            .start_timer();
        rayon::scope(|s| {
            s.spawn(|_| {
                self.ledger_db
                    .metadata_db()
                    .write_schemas(ledger_metadata_batch)
                    .unwrap();
            });
            s.spawn(|_| {
                self.state_kv_db
                    .commit(
                        last_version,
                        state_kv_metadata_batch,
                        sharded_state_kv_batches,
                    )
                    .unwrap();
            });
        });

        Ok(())
    }

    fn commit_events(
        &self,
        txns_to_commit: &[TransactionToCommit],
        first_version: Version,
        skip_index: bool,
    ) -> Result<()> {
        let _timer = OTHER_TIMERS_SECONDS
            .with_label_values(&["commit_events"])
            .start_timer();
        let batch = SchemaBatch::new();
        let num_txns = txns_to_commit.len();
        txns_to_commit
            .par_iter()
            .with_min_len(optimal_min_len(num_txns, 128))
            .enumerate()
            .try_for_each(|(i, txn_to_commit)| -> Result<()> {
                self.ledger_db.event_db().put_events(
                    first_version + i as u64,
                    txn_to_commit.events(),
                    skip_index,
                    &batch,
                )?;

                Ok(())
            })?;
        let _timer = OTHER_TIMERS_SECONDS
            .with_label_values(&["commit_events___commit"])
            .start_timer();
        self.ledger_db.event_db().write_schemas(batch)
    }

    fn commit_transaction_accumulator(
        &self,
        txns_to_commit: &[TransactionToCommit],
        first_version: u64,
    ) -> Result<HashValue> {
        let _timer = OTHER_TIMERS_SECONDS
            .with_label_values(&["commit_transaction_accumulator"])
            .start_timer();

        let batch = SchemaBatch::new();
        let root_hash = self
            .ledger_db
            .transaction_accumulator_db()
            .put_transaction_accumulator(
                first_version,
                &txns_to_commit
                    .iter()
                    .map(|txn_to_commit| txn_to_commit.transaction_info())
                    .collect::<Vec<_>>(),
                &batch,
            )?;

        let _timer = OTHER_TIMERS_SECONDS
            .with_label_values(&["commit_transaction_accumulator___commit"])
            .start_timer();
        self.ledger_db
            .transaction_accumulator_db()
            .write_schemas(batch)?;

        let batch = SchemaBatch::new();
        let all_versions: Vec<_> =
            (first_version..first_version + txns_to_commit.len() as u64).collect();
        THREAD_MANAGER
            .get_non_exe_cpu_pool()
            .install(|| -> Result<()> {
                let all_root_hashes = all_versions
                    .into_par_iter()
                    .with_min_len(64)
                    .map(|version| {
                        self.ledger_db
                            .transaction_accumulator_db()
                            .get_root_hash(version)
                    })
                    .collect::<Result<Vec<_>>>()?;
                all_root_hashes
                    .iter()
                    .enumerate()
                    .try_for_each(|(i, hash)| {
                        let version = first_version + i as u64;
                        batch.put::<TransactionAccumulatorRootHashSchema>(&version, hash)
                    })?;
                self.ledger_db
                    .transaction_accumulator_db()
                    .write_schemas(batch)
            })?;

        Ok(root_hash)
    }

    fn commit_transaction_auxiliary_data(
        &self,
        txns_to_commit: &[TransactionToCommit],
        first_version: u64,
    ) -> Result<()> {
        let _timer = OTHER_TIMERS_SECONDS
            .with_label_values(&["commit_transaction_auxiliary_data"])
            .start_timer();

        let batch = SchemaBatch::new();
        txns_to_commit
            .iter()
            .enumerate()
            .try_for_each(|(i, txn_to_commit)| -> Result<()> {
                TransactionAuxiliaryDataDb::put_transaction_auxiliary_data(
                    first_version + i as u64,
                    txn_to_commit.transaction_auxiliary_data(),
                    &batch,
                )?;

                Ok(())
            })?;

        let _timer = OTHER_TIMERS_SECONDS
            .with_label_values(&["commit_transaction_auxiliary_data___commit"])
            .start_timer();
        self.ledger_db
            .transaction_auxiliary_data_db()
            .write_schemas(batch)
    }

    fn commit_transaction_infos(
        &self,
        txns_to_commit: &[TransactionToCommit],
        first_version: u64,
    ) -> Result<()> {
        let _timer = OTHER_TIMERS_SECONDS
            .with_label_values(&["commit_transaction_infos"])
            .start_timer();
        let batch = SchemaBatch::new();
        let num_txns = txns_to_commit.len();
        txns_to_commit
            .par_iter()
            .with_min_len(optimal_min_len(num_txns, 128))
            .enumerate()
            .try_for_each(|(i, txn_to_commit)| -> Result<()> {
                let version = first_version + i as u64;
                TransactionInfoDb::put_transaction_info(
                    version,
                    txn_to_commit.transaction_info(),
                    &batch,
                )?;

                Ok(())
            })?;

        let _timer = OTHER_TIMERS_SECONDS
            .with_label_values(&["commit_transaction_infos___commit"])
            .start_timer();
        self.ledger_db.transaction_info_db().write_schemas(batch)
    }

    fn get_and_check_commit_range(
        &self,
        version_to_commit: Version,
    ) -> Result<Option<Version>> {
        let old_committed_ver = self.ledger_db.metadata_db().get_synced_version()?;
        let pre_committed_ver = self.ledger_db.metadata_db().get_pre_committed_version();
        ensure!(
            old_committed_ver.is_none() || version_to_commit >= old_committed_ver.unwrap(),
            "Version too old to commit. Committed: {:?}; Trying to commit with LI: {}",
            old_committed_ver,
            version_to_commit,
        );
        ensure!(
            pre_committed_ver.is_some() && version_to_commit <= pre_committed_ver.unwrap(),
            "Version too new to commit. Pre-committed: {:?}, Trying to commit with LI: {}",
            pre_committed_ver,
            version_to_commit,
        );
        Ok(old_committed_ver)
    }

    fn check_and_put_ledger_info(
        &self,
        version: Version,
        ledger_info_with_sig: &LedgerInfoWithSignatures,
        ledger_batch: &SchemaBatch
    ) -> Result<(), AptosDbError> {
        let ledger_info = ledger_info_with_sig.ledger_info();

        // Verify the version.
        ensure!(
            ledger_info.version() == version,
            "Version in LedgerInfo doesn't match last version. {:?} vs {:?}",
            ledger_info.version(),
            version,
        );

        // Verify the root hash.
        let db_root_hash = self.ledger_db.transaction_accumulator_db().get_root_hash(version)?;
        let li_root_hash = ledger_info_with_sig.ledger_info().transaction_accumulator_hash();
        ensure!(
            db_root_hash == li_root_hash,
            "Root hash pre-committed doesn't match LedgerInfo. pre-commited: {:?} vs in LedgerInfo: {:?}",
            db_root_hash,
            li_root_hash,
        );

        // Verify epoch continuity.
        let current_epoch = self
            .ledger_db
            .metadata_db()
            .get_latest_ledger_info_option()
            .map_or(0, |li| li.ledger_info().next_block_epoch());
        ensure!(
            ledger_info_with_sig.ledger_info().epoch() == current_epoch,
            "Gap in epoch history. Trying to put in LedgerInfo in epoch: {}, current epoch: {}",
            ledger_info_with_sig.ledger_info().epoch(),
            current_epoch,
        );

        // Put write to batch.
        self.ledger_db
            .metadata_db()
            .put_ledger_info(ledger_info_with_sig, ledger_batch)?;
        Ok(())
    }

    fn post_commit(
        &self,
        old_committed_version: Option<Version>,
        version: Version,
        ledger_info_with_sigs: Option<&LedgerInfoWithSignatures>,
        txns_to_commit: Option<&[TransactionToCommit]>,
    ) -> Result<()> {
        // If commit succeeds and there are at least one transaction written to the storage, we
        // will inform the pruner thread to work.
        if old_committed_version.is_none() || version > old_committed_version.unwrap() {
            let first_version = old_committed_version.map_or(0, |v| v + 1);
            let num_txns = version + 1 - first_version;

            COMMITTED_TXNS.inc_by(num_txns);
            LATEST_TXN_VERSION.set(version as i64);
            // Activate the ledger pruner and state kv pruner.
            // Note the state merkle pruner is activated when state snapshots are persisted
            // in their async thread.
            self.ledger_pruner
                .maybe_set_pruner_target_db_version(version);
            self.state_store
                .state_kv_pruner
                .maybe_set_pruner_target_db_version(version);

            // Note: this must happen after txns have been saved to db because types can be newly
            // created in this same chunk of transactions.
            if let Some(indexer) = &self.indexer {
                let _timer = OTHER_TIMERS_SECONDS.timer_with(&["indexer_index"]);
                // n.b. txns_to_commit can be partial, when the control was handed over from consensus to state sync
                // where state sync won't send the pre-committed part to the DB again.
                if txns_to_commit.is_some() && txns_to_commit.unwrap().len() == num_txns as usize {
                    let write_sets = txns_to_commit.unwrap().iter().map(|txn| txn.write_set()).collect_vec();
                    indexer.index(self.state_store.clone(), first_version, &write_sets)?;
                } else {
                    let write_sets: Vec<_> = self.ledger_db.write_set_db().get_write_set_iter(first_version, num_txns as usize)?.try_collect()?;
                    let write_set_refs = write_sets.iter().collect_vec();
                    indexer.index(self.state_store.clone(), first_version, &write_set_refs)?;
                };
            }
        }

        // Once everything is successfully persisted, update the latest in-memory ledger info.
        if let Some(x) = ledger_info_with_sigs {
            self.ledger_db
                .metadata_db()
                .set_latest_ledger_info(x.clone());

            LEDGER_VERSION.set(x.ledger_info().version() as i64);
            NEXT_BLOCK_EPOCH.set(x.ledger_info().next_block_epoch() as i64);
        }

        Ok(())
    }
}
