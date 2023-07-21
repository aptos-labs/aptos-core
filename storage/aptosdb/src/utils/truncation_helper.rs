// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![allow(dead_code)]

use crate::{
    db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
    ledger_db::LedgerDb,
    schema::{
        epoch_by_version::EpochByVersionSchema, jellyfish_merkle_node::JellyfishMerkleNodeSchema,
        ledger_info::LedgerInfoSchema, stale_node_index::StaleNodeIndexSchema,
        stale_node_index_cross_epoch::StaleNodeIndexCrossEpochSchema,
        stale_state_value_index::StaleStateValueIndexSchema, state_value::StateValueSchema,
        transaction::TransactionSchema, transaction_accumulator::TransactionAccumulatorSchema,
        transaction_info::TransactionInfoSchema, version_data::VersionDataSchema,
        write_set::WriteSetSchema,
    },
    state_kv_db::StateKvDb,
    state_merkle_db::StateMerkleDb,
    utils::get_progress,
    EventStore, TransactionStore, NUM_STATE_SHARDS,
};
use anyhow::Result;
use aptos_jellyfish_merkle::{node_type::NodeKey, StaleNodeIndex};
use aptos_schemadb::{
    schema::{Schema, SeekKeyCodec},
    ReadOptions, SchemaBatch, DB,
};
use aptos_types::{proof::position::Position, transaction::Version};
use claims::{assert_ge, assert_lt};
use status_line::StatusLine;
use std::{
    fmt::{Display, Formatter},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

pub(crate) fn get_overall_commit_progress(ledger_metadata_db: &DB) -> Result<Option<Version>> {
    get_progress(ledger_metadata_db, &DbMetadataKey::OverallCommitProgress)
}

pub(crate) fn get_ledger_commit_progress(ledger_metadata_db: &DB) -> Result<Option<Version>> {
    get_progress(ledger_metadata_db, &DbMetadataKey::LedgerCommitProgress)
}

pub(crate) fn get_state_kv_commit_progress(state_kv_db: &StateKvDb) -> Result<Option<Version>> {
    get_progress(
        state_kv_db.metadata_db(),
        &DbMetadataKey::StateKvCommitProgress,
    )
}

pub(crate) fn get_state_merkle_commit_progress(
    state_merkle_db: &StateMerkleDb,
) -> Result<Option<Version>> {
    get_progress(
        state_merkle_db.metadata_db(),
        &DbMetadataKey::StateMerkleCommitProgress,
    )
}

pub(crate) fn truncate_ledger_db(
    ledger_db: Arc<LedgerDb>,
    current_version: Version,
    target_version: Version,
    batch_size: usize,
) -> Result<()> {
    let status = StatusLine::new(Progress::new(target_version));

    let event_store = EventStore::new(ledger_db.event_db_arc());
    let transaction_store = TransactionStore::new(Arc::clone(&ledger_db));

    let mut current_version = current_version;
    while current_version > target_version {
        let start_version =
            std::cmp::max(current_version - batch_size as u64 + 1, target_version + 1);
        let end_version = current_version + 1;
        // TODO(grao): Support splitted ledger DBs here.
        truncate_ledger_db_single_batch(
            ledger_db.metadata_db(),
            &event_store,
            &transaction_store,
            start_version,
            end_version,
        )?;
        current_version = start_version - 1;
        status.set_current_version(current_version);
    }
    assert_eq!(current_version, target_version);
    Ok(())
}

pub(crate) fn truncate_state_kv_db(
    state_kv_db: Arc<StateKvDb>,
    current_version: Version,
    target_version: Version,
    batch_size: usize,
) -> Result<()> {
    let status = StatusLine::new(Progress::new(target_version));

    let mut current_version = current_version;
    while current_version > target_version {
        let target_version_for_this_batch =
            std::cmp::max(current_version - batch_size as u64, target_version);
        state_kv_db.write_progress(target_version_for_this_batch)?;
        truncate_state_kv_db_shards(
            &state_kv_db,
            target_version_for_this_batch,
            Some(current_version),
        )?;
        current_version = target_version_for_this_batch;
        status.set_current_version(current_version);
    }
    assert_eq!(current_version, target_version);
    Ok(())
}

pub(crate) fn truncate_state_kv_db_shards(
    state_kv_db: &StateKvDb,
    target_version: Version,
    expected_current_version: Option<Version>,
) -> Result<()> {
    // TODO(grao): Consider do it in parallel.
    for shard_id in 0..NUM_STATE_SHARDS {
        truncate_state_kv_db_single_shard(
            state_kv_db,
            shard_id as u8,
            target_version,
            expected_current_version,
        )?;
    }
    Ok(())
}

pub(crate) fn truncate_state_kv_db_single_shard(
    state_kv_db: &StateKvDb,
    shard_id: u8,
    target_version: Version,
    expected_current_version: Option<Version>,
) -> Result<()> {
    let batch = SchemaBatch::new();
    delete_state_value_and_index(
        state_kv_db.db_shard(shard_id),
        target_version + 1,
        expected_current_version,
        &batch,
    )?;
    state_kv_db.commit_single_shard(target_version, shard_id, batch)
}

pub(crate) fn truncate_state_merkle_db(
    state_merkle_db: &StateMerkleDb,
    target_version: Version,
) -> Result<()> {
    let status = StatusLine::new(Progress::new(target_version));
    loop {
        let current_version = get_current_version_in_state_merkle_db(state_merkle_db)?
            .expect("Current version of state merkle db must exist.");
        status.set_current_version(current_version);
        assert_ge!(current_version, target_version);
        if current_version == target_version {
            break;
        }

        let version_before =
            find_closest_node_version_at_or_before(state_merkle_db, current_version - 1)?
                .expect("Must exist.");

        let top_levels_batch = SchemaBatch::new();

        delete_nodes_and_stale_indices_at_or_after_version(
            state_merkle_db.metadata_db(),
            current_version,
            &top_levels_batch,
        )?;

        state_merkle_db.commit_top_levels(version_before, top_levels_batch)?;

        truncate_state_merkle_db_shards(state_merkle_db, version_before)?;
    }

    Ok(())
}

pub(crate) fn truncate_state_merkle_db_shards(
    state_merkle_db: &StateMerkleDb,
    target_version: Version,
) -> Result<()> {
    // TODO(grao): Consider do it in parallel.
    for shard_id in 0..NUM_STATE_SHARDS {
        truncate_state_merkle_db_single_shard(state_merkle_db, shard_id as u8, target_version)?;
    }

    Ok(())
}

pub(crate) fn truncate_state_merkle_db_single_shard(
    state_merkle_db: &StateMerkleDb,
    shard_id: u8,
    target_version: Version,
) -> Result<()> {
    let batch = SchemaBatch::new();
    delete_nodes_and_stale_indices_at_or_after_version(
        state_merkle_db.db_shard(shard_id),
        target_version + 1,
        &batch,
    )?;
    state_merkle_db.commit_single_shard(target_version, shard_id, batch)
}

pub(crate) fn get_current_version_in_state_merkle_db(
    state_merkle_db: &StateMerkleDb,
) -> Result<Option<Version>> {
    find_closest_node_version_at_or_before(state_merkle_db, u64::max_value())
}

pub(crate) fn find_closest_node_version_at_or_before(
    state_merkle_db: &StateMerkleDb,
    version: Version,
) -> Result<Option<Version>> {
    let mut iter = state_merkle_db
        .metadata_db()
        .rev_iter::<JellyfishMerkleNodeSchema>(Default::default())?;
    iter.seek_for_prev(&NodeKey::new_empty_path(version))?;
    Ok(iter.next().transpose()?.map(|item| item.0.version()))
}

pub(crate) fn num_frozen_nodes_in_accumulator(num_leaves: u64) -> u64 {
    2 * num_leaves - num_leaves.count_ones() as u64
}

fn truncate_transaction_accumulator(
    ledger_db: &DB,
    start_version: Version,
    end_version: Version,
    batch: &SchemaBatch,
) -> Result<()> {
    let num_frozen_nodes = num_frozen_nodes_in_accumulator(end_version);
    let mut iter = ledger_db.iter::<TransactionAccumulatorSchema>(ReadOptions::default())?;
    iter.seek_to_last();
    let (position, _) = iter.next().transpose()?.unwrap();
    assert_eq!(position.to_postorder_index() + 1, num_frozen_nodes);

    let num_frozen_nodes_after_this_batch = num_frozen_nodes_in_accumulator(start_version);

    let mut num_nodes_to_delete = num_frozen_nodes - num_frozen_nodes_after_this_batch;

    let start_position = Position::from_postorder_index(num_frozen_nodes_after_this_batch)?;
    iter.seek(&start_position)?;

    for item in iter {
        let (position, _) = item?;
        batch.delete::<TransactionAccumulatorSchema>(&position)?;
        num_nodes_to_delete -= 1;
    }

    assert_eq!(num_nodes_to_delete, 0);

    Ok(())
}

fn truncate_ledger_db_single_batch(
    ledger_db: &DB,
    event_store: &EventStore,
    transaction_store: &TransactionStore,
    start_version: Version,
    end_version: Version,
) -> Result<()> {
    let batch = SchemaBatch::new();

    delete_transaction_index_data(transaction_store, start_version, end_version, &batch)?;
    delete_per_epoch_data(ledger_db, start_version, end_version, &batch)?;
    delete_per_version_data(start_version, end_version, &batch)?;

    event_store.prune_events(start_version, end_version, &batch)?;

    truncate_transaction_accumulator(ledger_db, start_version, end_version, &batch)?;

    batch.put::<DbMetadataSchema>(
        &DbMetadataKey::LedgerCommitProgress,
        &DbMetadataValue::Version(start_version - 1),
    )?;
    ledger_db.write_schemas(batch)
}

fn delete_transaction_index_data(
    transaction_store: &TransactionStore,
    start_version: Version,
    end_version: Version,
    batch: &SchemaBatch,
) -> Result<()> {
    let transactions = transaction_store
        .get_transaction_iter(start_version, (end_version - start_version) as usize)?
        .collect::<Result<Vec<_>>>()?;
    transaction_store.prune_transaction_by_account(&transactions, batch)?;
    transaction_store.prune_transaction_by_hash(&transactions, batch)?;

    Ok(())
}

fn delete_per_epoch_data(
    ledger_db: &DB,
    start_version: Version,
    end_version: Version,
    batch: &SchemaBatch,
) -> Result<()> {
    let mut iter = ledger_db.iter::<LedgerInfoSchema>(ReadOptions::default())?;
    iter.seek_to_last();
    if let Some((epoch, ledger_info)) = iter.next().transpose()? {
        let version = ledger_info.commit_info().version();
        assert_lt!(version, end_version);
        if version >= start_version {
            batch.delete::<LedgerInfoSchema>(&epoch)?;
        }
    }

    let mut iter = ledger_db.iter::<EpochByVersionSchema>(ReadOptions::default())?;
    iter.seek(&start_version)?;

    for item in iter {
        let (version, epoch) = item?;
        assert_lt!(version, end_version);
        batch.delete::<EpochByVersionSchema>(&version)?;
        batch.delete::<LedgerInfoSchema>(&epoch)?;
    }

    Ok(())
}

fn delete_per_version_data(
    start_version: Version,
    end_version: Version,
    batch: &SchemaBatch,
) -> Result<()> {
    for version in start_version..end_version {
        batch.delete::<TransactionInfoSchema>(&version)?;
        batch.delete::<TransactionSchema>(&version)?;
        batch.delete::<VersionDataSchema>(&version)?;
        batch.delete::<WriteSetSchema>(&version)?;
    }

    Ok(())
}

fn delete_state_value_and_index(
    state_kv_db_shard: &DB,
    start_version: Version,
    expected_current_version: Option<Version>,
    batch: &SchemaBatch,
) -> Result<()> {
    let mut iter = state_kv_db_shard.iter::<StaleStateValueIndexSchema>(ReadOptions::default())?;
    iter.seek(&start_version)?;

    for item in iter {
        let (index, _) = item?;
        if let Some(expected_current_version) = expected_current_version {
            assert_lt!(index.stale_since_version, expected_current_version);
        }
        batch.delete::<StaleStateValueIndexSchema>(&index)?;
        batch.delete::<StateValueSchema>(&(index.state_key, index.stale_since_version))?;
    }

    Ok(())
}

fn delete_stale_node_index_at_or_after_version<S>(
    db: &DB,
    version: Version,
    batch: &SchemaBatch,
) -> Result<()>
where
    S: Schema<Key = StaleNodeIndex>,
    Version: SeekKeyCodec<S>,
{
    let mut iter = db.iter::<S>(ReadOptions::default())?;
    iter.seek(&version)?;
    for item in iter {
        let (index, _) = item?;
        assert_ge!(index.stale_since_version, version);
        batch.delete::<S>(&index)?;
    }

    Ok(())
}

fn delete_nodes_and_stale_indices_at_or_after_version(
    db: &DB,
    version: Version,
    batch: &SchemaBatch,
) -> Result<()> {
    delete_stale_node_index_at_or_after_version::<StaleNodeIndexSchema>(db, version, batch)?;
    delete_stale_node_index_at_or_after_version::<StaleNodeIndexCrossEpochSchema>(
        db, version, batch,
    )?;

    let mut iter = db.iter::<JellyfishMerkleNodeSchema>(ReadOptions::default())?;
    iter.seek(&NodeKey::new_empty_path(version))?;
    for item in iter {
        let (key, _) = item?;
        batch.delete::<JellyfishMerkleNodeSchema>(&key)?;
    }

    Ok(())
}

struct Progress {
    current_version: AtomicU64,
    target_version: Version,
}

impl Progress {
    pub fn new(target_version: Version) -> Self {
        Self {
            current_version: 0.into(),
            target_version,
        }
    }

    pub fn set_current_version(&self, current_version: Version) {
        self.current_version
            .store(current_version, Ordering::Relaxed);
    }
}

impl Display for Progress {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "current: {}, target: {}",
            self.current_version.load(Ordering::Relaxed),
            self.target_version
        )
    }
}
