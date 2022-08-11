// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

///! This file contains utilities that are helpful for performing
///! database restore operations, as required by db-restore and
///! state sync v2.
use crate::{
    change_set::ChangeSet, event_store::EventStore, ledger_store::LedgerStore,
    schema::transaction_accumulator::TransactionAccumulatorSchema,
    transaction_store::TransactionStore,
};
use anyhow::{ensure, Result};
use aptos_crypto::HashValue;
use aptos_types::proof::position::Position;
use aptos_types::{
    contract_event::ContractEvent,
    ledger_info::LedgerInfoWithSignatures,
    proof::{definition::LeafCount, position::FrozenSubTreeIterator},
    transaction::{Transaction, TransactionInfo, TransactionOutput, Version},
};
use schemadb::DB;
use std::sync::Arc;

/// Saves the given ledger infos to the ledger store. If a change set is provided,
/// a batch of db alterations will be added to the change set without writing them to the db.
pub fn save_ledger_infos(
    db: Arc<DB>,
    ledger_store: Arc<LedgerStore>,
    ledger_infos: &[LedgerInfoWithSignatures],
    existing_change_set: Option<&mut ChangeSet>,
) -> Result<()> {
    ensure!(!ledger_infos.is_empty(), "No LedgerInfos to save.");

    if let Some(existing_change_set) = existing_change_set {
        save_ledger_infos_impl(ledger_store, ledger_infos, existing_change_set)?;
    } else {
        let mut change_set = ChangeSet::new();
        save_ledger_infos_impl(ledger_store.clone(), ledger_infos, &mut change_set)?;
        db.write_schemas(change_set.batch)?;
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
    db: Arc<DB>,
    num_leaves: LeafCount,
    frozen_subtrees: &[HashValue],
    existing_change_set: Option<&mut ChangeSet>,
) -> Result<()> {
    let positions: Vec<_> = FrozenSubTreeIterator::new(num_leaves).collect();
    ensure!(
        positions.len() == frozen_subtrees.len(),
        "Number of frozen subtree roots not expected. Expected: {}, actual: {}",
        positions.len(),
        frozen_subtrees.len(),
    );

    if let Some(existing_change_set) = existing_change_set {
        confirm_or_save_frozen_subtrees_impl(db, frozen_subtrees, positions, existing_change_set)?;
    } else {
        let mut change_set = ChangeSet::new();
        confirm_or_save_frozen_subtrees_impl(
            db.clone(),
            frozen_subtrees,
            positions,
            &mut change_set,
        )?;
        db.write_schemas(change_set.batch)?;
    }

    Ok(())
}

/// Saves the given transactions to the db. If a change set is provided, a batch
/// of db alterations will be added to the change set without writing them to the db.
pub fn save_transactions(
    db: Arc<DB>,
    ledger_store: Arc<LedgerStore>,
    transaction_store: Arc<TransactionStore>,
    event_store: Arc<EventStore>,
    first_version: Version,
    txns: &[Transaction],
    txn_infos: &[TransactionInfo],
    events: &[Vec<ContractEvent>],
    existing_change_set: Option<&mut ChangeSet>,
) -> Result<()> {
    if let Some(existing_change_set) = existing_change_set {
        save_transactions_impl(
            ledger_store,
            transaction_store,
            event_store,
            first_version,
            txns,
            txn_infos,
            events,
            existing_change_set,
        )?;
    } else {
        let mut change_set = ChangeSet::new();
        save_transactions_impl(
            ledger_store,
            transaction_store,
            event_store,
            first_version,
            txns,
            txn_infos,
            events,
            &mut change_set,
        )?;
        db.write_schemas(change_set.batch)?;
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
    existing_change_set: Option<&mut ChangeSet>,
) -> Result<()> {
    if let Some(existing_change_set) = existing_change_set {
        save_transaction_outputs_impl(
            transaction_store,
            first_version,
            transaction_outputs,
            existing_change_set,
        )?;
    } else {
        let mut change_set = ChangeSet::new();
        save_transaction_outputs_impl(
            transaction_store,
            first_version,
            transaction_outputs,
            &mut change_set,
        )?;
        db.write_schemas(change_set.batch)?;
    }

    Ok(())
}

/// A helper function that saves the ledger infos to the given change set
fn save_ledger_infos_impl(
    ledger_store: Arc<LedgerStore>,
    ledger_infos: &[LedgerInfoWithSignatures],
    change_set: &mut ChangeSet,
) -> Result<()> {
    ledger_infos
        .iter()
        .map(|li| ledger_store.put_ledger_info(li, change_set))
        .collect::<Result<Vec<_>>>()?;

    Ok(())
}

/// A helper function that saves the transactions to the given change set
pub fn save_transactions_impl(
    ledger_store: Arc<LedgerStore>,
    transaction_store: Arc<TransactionStore>,
    event_store: Arc<EventStore>,
    first_version: Version,
    txns: &[Transaction],
    txn_infos: &[TransactionInfo],
    events: &[Vec<ContractEvent>],
    change_set: &mut ChangeSet,
) -> Result<()> {
    for (idx, txn) in txns.iter().enumerate() {
        transaction_store.put_transaction(first_version + idx as Version, txn, change_set)?;
    }
    ledger_store.put_transaction_infos(first_version, txn_infos, change_set)?;
    event_store.put_events_multiple_versions(first_version, events, change_set)?;

    Ok(())
}

/// A helper function that saves the transaction outputs to the given change set
pub fn save_transaction_outputs_impl(
    transaction_store: Arc<TransactionStore>,
    first_version: Version,
    transaction_outputs: Vec<TransactionOutput>,
    change_set: &mut ChangeSet,
) -> Result<()> {
    for output in transaction_outputs {
        transaction_store.put_write_set(first_version, output.write_set(), change_set)?;
    }

    Ok(())
}

/// A helper function that confirms or saves the frozen subtrees to the given change set
fn confirm_or_save_frozen_subtrees_impl(
    db: Arc<DB>,
    frozen_subtrees: &[HashValue],
    positions: Vec<Position>,
    change_set: &mut ChangeSet,
) -> Result<()> {
    positions
        .iter()
        .zip(frozen_subtrees.iter().rev())
        .map(|(p, h)| {
            if let Some(_h) = db.get::<TransactionAccumulatorSchema>(p)? {
                ensure!(
                        h == &_h,
                        "Frozen subtree root does not match that already in DB. Provided: {}, in db: {}.",
                        h,
                        _h,
                    );
            } else {
                change_set.batch.put::<TransactionAccumulatorSchema>(p, h)?;
            }
            Ok(())
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(())
}
