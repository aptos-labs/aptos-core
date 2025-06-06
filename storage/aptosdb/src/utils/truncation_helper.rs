// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![allow(dead_code)]

use crate::{
    ledger_db::{
        ledger_metadata_db::LedgerMetadataDb, transaction_db::TransactionDb, LedgerDb,
        LedgerDbSchemaBatches,
    },
    schema::{
        db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
        epoch_by_version::EpochByVersionSchema,
        jellyfish_merkle_node::JellyfishMerkleNodeSchema,
        ledger_info::LedgerInfoSchema,
        stale_node_index::StaleNodeIndexSchema,
        stale_node_index_cross_epoch::StaleNodeIndexCrossEpochSchema,
        stale_state_value_index::StaleStateValueIndexSchema,
        stale_state_value_index_by_key_hash::StaleStateValueIndexByKeyHashSchema,
        state_value::StateValueSchema,
        state_value_by_key_hash::StateValueByKeyHashSchema,
        transaction::TransactionSchema,
        transaction_accumulator::TransactionAccumulatorSchema,
        transaction_accumulator_root_hash::TransactionAccumulatorRootHashSchema,
        transaction_info::TransactionInfoSchema,
        transaction_summaries_by_account::TransactionSummariesByAccountSchema,
        version_data::VersionDataSchema,
        write_set::WriteSetSchema,
    },
    state_kv_db::StateKvDb,
    state_merkle_db::StateMerkleDb,
    state_store::MAX_COMMIT_PROGRESS_DIFFERENCE,
    transaction_store::TransactionStore,
    utils::get_progress,
};
use aptos_crypto::hash::CryptoHash;
use aptos_jellyfish_merkle::{node_type::NodeKey, StaleNodeIndex};
use aptos_logger::info;
use aptos_schemadb::{
    batch::SchemaBatch,
    schema::{Schema, SeekKeyCodec},
    DB,
};
use aptos_storage_interface::Result;
use aptos_types::{proof::position::Position, transaction::Version};
use claims::assert_ge;
use rayon::prelude::*;
use status_line::StatusLine;
use std::{
    fmt::{Display, Formatter},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

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

pub(crate) fn truncate_ledger_db(ledger_db: Arc<LedgerDb>, target_version: Version) -> Result<()> {
    let transaction_store = TransactionStore::new(Arc::clone(&ledger_db));

    let start_version = target_version + 1;
    truncate_ledger_db_single_batch(&ledger_db, &transaction_store, start_version)?;
    Ok(())
}

pub(crate) fn truncate_state_kv_db(
    state_kv_db: &StateKvDb,
    current_version: Version,
    target_version: Version,
    batch_size: usize,
) -> Result<()> {
    assert!(batch_size > 0);
    let status = StatusLine::new(Progress::new("Truncating State KV DB", target_version));
    status.set_current_version(current_version);

    let mut current_version = current_version;
    // current_version can be the same with target_version while there is data written to the db before
    // the progress is recorded -- we need to run the truncate for at least one batch
    loop {
        let target_version_for_this_batch = std::cmp::max(
            current_version.saturating_sub(batch_size as Version),
            target_version,
        );
        // By writing the progress first, we still maintain that it is less than or equal to the
        // actual progress per shard, even if it dies in the middle of truncation.
        state_kv_db.write_progress(target_version_for_this_batch)?;
        // the first batch can actually delete more versions than the target batch size because
        // we calculate the start version of this batch assuming the latest data is at
        // `current_version`. Otherwise, we need to seek all shards to determine the
        // actual latest version of data.
        truncate_state_kv_db_shards(state_kv_db, target_version_for_this_batch)?;
        current_version = target_version_for_this_batch;
        status.set_current_version(current_version);

        if current_version <= target_version {
            break;
        }
    }
    assert_eq!(current_version, target_version);
    Ok(())
}

pub(crate) fn truncate_state_kv_db_shards(
    state_kv_db: &StateKvDb,
    target_version: Version,
) -> Result<()> {
    (0..state_kv_db.hack_num_real_shards())
        .into_par_iter()
        .try_for_each(|shard_id| {
            truncate_state_kv_db_single_shard(state_kv_db, shard_id as u8, target_version)
        })
}

pub(crate) fn truncate_state_kv_db_single_shard(
    state_kv_db: &StateKvDb,
    shard_id: u8,
    target_version: Version,
) -> Result<()> {
    let mut batch = SchemaBatch::new();
    delete_state_value_and_index(
        state_kv_db.db_shard(shard_id),
        target_version + 1,
        &mut batch,
        state_kv_db.enabled_sharding(),
    )?;
    state_kv_db.commit_single_shard(target_version, shard_id, batch)
}

pub(crate) fn truncate_state_merkle_db(
    state_merkle_db: &StateMerkleDb,
    target_version: Version,
) -> Result<()> {
    let status = StatusLine::new(Progress::new("Truncating State Merkle DB.", target_version));

    loop {
        let current_version = get_current_version_in_state_merkle_db(state_merkle_db)?
            .expect("Current version of state merkle db must exist.");
        status.set_current_version(current_version);
        assert_ge!(current_version, target_version);
        if current_version == target_version {
            break;
        }

        let version_before = find_closest_node_version_at_or_before(
            state_merkle_db.metadata_db(),
            current_version - 1,
        )?
        .expect("Must exist.");

        let mut top_levels_batch = SchemaBatch::new();

        delete_nodes_and_stale_indices_at_or_after_version(
            state_merkle_db.metadata_db(),
            current_version,
            None, // shard_id
            &mut top_levels_batch,
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
    (0..state_merkle_db.hack_num_real_shards())
        .into_par_iter()
        .try_for_each(|shard_id| {
            truncate_state_merkle_db_single_shard(state_merkle_db, shard_id as u8, target_version)
        })
}

pub(crate) fn truncate_state_merkle_db_single_shard(
    state_merkle_db: &StateMerkleDb,
    shard_id: u8,
    target_version: Version,
) -> Result<()> {
    let mut batch = SchemaBatch::new();
    delete_nodes_and_stale_indices_at_or_after_version(
        state_merkle_db.db_shard(shard_id),
        target_version + 1,
        Some(shard_id),
        &mut batch,
    )?;
    state_merkle_db.db_shard(shard_id).write_schemas(batch)
}

pub(crate) fn find_tree_root_at_or_before(
    ledger_metadata_db: &LedgerMetadataDb,
    state_merkle_db: &StateMerkleDb,
    version: Version,
) -> Result<Option<Version>> {
    if let Some(closest_version) =
        find_closest_node_version_at_or_before(state_merkle_db.metadata_db(), version)?
    {
        if root_exists_at_version(state_merkle_db, closest_version)? {
            return Ok(Some(closest_version));
        }

        // It's possible that it's a partial commit when sharding is not enabled,
        // look again for the previous version:
        if version == 0 {
            return Ok(None);
        }
        if let Some(closest_version) =
            find_closest_node_version_at_or_before(state_merkle_db.metadata_db(), version - 1)?
        {
            if root_exists_at_version(state_merkle_db, closest_version)? {
                return Ok(Some(closest_version));
            }

            // Now we are probably looking at a pruned version in this epoch, look for the previous
            // epoch ending:
            let mut iter = ledger_metadata_db.db().iter::<EpochByVersionSchema>()?;
            iter.seek_for_prev(&version)?;
            if let Some((closest_epoch_version, _)) = iter.next().transpose()? {
                if root_exists_at_version(state_merkle_db, closest_epoch_version)? {
                    return Ok(Some(closest_epoch_version));
                }
            }
        }
    }

    Ok(None)
}

pub(crate) fn root_exists_at_version(
    state_merkle_db: &StateMerkleDb,
    version: Version,
) -> Result<bool> {
    Ok(state_merkle_db
        .metadata_db()
        .get::<JellyfishMerkleNodeSchema>(&NodeKey::new_empty_path(version))?
        .is_some())
}

pub(crate) fn get_current_version_in_state_merkle_db(
    state_merkle_db: &StateMerkleDb,
) -> Result<Option<Version>> {
    find_closest_node_version_at_or_before(state_merkle_db.metadata_db(), Version::MAX)
}

pub(crate) fn get_max_version_in_state_merkle_db(
    state_merkle_db: &StateMerkleDb,
) -> Result<Option<Version>> {
    let mut version = get_current_version_in_state_merkle_db(state_merkle_db)?;
    let num_real_shards = state_merkle_db.hack_num_real_shards() as u8;
    if num_real_shards > 1 {
        for shard_id in 0..num_real_shards {
            let shard_version = find_closest_node_version_at_or_before(
                state_merkle_db.db_shard(shard_id),
                Version::MAX,
            )?;
            if version.is_none() {
                version = shard_version;
            } else if let Some(shard_version) = shard_version {
                if shard_version > version.unwrap() {
                    version = Some(shard_version);
                }
            }
        }
    }
    Ok(version)
}

pub(crate) fn find_closest_node_version_at_or_before(
    db: &DB,
    version: Version,
) -> Result<Option<Version>> {
    let mut iter = db.rev_iter::<JellyfishMerkleNodeSchema>()?;
    iter.seek_for_prev(&NodeKey::new_empty_path(version))?;
    Ok(iter.next().transpose()?.map(|item| item.0.version()))
}

pub(crate) fn num_frozen_nodes_in_accumulator(num_leaves: u64) -> u64 {
    2 * num_leaves - num_leaves.count_ones() as u64
}

fn truncate_transaction_accumulator(
    transaction_accumulator_db: &DB,
    start_version: Version,
    batch: &mut SchemaBatch,
) -> Result<()> {
    let mut iter = transaction_accumulator_db.iter::<TransactionAccumulatorSchema>()?;
    iter.seek_to_last();
    let (position, _) = iter.next().transpose()?.unwrap();
    let num_frozen_nodes = position.to_postorder_index() + 1;
    let num_frozen_nodes_after = num_frozen_nodes_in_accumulator(start_version);
    let mut num_nodes_to_delete = num_frozen_nodes - num_frozen_nodes_after;

    let start_position = Position::from_postorder_index(num_frozen_nodes_after)?;
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
    ledger_db: &LedgerDb,
    transaction_store: &TransactionStore,
    start_version: Version,
) -> Result<()> {
    let mut batch = LedgerDbSchemaBatches::new();

    delete_transaction_index_data(
        ledger_db,
        transaction_store,
        start_version,
        &mut batch.transaction_db_batches,
    )?;
    delete_per_epoch_data(
        &ledger_db.metadata_db_arc(),
        start_version,
        &mut batch.ledger_metadata_db_batches,
    )?;
    delete_per_version_data(ledger_db, start_version, &mut batch)?;

    delete_event_data(ledger_db, start_version, &mut batch.event_db_batches)?;

    truncate_transaction_accumulator(
        ledger_db.transaction_accumulator_db_raw(),
        start_version,
        &mut batch.transaction_accumulator_db_batches,
    )?;

    let mut progress_batch = SchemaBatch::new();
    progress_batch.put::<DbMetadataSchema>(
        &DbMetadataKey::LedgerCommitProgress,
        &DbMetadataValue::Version(start_version - 1),
    )?;
    ledger_db.metadata_db().write_schemas(progress_batch)?;

    ledger_db.write_schemas(batch)
}

fn delete_transaction_index_data(
    ledger_db: &LedgerDb,
    transaction_store: &TransactionStore,
    start_version: Version,
    batch: &mut SchemaBatch,
) -> Result<()> {
    let transactions = ledger_db
        .transaction_db()
        .get_transaction_iter(start_version, MAX_COMMIT_PROGRESS_DIFFERENCE as usize * 2)?
        .collect::<Result<Vec<_>>>()?;
    let num_txns = transactions.len();
    if num_txns > 0 {
        info!(
            start_version = start_version,
            latest_version = start_version + num_txns as u64 - 1,
            "Truncate transaction index data."
        );
        ledger_db
            .transaction_db()
            .prune_transaction_by_hash_indices(transactions.iter().map(|txn| txn.hash()), batch)?;

        let transactions = (start_version..=start_version + transactions.len() as u64 - 1)
            .zip(transactions)
            .collect::<Vec<_>>();
        transaction_store.prune_transaction_by_account(&transactions, batch)?;
    }

    Ok(())
}

fn delete_per_epoch_data(
    ledger_db: &DB,
    start_version: Version,
    batch: &mut SchemaBatch,
) -> Result<()> {
    let mut iter = ledger_db.iter::<LedgerInfoSchema>()?;
    iter.seek_to_last();
    if let Some((epoch, ledger_info)) = iter.next().transpose()? {
        let version = ledger_info.commit_info().version();
        if version >= start_version {
            info!(
                version = version,
                epoch = epoch,
                "Truncate latest epoch data."
            );
            batch.delete::<LedgerInfoSchema>(&epoch)?;
        }
    }

    let mut iter = ledger_db.iter::<EpochByVersionSchema>()?;
    iter.seek(&start_version)?;

    for item in iter {
        let (version, epoch) = item?;
        info!(
            version = version,
            epoch = epoch,
            "Truncate epoch ending data."
        );
        batch.delete::<EpochByVersionSchema>(&version)?;
        batch.delete::<LedgerInfoSchema>(&epoch)?;
    }

    Ok(())
}

fn delete_per_version_data(
    ledger_db: &LedgerDb,
    start_version: Version,
    batch: &mut LedgerDbSchemaBatches,
) -> Result<()> {
    delete_per_version_data_impl::<TransactionAccumulatorRootHashSchema>(
        ledger_db.transaction_accumulator_db_raw(),
        start_version,
        &mut batch.transaction_accumulator_db_batches,
    )?;
    delete_per_version_data_impl::<TransactionInfoSchema>(
        ledger_db.transaction_info_db_raw(),
        start_version,
        &mut batch.transaction_info_db_batches,
    )?;
    delete_transactions_and_transaction_summary_data(
        ledger_db.transaction_db(),
        start_version,
        &mut batch.transaction_db_batches,
    )?;
    delete_per_version_data_impl::<VersionDataSchema>(
        &ledger_db.metadata_db_arc(),
        start_version,
        &mut batch.ledger_metadata_db_batches,
    )?;
    delete_per_version_data_impl::<WriteSetSchema>(
        ledger_db.write_set_db_raw(),
        start_version,
        &mut batch.write_set_db_batches,
    )?;

    Ok(())
}

fn delete_transactions_and_transaction_summary_data(
    transaction_db: &TransactionDb,
    start_version: Version,
    batch: &mut SchemaBatch,
) -> Result<()> {
    let mut iter = transaction_db.db().iter::<TransactionSchema>()?;
    iter.seek_to_last();
    if let Some((latest_version, _)) = iter.next().transpose()? {
        if latest_version >= start_version {
            info!(
                start_version = start_version,
                latest_version = latest_version,
                cf_name = TransactionSchema::COLUMN_FAMILY_NAME,
                "Truncate per version data."
            );
            for version in start_version..=latest_version {
                let transaction = transaction_db.get_transaction(version)?;
                batch.delete::<TransactionSchema>(&version)?;
                if let Some(signed_txn) = transaction.try_as_signed_user_txn() {
                    batch.delete::<TransactionSummariesByAccountSchema>(&(
                        signed_txn.sender(),
                        version,
                    ))?;
                }
            }
        }
    }
    Ok(())
}

fn delete_per_version_data_impl<S>(
    ledger_db: &DB,
    start_version: Version,
    batch: &mut SchemaBatch,
) -> Result<()>
where
    S: Schema<Key = Version>,
{
    let mut iter = ledger_db.iter::<S>()?;
    iter.seek_to_last();
    if let Some((latest_version, _)) = iter.next().transpose()? {
        if latest_version >= start_version {
            info!(
                start_version = start_version,
                latest_version = latest_version,
                cf_name = S::COLUMN_FAMILY_NAME,
                "Truncate per version data."
            );
            for version in start_version..=latest_version {
                batch.delete::<S>(&version)?;
            }
        }
    }
    Ok(())
}

fn delete_event_data(
    ledger_db: &LedgerDb,
    start_version: Version,
    batch: &mut SchemaBatch,
) -> Result<()> {
    if let Some(latest_version) = ledger_db.event_db().latest_version()? {
        if latest_version >= start_version {
            info!(
                start_version = start_version,
                latest_version = latest_version,
                "Truncate event data."
            );
            let num_events_per_version = ledger_db.event_db().prune_event_indices(
                start_version,
                latest_version + 1,
                // Assuming same data will be overwritten into indices, we don't bother to deal
                // with the existence or placement of indices
                // TODO: prune data from internal indices
                None,
            )?;
            ledger_db.event_db().prune_events(
                num_events_per_version,
                start_version,
                latest_version + 1,
                batch,
            )?;
        }
    }
    Ok(())
}

fn delete_state_value_and_index(
    state_kv_db_shard: &DB,
    start_version: Version,
    batch: &mut SchemaBatch,
    enable_sharding: bool,
) -> Result<()> {
    if enable_sharding {
        let mut iter = state_kv_db_shard.iter::<StaleStateValueIndexByKeyHashSchema>()?;
        iter.seek(&start_version)?;

        for item in iter {
            let (index, _) = item?;
            batch.delete::<StaleStateValueIndexByKeyHashSchema>(&index)?;
            batch.delete::<StateValueByKeyHashSchema>(&(
                index.state_key_hash,
                index.stale_since_version,
            ))?;
        }
    } else {
        let mut iter = state_kv_db_shard.iter::<StaleStateValueIndexSchema>()?;
        iter.seek(&start_version)?;

        for item in iter {
            let (index, _) = item?;
            batch.delete::<StaleStateValueIndexSchema>(&index)?;
            batch.delete::<StateValueSchema>(&(index.state_key, index.stale_since_version))?;
        }
    }

    Ok(())
}

fn delete_stale_node_index_at_or_after_version<S>(
    db: &DB,
    version: Version,
    batch: &mut SchemaBatch,
) -> Result<()>
where
    S: Schema<Key = StaleNodeIndex>,
    Version: SeekKeyCodec<S>,
{
    let mut iter = db.iter::<S>()?;
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
    shard_id: Option<u8>,
    batch: &mut SchemaBatch,
) -> Result<()> {
    delete_stale_node_index_at_or_after_version::<StaleNodeIndexSchema>(db, version, batch)?;
    delete_stale_node_index_at_or_after_version::<StaleNodeIndexCrossEpochSchema>(
        db, version, batch,
    )?;

    let mut iter = db.iter::<JellyfishMerkleNodeSchema>()?;
    iter.seek(&NodeKey::new_empty_path(version))?;
    for item in iter {
        let (key, _) = item?;
        batch.delete::<JellyfishMerkleNodeSchema>(&key)?;
    }

    StateMerkleDb::put_progress(version.checked_sub(1), shard_id, batch)
}

struct Progress {
    message: &'static str,
    current_version: AtomicU64,
    target_version: Version,
}

impl Progress {
    pub fn new(message: &'static str, target_version: Version) -> Self {
        Self {
            message,
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
            "{}: current: {}, target: {}",
            self.message,
            self.current_version.load(Ordering::Relaxed),
            self.target_version
        )
    }
}
