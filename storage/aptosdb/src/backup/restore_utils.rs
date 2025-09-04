// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This file contains utilities that are helpful for performing
//! database restore operations, as required by restore and
//! state sync v2.
use crate::{
    ledger_db::{
        LedgerDb, LedgerDbSchemaBatches, ledger_metadata_db::LedgerMetadataDb,
        persisted_auxiliary_info_db::PersistedAuxiliaryInfoDb,
        transaction_info_db::TransactionInfoDb, write_set_db::WriteSetDb,
    },
    schema::{
        db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
        transaction_accumulator::TransactionAccumulatorSchema,
    },
    state_store::StateStore,
    utils::ShardedStateKvSchemaBatch,
};
use aptos_crypto::HashValue;
use aptos_schemadb::{DB, batch::SchemaBatch};
use aptos_storage_interface::{
    AptosDbError, Result, db_ensure as ensure, state_store::state_update_refs::StateUpdateRefs,
};
use aptos_types::{
    account_config::new_block_event_key,
    contract_event::ContractEvent,
    ledger_info::LedgerInfoWithSignatures,
    proof::{
        definition::LeafCount,
        position::{FrozenSubTreeIterator, Position},
    },
    transaction::{PersistedAuxiliaryInfo, Transaction, TransactionInfo, Version},
    write_set::WriteSet,
};
use std::sync::Arc;

/// Saves the given ledger infos to the ledger store. If a change set is provided,
/// a batch of db alterations will be added to the change set without writing them to the db.
pub(crate) fn save_ledger_infos(
    ledger_metadata_db: &LedgerMetadataDb,
    ledger_infos: &[LedgerInfoWithSignatures],
    existing_batch: Option<&mut SchemaBatch>,
) -> Result<()> {
    ensure!(!ledger_infos.is_empty(), "No LedgerInfos to save.");

    if let Some(existing_batch) = existing_batch {
        save_ledger_infos_impl(ledger_metadata_db, ledger_infos, existing_batch)?;
    } else {
        let mut batch = SchemaBatch::new();
        save_ledger_infos_impl(ledger_metadata_db, ledger_infos, &mut batch)?;
        ledger_metadata_db.write_schemas(batch)?;
        update_latest_ledger_info(ledger_metadata_db, ledger_infos)?;
    }

    Ok(())
}

/// Updates the latest ledger info iff a ledger info with a higher epoch is found
pub(crate) fn update_latest_ledger_info(
    ledger_metadata_db: &LedgerMetadataDb,
    ledger_infos: &[LedgerInfoWithSignatures],
) -> Result<()> {
    if let Some(li) = ledger_metadata_db.get_latest_ledger_info_option() {
        if li.ledger_info().epoch() > ledger_infos.last().unwrap().ledger_info().epoch() {
            // No need to update latest ledger info.
            return Ok(());
        }
    }
    ledger_metadata_db.set_latest_ledger_info(ledger_infos.last().unwrap().clone());

    Ok(())
}

/// Confirms or saves the frozen subtrees. If a change set is provided, a batch
/// of db alterations will be added to the change set without writing them to the db.
pub fn confirm_or_save_frozen_subtrees(
    transaction_accumulator_db: &DB,
    num_leaves: LeafCount,
    frozen_subtrees: &[HashValue],
    existing_batch: Option<&mut SchemaBatch>,
) -> Result<()> {
    let positions: Vec<_> = FrozenSubTreeIterator::new(num_leaves).collect();
    ensure!(
        positions.len() == frozen_subtrees.len(),
        "Number of frozen subtree roots not expected. Expected: {}, actual: {}",
        positions.len(),
        frozen_subtrees.len(),
    );

    if let Some(existing_batch) = existing_batch {
        confirm_or_save_frozen_subtrees_impl(
            transaction_accumulator_db,
            frozen_subtrees,
            positions,
            existing_batch,
        )?;
    } else {
        let mut batch = SchemaBatch::new();
        confirm_or_save_frozen_subtrees_impl(
            transaction_accumulator_db,
            frozen_subtrees,
            positions,
            &mut batch,
        )?;
        transaction_accumulator_db.write_schemas(batch)?;
    }

    Ok(())
}

/// Saves the given transactions to the db. If a change set is provided, a batch
/// of db alterations will be added to the change set without writing them to the db.
pub(crate) fn save_transactions(
    state_store: Arc<StateStore>,
    ledger_db: Arc<LedgerDb>,
    first_version: Version,
    txns: &[Transaction],
    persisted_aux_info: &[PersistedAuxiliaryInfo],
    txn_infos: &[TransactionInfo],
    events: &[Vec<ContractEvent>],
    write_sets: Vec<WriteSet>,
    existing_batch: Option<(
        &mut LedgerDbSchemaBatches,
        &mut ShardedStateKvSchemaBatch,
        &mut SchemaBatch,
    )>,
    kv_replay: bool,
) -> Result<()> {
    if let Some((ledger_db_batch, state_kv_batches, _state_kv_metadata_batch)) = existing_batch {
        save_transactions_impl(
            state_store,
            ledger_db,
            first_version,
            txns,
            persisted_aux_info,
            txn_infos,
            events,
            write_sets.as_ref(),
            ledger_db_batch,
            state_kv_batches,
            kv_replay,
        )?;
    } else {
        let mut ledger_db_batch = LedgerDbSchemaBatches::new();
        let mut sharded_kv_schema_batch = state_store
            .state_db
            .state_kv_db
            .new_sharded_native_batches();
        save_transactions_impl(
            Arc::clone(&state_store),
            Arc::clone(&ledger_db),
            first_version,
            txns,
            persisted_aux_info,
            txn_infos,
            events,
            write_sets.as_ref(),
            &mut ledger_db_batch,
            &mut sharded_kv_schema_batch,
            kv_replay,
        )?;
        // get the last version and commit to the state kv db
        // commit the state kv before ledger in case of failure happens
        let last_version = first_version + txns.len() as u64 - 1;
        state_store
            .state_db
            .state_kv_db
            .commit(last_version, None, sharded_kv_schema_batch)?;

        ledger_db.write_schemas(ledger_db_batch)?;
    }

    Ok(())
}

