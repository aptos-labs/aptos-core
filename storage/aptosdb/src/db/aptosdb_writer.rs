// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    backup::restore_utils,
    db::{AptosDB, aptosdb_internal::gauged_api},
    ledger_db::{
        LedgerDbSchemaBatches, ledger_metadata_db::LedgerMetadataDb,
        transaction_auxiliary_data_db::TransactionAuxiliaryDataDb,
        transaction_info_db::TransactionInfoDb,
    },
    metrics::{
        COMMITTED_TXNS, LATEST_TXN_VERSION, LEDGER_VERSION, NEXT_BLOCK_EPOCH, OTHER_TIMERS_SECONDS,
    },
    pruner::PrunerManager,
    schema::{
        db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
        transaction_accumulator_root_hash::TransactionAccumulatorRootHashSchema,
    },
};
use aptos_crypto::HashValue;
use aptos_experimental_runtimes::thread_manager::THREAD_MANAGER;
use aptos_metrics_core::TimerHelper;
use aptos_schemadb::batch::SchemaBatch;
use aptos_storage_interface::{
    AptosDbError, DbReader, DbWriter, Result, StateSnapshotReceiver,
    chunk_to_commit::ChunkToCommit, db_ensure as ensure,
};
use aptos_types::{
    account_config::new_block_event_key,
    ledger_info::LedgerInfoWithSignatures,
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::{
        Transaction, TransactionAuxiliaryData, TransactionInfo, TransactionOutput,
        TransactionOutputListWithProofV2, Version,
    },
    write_set::WriteSet,
};
use itertools::Itertools;
use rayon::prelude::*;
use std::{iter::Iterator, time::Instant};

