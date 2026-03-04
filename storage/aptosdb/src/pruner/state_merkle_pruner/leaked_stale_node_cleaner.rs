// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    metrics::{STALE_NODE_CLEANUP, STALE_NODE_CLEANUP_COUNT},
    schema::{
        db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
        jellyfish_merkle_node::JellyfishMerkleNodeSchema,
        stale_node_index::StaleNodeIndexSchema,
        stale_node_index_cross_epoch::StaleNodeIndexCrossEpochSchema,
    },
    state_merkle_db::StateMerkleDb,
    utils::get_progress,
};
use aptos_jellyfish_merkle::StaleNodeIndex;
use aptos_logger::{info, warn};
use aptos_schemadb::{
    batch::SchemaBatch,
    schema::{KeyCodec, Schema},
    DB,
};
use aptos_storage_interface::Result;
use std::sync::Arc;

pub(crate) fn maybe_start_cleaner(state_merkle_db: Arc<StateMerkleDb>, batch_size: usize) {
    if batch_size == 0 {
        info!("Stale node cleanup disabled (batch_size=0).");
        return;
    }

    let metadata_db = state_merkle_db.metadata_db();

    // Check if cleanup has already been done.
    match metadata_db.get::<DbMetadataSchema>(&DbMetadataKey::StaleNodeCleanupDone) {
        Ok(Some(_)) => {
            info!("Stale node cleanup already done, skipping.");
            STALE_NODE_CLEANUP
                .with_label_values(&["overall", "done"])
                .set(1);
            return;
        },
        Err(e) => {
            warn!(
                error = ?e,
                "Failed to read StaleNodeCleanupDone marker, skipping cleanup."
            );
            return;
        },
        Ok(None) => {
            // Marker not found, proceed with cleanup.
        },
    }

    // Read or initialize cleanup progress. On first run, snapshot the current pruner progress
    // and persist it so the same values are used across restarts.
    let (regular_target_version, epoch_target_version) =
        match get_or_init_cleanup_progress(metadata_db) {
            Ok(Some(v)) => v,
            Ok(None) => {
                info!("No pruner progress found, skipping stale node cleanup.");
                return;
            },
            Err(e) => {
                warn!(error = ?e, "Failed to read cleanup progress, skipping stale node cleanup.");
                return;
            },
        };

    STALE_NODE_CLEANUP
        .with_label_values(&["overall", "target_regular_version"])
        .set(regular_target_version as i64);
    STALE_NODE_CLEANUP
        .with_label_values(&["overall", "target_epoch_version"])
        .set(epoch_target_version as i64);

    info!(
        regular_target_version = regular_target_version,
        epoch_target_version = epoch_target_version,
        "Starting leaked stale node cleanup background thread."
    );

    std::thread::Builder::new()
        .name("stale_node_cleaner".to_string())
        .spawn(move || {
            if let Err(e) = run_cleanup(
                &state_merkle_db,
                regular_target_version,
                epoch_target_version,
                batch_size,
            ) {
                warn!(error = ?e, "Stale node cleanup failed.");
            }
        })
        .expect("Failed to spawn stale_node_cleaner thread.");
}

fn run_cleanup(
    state_merkle_db: &StateMerkleDb,
    regular_target_version: u64,
    epoch_target_version: u64,
    batch_size: usize,
) -> Result<()> {
    let num_shards = state_merkle_db.num_shards();

    // Clean shard DBs first.
    for shard_id in 0..num_shards {
        let db = state_merkle_db.db_shard(shard_id);
        let db_name = format!("shard_{shard_id}");
        clean_single_db(
            db,
            regular_target_version,
            epoch_target_version,
            &db_name,
            batch_size,
        )?;
    }

    // Clean metadata DB last. Its done marker also serves as the overall done signal.
    let metadata_db = state_merkle_db.metadata_db();
    clean_single_db(
        metadata_db,
        regular_target_version,
        epoch_target_version,
        "metadata",
        batch_size,
    )?;

    STALE_NODE_CLEANUP
        .with_label_values(&["overall", "done"])
        .set(1);

    info!("Stale node cleanup completed.");

    Ok(())
}