/// A helper function that saves the ledger infos to the given change set
fn save_ledger_infos_impl(
    ledger_metadata_db: &LedgerMetadataDb,
    ledger_infos: &[LedgerInfoWithSignatures],
    batch: &mut SchemaBatch,
) -> Result<()> {
    ledger_infos
        .iter()
        .map(|li| ledger_metadata_db.put_ledger_info(li, batch))
        .collect::<Result<Vec<_>>>()?;

    Ok(())
}

/// A helper function that saves the transactions to the given change set
pub(crate) fn save_transactions_impl(
    state_store: Arc<StateStore>,
    ledger_db: Arc<LedgerDb>,
    first_version: Version,
    txns: &[Transaction],
    persisted_aux_info: &[PersistedAuxiliaryInfo],
    txn_infos: &[TransactionInfo],
    events: &[Vec<ContractEvent>],
    write_sets: &[WriteSet],
    ledger_db_batch: &mut LedgerDbSchemaBatches,
    state_kv_batches: &mut ShardedStateKvSchemaBatch,
    kv_replay: bool,
) -> Result<()> {
    for (idx, txn) in txns.iter().enumerate() {
        ledger_db.transaction_db().put_transaction(
            first_version + idx as Version,
            txn,
            /*skip_index=*/ false,
            &mut ledger_db_batch.transaction_db_batches,
        )?;
    }

    for (idx, aux_info) in persisted_aux_info.iter().enumerate() {
        PersistedAuxiliaryInfoDb::put_persisted_auxiliary_info(
            first_version + idx as Version,
            aux_info,
            &mut ledger_db_batch.persisted_auxiliary_info_db_batches,
        )?;
    }

    for (idx, txn_info) in txn_infos.iter().enumerate() {
        TransactionInfoDb::put_transaction_info(
            first_version + idx as Version,
            txn_info,
            &mut ledger_db_batch.transaction_info_db_batches,
        )?;
    }

    ledger_db
        .transaction_accumulator_db()
        .put_transaction_accumulator(
            first_version,
            txn_infos,
            &mut ledger_db_batch.transaction_accumulator_db_batches,
        )?;

    ledger_db.event_db().put_events_multiple_versions(
        first_version,
        events,
        &mut ledger_db_batch.event_db_batches,
    )?;

    if ledger_db.enable_storage_sharding() {
        for (idx, txn_events) in events.iter().enumerate() {
            for event in txn_events {
                if let Some(event_key) = event.event_key() {
                    if *event_key == new_block_event_key() {
                        LedgerMetadataDb::put_block_info(
                            first_version + idx as Version,
                            event,
                            &mut ledger_db_batch.ledger_metadata_db_batches,
                        )?;
                    }
                }
            }
        }
    }
    // insert changes in write set schema batch
    for (idx, ws) in write_sets.iter().enumerate() {
        WriteSetDb::put_write_set(
            first_version + idx as Version,
            ws,
            &mut ledger_db_batch.write_set_db_batches,
        )?;
    }

    if kv_replay && first_version > 0 && state_store.get_usage(Some(first_version - 1)).is_ok() {
        let ledger_state = state_store.calculate_state_and_put_updates(
            &StateUpdateRefs::index_write_sets(first_version, write_sets, write_sets.len(), None),
            &mut ledger_db_batch.ledger_metadata_db_batches, // used for storing the storage usage
            state_kv_batches,
        )?;
        // n.b. ideally this is set after the batches are committed
        state_store.set_state_ignoring_summary(ledger_state);
    }

    let last_version = first_version + txns.len() as u64 - 1;
    ledger_db_batch
        .ledger_metadata_db_batches
        .put::<DbMetadataSchema>(
            &DbMetadataKey::LedgerCommitProgress,
            &DbMetadataValue::Version(last_version),
        )?;
    ledger_db_batch
        .ledger_metadata_db_batches
        .put::<DbMetadataSchema>(
            &DbMetadataKey::OverallCommitProgress,
            &DbMetadataValue::Version(last_version),
        )?;

    Ok(())
}

/// A helper function that confirms or saves the frozen subtrees to the given change set
fn confirm_or_save_frozen_subtrees_impl(
    transaction_accumulator_db: &DB,
    frozen_subtrees: &[HashValue],
    positions: Vec<Position>,
    batch: &mut SchemaBatch,
) -> Result<()> {
    positions
        .iter()
        .zip(frozen_subtrees.iter().rev())
        .map(|(p, h)| {
            if let Some(_h) = transaction_accumulator_db.get::<TransactionAccumulatorSchema>(p)? {
                ensure!(
                        h == &_h,
                        "Frozen subtree root does not match that already in DB. Provided: {}, in db: {}.",
                        h,
                        _h,
                    );
            } else {
                batch.put::<TransactionAccumulatorSchema>(p, h)?;
            }
            Ok(())
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(())
}
