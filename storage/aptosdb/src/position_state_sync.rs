// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Producer/receiver for native-position state-sync, built on the generic
//! `StateSnapshotRestore` machinery.

#![forbid(unsafe_code)]

use crate::{
    position_db::PositionDb,
    position_merkle_db::PositionMerkleDb,
    schema::db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
    state_restore::{
        StateSnapshotRestore, StateSnapshotRestoreMode, StateValueBatch, StateValueWriter,
    },
};
use aptos_crypto::{
    hash::{CryptoHash, SPARSE_MERKLE_PLACEHOLDER_HASH},
    HashValue,
};
use aptos_db_indexer_schemas::metadata::StateSnapshotProgress;
use aptos_jellyfish_merkle::JellyfishMerkleTree;
use aptos_schemadb::batch::SchemaBatch;
use aptos_storage_interface::{Result, StateSnapshotReceiver};
use aptos_types::{
    state_store::{
        state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
    },
    transaction::Version,
};
use std::sync::Arc;

/// Number of live position leaves at `version` (0 for an empty tree).
pub fn get_position_state_item_count(
    position_merkle_db: &Arc<PositionMerkleDb>,
    version: Version,
) -> Result<usize> {
    // An empty position tree has no root node; report 0 rather than erroring.
    if position_merkle_db.get_root_hash(version)? == *SPARSE_MERKLE_PLACEHOLDER_HASH {
        return Ok(0);
    }
    let tree = JellyfishMerkleTree::<_, StateKey>::new(position_merkle_db.as_ref());
    tree.get_leaf_count(version)
}

/// `StateValueWriter` for the position value CF, so `StateSnapshotRestore`
/// drives the native-position restore.
pub struct PositionStateValueWriter {
    position_db: Arc<PositionDb>,
}

impl PositionStateValueWriter {
    pub fn new(position_db: &Arc<PositionDb>) -> Self {
        Self {
            position_db: Arc::clone(position_db),
        }
    }
}

impl StateValueWriter<StateKey, StateValue> for PositionStateValueWriter {
    fn write_kv_batch(
        &self,
        version: Version,
        kv_batch: &StateValueBatch<StateKey, Option<StateValue>>,
        progress: StateSnapshotProgress,
    ) -> Result<()> {
        let per_shard =
            PositionDb::shard_position_value_writes(kv_batch.iter().map(
                |((state_key, ver), maybe_value)| (state_key.hash(), *ver, maybe_value.clone()),
            ))?;
        let mut metadata_batch = SchemaBatch::new();
        metadata_batch.put::<DbMetadataSchema>(
            &DbMetadataKey::PositionSnapshotKvRestoreProgress(version),
            &DbMetadataValue::StateSnapshotProgress(progress),
        )?;
        self.position_db
            .commit(version, Some(metadata_batch), per_shard)
    }

    fn kv_finish(&self, _version: Version, _usage: StateStorageUsage) -> Result<()> {
        // Usage is already carried in the persisted StateSnapshotProgress.
        Ok(())
    }

    fn get_progress(&self, version: Version) -> Result<Option<StateSnapshotProgress>> {
        Ok(self
            .position_db
            .metadata_db()
            .get::<DbMetadataSchema>(&DbMetadataKey::PositionSnapshotKvRestoreProgress(version))?
            .map(|v| v.expect_state_snapshot_progress()))
    }
}

/// Returns a snapshot receiver that verifies each chunk's range proof against
/// `expected_root_hash` and writes to the position CF.
pub fn get_position_snapshot_receiver(
    position_db: &Arc<PositionDb>,
    position_merkle_db: &Arc<PositionMerkleDb>,
    version: Version,
    expected_root_hash: HashValue,
) -> Result<Box<dyn StateSnapshotReceiver<StateKey, StateValue>>> {
    let value_writer = Arc::new(PositionStateValueWriter::new(position_db));
    Ok(Box::new(StateSnapshotRestore::new(
        position_merkle_db,
        &value_writer,
        version,
        expected_root_hash,
        false, /* async_commit */
        StateSnapshotRestoreMode::Default,
    )?))
}