/// Returns the cleanup progress values, or None if no pruner progress exists.
/// On first call, snapshots the current pruner progress and persists it. On subsequent
/// calls (after restart), reads back the persisted values.
fn get_or_init_cleanup_progress(metadata_db: &DB) -> Result<Option<(u64, u64)>> {
    // Check if cleanup progress was already persisted from a previous run.
    let existing_regular =
        get_progress(metadata_db, &DbMetadataKey::StaleNodeCleanupRegularProgress)?;
    let existing_epoch = get_progress(metadata_db, &DbMetadataKey::StaleNodeCleanupEpochProgress)?;

    if let (Some(regular), Some(epoch)) = (existing_regular, existing_epoch) {
        info!(
            regular_target_version = regular,
            epoch_target_version = epoch,
            "Resuming stale node cleanup with persisted progress."
        );
        return Ok(Some((regular, epoch)));
    }

    // First run: snapshot the current pruner progress.
    let regular = match get_progress(metadata_db, &DbMetadataKey::StateMerklePrunerProgress)? {
        Some(v) => v,
        None => return Ok(None),
    };
    let epoch = match get_progress(
        metadata_db,
        &DbMetadataKey::EpochEndingStateMerklePrunerProgress,
    )? {
        Some(v) => v,
        None => return Ok(None),
    };

    // Persist the snapshot.
    let mut batch = SchemaBatch::new();
    batch.put::<DbMetadataSchema>(
        &DbMetadataKey::StaleNodeCleanupRegularProgress,
        &DbMetadataValue::Version(regular),
    )?;
    batch.put::<DbMetadataSchema>(
        &DbMetadataKey::StaleNodeCleanupEpochProgress,
        &DbMetadataValue::Version(epoch),
    )?;
    metadata_db.write_schemas(batch)?;

    info!(
        regular_target_version = regular,
        epoch_target_version = epoch,
        "Persisted stale node cleanup progress for the first time."
    );

    Ok(Some((regular, epoch)))
}

fn clean_single_db(
    db: &DB,
    regular_target_version: u64,
    epoch_target_version: u64,
    db_name: &str,
    batch_size: usize,
) -> Result<()> {
    // Check per-DB done marker.
    if db
        .get::<DbMetadataSchema>(&DbMetadataKey::StaleNodeCleanupDone)?
        .is_some()
    {
        info!(
            db_name = db_name,
            "Stale node cleanup already done, skipping."
        );
        STALE_NODE_CLEANUP
            .with_label_values(&[db_name, "done"])
            .set(1);
        return Ok(());
    }

    let regular_label = format!("{db_name}_regular");
    clean_stale_indices::<StaleNodeIndexSchema>(
        db,
        regular_target_version,
        &regular_label,
        batch_size,
    )?;

    let cross_epoch_label = format!("{db_name}_cross_epoch");
    clean_stale_indices::<StaleNodeIndexCrossEpochSchema>(
        db,
        epoch_target_version,
        &cross_epoch_label,
        batch_size,
    )?;

    // Write per-DB done marker.
    let mut batch = SchemaBatch::new();
    batch.put::<DbMetadataSchema>(
        &DbMetadataKey::StaleNodeCleanupDone,
        &DbMetadataValue::Version(0),
    )?;
    db.write_schemas(batch)?;

    STALE_NODE_CLEANUP
        .with_label_values(&[db_name, "done"])
        .set(1);

    info!(db_name = db_name, "Finished cleaning stale nodes for db.");

    Ok(())
}

fn clean_stale_indices<S>(
    db: &DB,
    target_version: u64,
    label: &str,
    batch_size: usize,
) -> Result<()>
where
    S: Schema<Key = StaleNodeIndex, Value = ()>,
    StaleNodeIndex: KeyCodec<S>,
{
    let counter = STALE_NODE_CLEANUP_COUNT.with_label_values(&[label]);
    let current_version_gauge = STALE_NODE_CLEANUP.with_label_values(&[label, "current_version"]);
    loop {
        let mut batch = SchemaBatch::new();
        let mut count = 0u64;
        let mut last_version = 0u64;

        let mut iter = db.iter::<S>()?;
        iter.seek_to_first();
        for item in iter {
            let (index, _) = item?;
            if index.stale_since_version > target_version {
                break;
            }
            last_version = index.stale_since_version;
            batch.delete::<JellyfishMerkleNodeSchema>(&index.node_key)?;
            batch.delete::<S>(&index)?;
            count += 1;
            if count >= batch_size as u64 {
                break;
            }
        }

        if count == 0 {
            break;
        }

        db.write_schemas(batch)?;
        counter.inc_by(count);
        current_version_gauge.set(last_version as i64);
    }

    Ok(())
}
