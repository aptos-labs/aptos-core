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
use aptos_types::{
    contract_event::ContractEvent,
    ledger_info::LedgerInfoWithSignatures,
    proof::{definition::LeafCount, position::FrozenSubTreeIterator},
    transaction::{Transaction, TransactionInfo, TransactionOutput, Version},
};
use schemadb::DB;
use std::sync::Arc;

pub fn save_ledger_infos(
    db: Arc<DB>,
    ledger_store: Arc<LedgerStore>,
    ledger_infos: &[LedgerInfoWithSignatures],
) -> Result<()> {
    ensure!(!ledger_infos.is_empty(), "No LedgerInfos to save.");

    let mut cs = ChangeSet::new();
    ledger_infos
        .iter()
        .map(|li| ledger_store.put_ledger_info(li, &mut cs))
        .collect::<Result<Vec<_>>>()?;
    db.write_schemas(cs.batch)?;

    if let Some(li) = ledger_store.get_latest_ledger_info_option() {
        if li.ledger_info().epoch() > ledger_infos.last().unwrap().ledger_info().epoch() {
            // No need to update latest ledger info.
            return Ok(());
        }
    }

    ledger_store.set_latest_ledger_info(ledger_infos.last().unwrap().clone());
    Ok(())
}

pub fn confirm_or_save_frozen_subtrees(
    db: Arc<DB>,
    num_leaves: LeafCount,
    frozen_subtrees: &[HashValue],
) -> Result<()> {
    let mut cs = ChangeSet::new();
    let positions: Vec<_> = FrozenSubTreeIterator::new(num_leaves).collect();

    ensure!(
        positions.len() == frozen_subtrees.len(),
        "Number of frozen subtree roots not expected. Expected: {}, actual: {}",
        positions.len(),
        frozen_subtrees.len(),
    );

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
                cs.batch.put::<TransactionAccumulatorSchema>(p, h)?;
            }
            Ok(())
        })
        .collect::<Result<Vec<_>>>()?;
    db.write_schemas(cs.batch)
}

pub fn save_transactions(
    db: Arc<DB>,
    ledger_store: Arc<LedgerStore>,
    transaction_store: Arc<TransactionStore>,
    event_store: Arc<EventStore>,
    first_version: Version,
    txns: &[Transaction],
    txn_infos: &[TransactionInfo],
    events: &[Vec<ContractEvent>],
) -> Result<()> {
    let mut cs = ChangeSet::new();
    for (idx, txn) in txns.iter().enumerate() {
        transaction_store.put_transaction(first_version + idx as Version, txn, &mut cs)?;
    }
    ledger_store.put_transaction_infos(first_version, txn_infos, &mut cs)?;
    event_store.put_events_multiple_versions(first_version, events, &mut cs)?;

    db.write_schemas(cs.batch)
}

pub fn save_transaction_outputs(
    db: Arc<DB>,
    transaction_store: Arc<TransactionStore>,
    first_version: Version,
    transaction_outputs: Vec<TransactionOutput>,
) -> Result<()> {
    let mut cs = ChangeSet::new();
    for output in transaction_outputs {
        transaction_store.put_write_set(first_version, output.write_set(), &mut cs)?;
    }
    db.write_schemas(cs.batch)
}
