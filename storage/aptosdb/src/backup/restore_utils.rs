// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::StateStore;
///! This file contains utilities that are helpful for performing
///! database restore operations, as required by restore and
///! state sync v2.
use crate::{
    event_store::EventStore, ledger_store::LedgerStore, new_sharded_kv_schema_batch,
    schema::transaction_accumulator::TransactionAccumulatorSchema,
    transaction_store::TransactionStore, ShardedStateKvSchemaBatch,
};
use anyhow::{ensure, Result};
use aptos_crypto::HashValue;
use aptos_schemadb::{SchemaBatch, DB};
use aptos_types::{
    contract_event::ContractEvent,
    ledger_info::LedgerInfoWithSignatures,
    proof::{
        definition::LeafCount,
        position::{FrozenSubTreeIterator, Position},
    },
    transaction::{Transaction, TransactionInfo, TransactionOutput, Version},
    write_set::WriteSet,
};
use std::sync::Arc;

/// Saves the given ledger infos to the ledger store. If a change set is provided,
/// a batch of db alterations will be added to the change set without writing them to the db.
pub fn save_ledger_infos(
    ledger_metadata_db: &DB,
    ledger_store: Arc<LedgerStore>,
    ledger_infos: &[LedgerInfoWithSignatures],
    existing_batch: Option<&mut SchemaBatch>,
) -> Result<()> {
    ensure!(!ledger_infos.is_empty(), "No LedgerInfos to save.");

    if let Some(existing_batch) = existing_batch {
        save_ledger_infos_impl(ledger_store, ledger_infos, existing_batch)?;
    } else {
        let mut batch = SchemaBatch::new();
        save_ledger_infos_impl(ledger_store.clone(), ledger_infos, &mut batch)?;
        ledger_metadata_db.write_schemas(batch)?;
        update_latest_ledger_info(ledger_store, ledger_infos)?;
    }

    Ok(())
}

/// Updates the latest ledger info iff a ledger info with a higher epoch is found
pub fn update_latest_ledger_info(
    ledger_store: Arc<LedgerStore>,
    ledger_infos: &[LedgerInfoWithSignatures],
) -> Result<()> {
    if let Some(li) = ledger_store.get_latest_ledger_info_option() {
        if li.ledger_info().epoch() > ledger_infos.last().unwrap().ledger_info().epoch() {
            // No need to update latest ledger info.
            return Ok(());
        }
    }
    ledger_store.set_latest_ledger_info(ledger_infos.last().unwrap().clone());

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
    ledger_store: Arc<LedgerStore>,
    transaction_store: Arc<TransactionStore>,
    event_store: Arc<EventStore>,
    state_store: Arc<StateStore>,
    first_version: Version,
    txns: &[Transaction],
    txn_infos: &[TransactionInfo],
    events: &[Vec<ContractEvent>],
    write_sets: Vec<WriteSet>,
    existing_batch: Option<(&mut SchemaBatch, &mut ShardedStateKvSchemaBatch)>,
    kv_replay: bool,
) -> Result<()> {
    if let Some(existing_batch) = existing_batch {
        let batch = existing_batch.0;
        let state_kv_batches = existing_batch.1;
        save_transactions_impl(
            Arc::clone(&ledger_store),
            transaction_store,
            event_store,
            state_store,
            first_version,
            txns,
            txn_infos,
            events,
            write_sets.as_ref(),
            batch,
            state_kv_batches,
            kv_replay,
        )?;
    } else {
        let mut batch = SchemaBatch::new();
        let mut sharded_kv_schema_batch = new_sharded_kv_schema_batch();
        save_transactions_impl(
            Arc::clone(&ledger_store),
            transaction_store,
            event_store,
            Arc::clone(&state_store),
            first_version,
            txns,
            txn_infos,
            events,
            write_sets.as_ref(),
            &mut batch,
            &mut sharded_kv_schema_batch,
            kv_replay,
        )?;
        // get the last version and commit to the state kv db
        // commit the state kv before ledger in case of failure happens
        let last_version = first_version + txns.len() as u64 - 1;
        state_store
            .state_db
            .state_kv_db
            .commit(last_version, sharded_kv_schema_batch)?;

        // TODO(grao): Support splitted ledger DBs here.
        ledger_store.ledger_db.metadata_db().write_schemas(batch)?;
    }

    Ok(())
}

/// Saves the given transaction outputs to the db. If a change set is provided, a batch
/// of db alterations will be added to the change set without writing them to the db.
pub fn save_transaction_outputs(
    db: Arc<DB>,
    transaction_store: Arc<TransactionStore>,
    first_version: Version,
    transaction_outputs: Vec<TransactionOutput>,
    existing_batch: Option<&mut SchemaBatch>,
) -> Result<()> {
    if let Some(existing_batch) = existing_batch {
        save_transaction_outputs_impl(
            transaction_store,
            first_version,
            transaction_outputs,
            existing_batch,
        )?;
    } else {
        let mut batch = SchemaBatch::new();
        save_transaction_outputs_impl(
            transaction_store,
            first_version,
            transaction_outputs,
            &mut batch,
        )?;
        db.write_schemas(batch)?;
    }

    Ok(())
}

/// A helper function that saves the ledger infos to the given change set
fn save_ledger_infos_impl(
    ledger_store: Arc<LedgerStore>,
    ledger_infos: &[LedgerInfoWithSignatures],
    batch: &mut SchemaBatch,
) -> Result<()> {
    ledger_infos
        .iter()
        .map(|li| ledger_store.put_ledger_info(li, batch))
        .collect::<Result<Vec<_>>>()?;

    Ok(())
}

/// A helper function that saves the transactions to the given change set
pub(crate) fn save_transactions_impl(
    ledger_store: Arc<LedgerStore>,
    transaction_store: Arc<TransactionStore>,
    event_store: Arc<EventStore>,
    state_store: Arc<StateStore>,
    first_version: Version,
    txns: &[Transaction],
    txn_infos: &[TransactionInfo],
    events: &[Vec<ContractEvent>],
    write_sets: &[WriteSet],
    batch: &mut SchemaBatch,
    state_kv_batches: &mut ShardedStateKvSchemaBatch,
    kv_replay: bool,
) -> Result<()> {
    // TODO(grao): Support splited ledger db here.
    for (idx, txn) in txns.iter().enumerate() {
        transaction_store.put_transaction(first_version + idx as Version, txn, batch)?;
    }
    ledger_store.put_transaction_infos(first_version, txn_infos, batch, batch)?;
    event_store.put_events_multiple_versions(first_version, events, batch)?;
    // insert changes in write set schema batch
    for (idx, ws) in write_sets.iter().enumerate() {
        transaction_store.put_write_set(first_version + idx as Version, ws, batch)?;
    }

    if kv_replay && first_version > 0 && state_store.get_usage(Some(first_version - 1)).is_ok() {
        state_store.put_write_sets(write_sets.to_vec(), first_version, batch, state_kv_batches)?;
    }

    Ok(())
}

/// A helper function that saves the transaction outputs to the given change set
pub fn save_transaction_outputs_impl(
    transaction_store: Arc<TransactionStore>,
    first_version: Version,
    transaction_outputs: Vec<TransactionOutput>,
    batch: &mut SchemaBatch,
) -> Result<()> {
    for output in transaction_outputs {
        transaction_store.put_write_set(first_version, output.write_set(), batch)?;
    }

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