impl DbWriter for AptosDB {
    fn pre_commit_ledger(&self, chunk: ChunkToCommit, sync_commit: bool) -> Result<()> {
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

            chunk
                .state_summary
                .latest()
                .global_state_summary
                .log_generation("db_save");

            self.pre_commit_validation(&chunk)?;
            let _new_root_hash =
                self.calculate_and_commit_ledger_and_state_kv(&chunk, self.skip_index_and_usage)?;

            let _timer = OTHER_TIMERS_SECONDS.timer_with(&["save_transactions__others"]);

            self.state_store.buffered_state().lock().update(
                chunk.result_ledger_state_with_summary(),
                chunk.estimated_total_state_updates(),
                sync_commit || chunk.is_reconfig,
            )?;

            Ok(())
        })
    }

    fn commit_ledger(
        &self,
        version: Version,
        ledger_info_with_sigs: Option<&LedgerInfoWithSignatures>,
        chunk_opt: Option<ChunkToCommit>,
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

            let mut ledger_batch = SchemaBatch::new();
            // Write down LedgerInfo if provided.
            if let Some(li) = ledger_info_with_sigs {
                self.check_and_put_ledger_info(version, li, &mut ledger_batch)?;
            }
            // Write down commit progress
            ledger_batch.put::<DbMetadataSchema>(
                &DbMetadataKey::OverallCommitProgress,
                &DbMetadataValue::Version(version),
            )?;
            self.ledger_db.metadata_db().write_schemas(ledger_batch)?;

            // Notify the pruners, invoke the indexer, and update in-memory ledger info.
            self.post_commit(old_committed_ver, version, ledger_info_with_sigs, chunk_opt)
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
        output_with_proof: TransactionOutputListWithProofV2,
        ledger_infos: &[LedgerInfoWithSignatures],
    ) -> Result<()> {
        let (output_with_proof, persisted_aux_info) = output_with_proof.into_parts();
        gauged_api("finalize_state_snapshot", || {
            // Ensure the output with proof only contains a single transaction output and info
            let num_transaction_outputs = output_with_proof.get_num_outputs();
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
            let mut sharded_kv_batch = self.state_kv_db.new_sharded_native_batches();
            let mut state_kv_metadata_batch = SchemaBatch::new();
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
                &persisted_aux_info,
                &transaction_infos,
                &events,
                wsets,
                Some((
                    &mut ledger_db_batch,
                    &mut sharded_kv_batch,
                    &mut state_kv_metadata_batch,
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
            self.state_store.reset();

            Ok(())
        })
    }
}

impl AptosDB {
    fn pre_commit_validation(&self, chunk: &ChunkToCommit) -> Result<()> {
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["save_transactions_validation"]);

        ensure!(!chunk.is_empty(), "chunk is empty, nothing to save.");

        let next_version = self.state_store.current_state_locked().next_version();
        // Ensure the incoming committing requests are always consecutive and the version in
        // buffered state is consistent with that in db.
        ensure!(
            chunk.first_version == next_version,
            "The first version passed in ({}), and the next version expected by db ({}) are inconsistent.",
            chunk.first_version,
            next_version,
        );

        Ok(())
    }

    fn calculate_and_commit_ledger_and_state_kv(
        &self,
        chunk: &ChunkToCommit,
        skip_index_and_usage: bool,
    ) -> Result<HashValue> {
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["save_transactions__work"]);

        let mut new_root_hash = HashValue::zero();
        THREAD_MANAGER.get_non_exe_cpu_pool().scope(|s| {
            // TODO(grao): Write progress for each of the following databases, and handle the
            // inconsistency at the startup time.
            //
            // TODO(grao): Consider propagating the error instead of panic, if necessary.
            s.spawn(|_| {
                self.commit_events(
                    chunk.first_version,
                    chunk.transaction_outputs,
                    skip_index_and_usage,
                )
                .unwrap()
            });
            s.spawn(|_| {
                self.ledger_db
                    .write_set_db()
                    .commit_write_sets(chunk.first_version, chunk.transaction_outputs)
                    .unwrap()
            });
            s.spawn(|_| {
                self.ledger_db
                    .transaction_db()
                    .commit_transactions(
                        chunk.first_version,
                        chunk.transactions,
                        skip_index_and_usage,
                    )
                    .unwrap()
            });
            s.spawn(|_| {
                self.ledger_db
                    .persisted_auxiliary_info_db()
                    .commit_auxiliary_info(chunk.first_version, chunk.persisted_auxiliary_infos)
                    .unwrap()
            });
            s.spawn(|_| {
                self.commit_state_kv_and_ledger_metadata(chunk, skip_index_and_usage)
                    .unwrap()
            });
            s.spawn(|_| {
                self.commit_transaction_infos(chunk.first_version, chunk.transaction_infos)
                    .unwrap()
            });
            s.spawn(|_| {
                new_root_hash = self
                    .commit_transaction_accumulator(chunk.first_version, chunk.transaction_infos)
                    .unwrap()
            });
        });

        Ok(new_root_hash)
    }

    fn commit_state_kv_and_ledger_metadata(
        &self,
        chunk: &ChunkToCommit,
        skip_index_and_usage: bool,
    ) -> Result<()> {
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["commit_state_kv_and_ledger_metadata"]);

        let mut ledger_metadata_batch = SchemaBatch::new();
        let mut sharded_state_kv_batches = self.state_kv_db.new_sharded_native_batches();

        self.state_store.put_state_updates(
            chunk.state,
            &chunk.state_update_refs.per_version,
            chunk.state_reads,
            &mut ledger_metadata_batch,
            &mut sharded_state_kv_batches,
        )?;

        // Write block index if event index is skipped.
        if skip_index_and_usage {
            for (i, txn_out) in chunk.transaction_outputs.iter().enumerate() {
                for event in txn_out.events() {
                    if let Some(event_key) = event.event_key() {
                        if *event_key == new_block_event_key() {
                            let version = chunk.first_version + i as Version;
                            LedgerMetadataDb::put_block_info(
                                version,
                                event,
                                &mut ledger_metadata_batch,
                            )?;
                        }
                    }
                }
            }
        }

        ledger_metadata_batch
            .put::<DbMetadataSchema>(
                &DbMetadataKey::LedgerCommitProgress,
                &DbMetadataValue::Version(chunk.expect_last_version()),
            )
            .unwrap();

        let _timer =
            OTHER_TIMERS_SECONDS.timer_with(&["commit_state_kv_and_ledger_metadata___commit"]);
        rayon::scope(|s| {
            s.spawn(|_| {
                self.ledger_db
                    .metadata_db()
                    .write_schemas(ledger_metadata_batch)
                    .unwrap();
            });
            s.spawn(|_| {
                self.state_kv_db
                    .commit(chunk.expect_last_version(), None, sharded_state_kv_batches)
                    .unwrap();
            });
        });

        Ok(())
    }

    fn commit_events(
        &self,
        first_version: Version,
        transaction_outputs: &[TransactionOutput],
        skip_index: bool,
    ) -> Result<()> {
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["commit_events"]);

        let chunk_size = transaction_outputs.len() / 4 + 1;
        let batches = transaction_outputs
            .par_chunks(chunk_size)
            .enumerate()
            .map(|(chunk_idx, chunk)| {
                let mut batch = self.ledger_db.event_db().db().new_native_batch();
                let chunk_first_ver = first_version + (chunk_size * chunk_idx) as u64;
                chunk.iter().enumerate().try_for_each(|(i, txn_out)| {
                    self.ledger_db.event_db().put_events(
                        chunk_first_ver + i as Version,
                        txn_out.events(),
                        skip_index,
                        &mut batch,
                    )
                })?;
                Ok(batch)
            })
            .collect::<Result<Vec<_>>>()?;

        {
            let _timer = OTHER_TIMERS_SECONDS.timer_with(&["commit_events___commit"]);
            for batch in batches {
                self.ledger_db.event_db().db().write_schemas(batch)?
            }
            Ok(())
        }
    }

    pub(super) fn commit_transaction_accumulator(
        &self,
        first_version: Version,
        transaction_infos: &[TransactionInfo],
    ) -> Result<HashValue> {
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["commit_transaction_accumulator"]);

        let num_txns = transaction_infos.len() as Version;

        let mut batch = SchemaBatch::new();
        let root_hash = self
            .ledger_db
            .transaction_accumulator_db()
            .put_transaction_accumulator(first_version, transaction_infos, &mut batch)?;

        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["commit_transaction_accumulator___commit"]);
        self.ledger_db
            .transaction_accumulator_db()
            .write_schemas(batch)?;

        let mut batch = SchemaBatch::new();
        let all_versions: Vec<_> = (first_version..first_version + num_txns).collect();
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

    #[allow(dead_code)]
    pub(super) fn commit_transaction_auxiliary_data<'a>(
        &self,
        first_version: Version,
        auxiliary_data: impl IntoIterator<Item = &'a TransactionAuxiliaryData>,
    ) -> Result<()> {
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["commit_transaction_auxiliary_data"]);

        let mut batch = SchemaBatch::new();
        auxiliary_data
            .into_iter()
            .enumerate()
            .try_for_each(|(i, aux_data)| -> Result<()> {
                TransactionAuxiliaryDataDb::put_transaction_auxiliary_data(
                    first_version + i as Version,
                    aux_data,
                    &mut batch,
                )?;

                Ok(())
            })?;

        let _timer =
            OTHER_TIMERS_SECONDS.timer_with(&["commit_transaction_auxiliary_data___commit"]);
        self.ledger_db
            .transaction_auxiliary_data_db()
            .write_schemas(batch)
    }

    pub(super) fn commit_transaction_infos(
        &self,
        first_version: Version,
        txn_infos: &[TransactionInfo],
    ) -> Result<()> {
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["commit_transaction_infos"]);

        let mut batch = SchemaBatch::new();
        txn_infos
            .iter()
            .enumerate()
            .try_for_each(|(i, txn_info)| -> Result<()> {
                let version = first_version + i as u64;
                TransactionInfoDb::put_transaction_info(version, txn_info, &mut batch)?;

                Ok(())
            })?;

        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["commit_transaction_infos___commit"]);
        self.ledger_db.transaction_info_db().write_schemas(batch)
    }

    fn get_and_check_commit_range(&self, version_to_commit: Version) -> Result<Option<Version>> {
        let old_committed_ver = self.ledger_db.metadata_db().get_synced_version()?;
        let pre_committed_ver = self.state_store.current_state_locked().version();
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
        ledger_batch: &mut SchemaBatch,
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
        let db_root_hash = self
            .ledger_db
            .transaction_accumulator_db()
            .get_root_hash(version)?;
        let li_root_hash = ledger_info_with_sig
            .ledger_info()
            .transaction_accumulator_hash();
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

        // Ensure that state tree at the end of the epoch is persisted.
        if ledger_info_with_sig.ledger_info().ends_epoch() {
            let state_snapshot = self.state_store.get_state_snapshot_before(version + 1)?;
            ensure!(
                state_snapshot.is_some() && state_snapshot.as_ref().unwrap().0 == version,
                "State checkpoint not persisted at the end of the epoch, version {}, next_epoch {}, snapshot in db: {:?}",
                version,
                ledger_info_with_sig.ledger_info().next_block_epoch(),
                state_snapshot,
            );
        }

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
        chunk_opt: Option<ChunkToCommit>,
    ) -> Result<()> {
        // If commit succeeds and there are at least one transaction written to the storage, we
        // will inform the pruner thread to work.
        if old_committed_version.is_none() || version > old_committed_version.unwrap() {
            let first_version = old_committed_version.map_or(0, |v| v + 1);
            let num_txns = version + 1 - first_version;

            COMMITTED_TXNS.inc_by(num_txns);
            LATEST_TXN_VERSION.set(version as i64);
            if let Some(update_sender) = &self.update_subscriber {
                update_sender
                    .send((Instant::now(), version))
                    .map_err(|err| {
                        AptosDbError::Other(format!("Failed to send update to subscriber: {}", err))
                    })?;
            }
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
                if chunk_opt.is_some() && chunk_opt.as_ref().unwrap().len() == num_txns as usize {
                    let write_sets = chunk_opt
                        .as_ref()
                        .unwrap()
                        .transaction_outputs
                        .iter()
                        .map(|t| t.write_set())
                        .collect_vec();
                    indexer.index(self.state_store.clone(), first_version, &write_sets)?;
                } else {
                    let write_sets: Vec<_> = self
                        .ledger_db
                        .write_set_db()
                        .get_write_set_iter(first_version, num_txns as usize)?
                        .try_collect()?;
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
