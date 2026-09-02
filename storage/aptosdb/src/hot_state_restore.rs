// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Receiver for hot state snapshots restored by fast sync, built on the generic
//! `StateSnapshotRestore` machinery.

use crate::{
    schema::{
        db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
        hot_state_value_by_key_hash::{HotStateEntry, HotStateValueByKeyHashSchema},
    },
    state_kv_db::StateKvDb,
    state_merkle_db::StateMerkleDb,
    state_restore::{
        StateSnapshotRestore, StateSnapshotRestoreMode, StateValueBatch, StateValueWriter,
    },
};
use aptos_crypto::HashValue;
use aptos_db_indexer_schemas::metadata::StateSnapshotProgress;
use aptos_schemadb::batch::{SchemaBatch, WriteBatch};
use aptos_storage_interface::{db_other_bail as bail, AptosDbError, Result, StateSnapshotReceiver};
use aptos_types::{
    state_store::{
        hot_state::HotStateValue, state_key::StateKey, state_storage_usage::StateStorageUsage,
    },
    transaction::Version,
};
use std::sync::Arc;

struct HotStateValueWriter {
    hot_state_kv_db: Arc<StateKvDb>,
}

impl StateValueWriter<StateKey, HotStateValue> for HotStateValueWriter {
    fn write_kv_batch(
        &self,
        version: Version,
        kv_batch: &StateValueBatch<StateKey, Option<HotStateValue>>,
        progress: StateSnapshotProgress,
    ) -> Result<()> {
        let mut sharded_batches = self.hot_state_kv_db.new_sharded_native_batches();
        for ((state_key, _version), hot_value_opt) in kv_batch {
            let hot_value = match hot_value_opt {
                Some(value) => value,
                None => {
                    // `None` would mean an eviction, which a snapshot never carries.
                    bail!(
                        "Hot state snapshot chunk carries an evicted key: {:?}",
                        state_key
                    );
                },
            };
            let hot_since_version = hot_value.hot_since_version();
            // NOTE: since `HotStateValue` does not carry `value_version` for the entry, we use
            // `hot_since_version` as the version for `value_version`, as if the key was last
            // written at `hot_since_version`, even if it was only read. Should be harmless.
            let entry = match hot_value.value_opt() {
                Some(value) => HotStateEntry::Occupied {
                    value: value.clone(),
                    value_version: hot_since_version,
                },
                None => HotStateEntry::Vacant,
            };
            sharded_batches[state_key.get_shard_id()].put::<HotStateValueByKeyHashSchema>(
                &(*state_key.crypto_hash_ref(), hot_since_version),
                &Some(entry),
            )?;
        }

        let mut metadata_batch = SchemaBatch::new();
        metadata_batch.put::<DbMetadataSchema>(
            &DbMetadataKey::HotStateSnapshotKvRestoreProgress(version),
            &DbMetadataValue::StateSnapshotProgress(progress),
        )?;
        self.hot_state_kv_db
            .commit(version, Some(metadata_batch), sharded_batches)
    }

    fn kv_finish(&self, _version: Version, _usage: StateStorageUsage) -> Result<()> {
        // Hot state usage is recomputed from the KV rows when they are loaded.
        Ok(())
    }

    fn get_progress(&self, version: Version) -> Result<Option<StateSnapshotProgress>> {
        Ok(self
            .hot_state_kv_db
            .metadata_db()
            .get::<DbMetadataSchema>(&DbMetadataKey::HotStateSnapshotKvRestoreProgress(version))?
            .map(|v| v.expect_state_snapshot_progress()))
    }
}

/// Returns a snapshot receiver that verifies each chunk's range proof against
/// `expected_root_hash` and writes the hot state JMT and KV.
pub(crate) fn get_hot_state_snapshot_receiver(
    hot_state_kv_db: Arc<StateKvDb>,
    hot_state_merkle_db: &Arc<StateMerkleDb>,
    version: Version,
    expected_root_hash: HashValue,
) -> Result<Box<dyn StateSnapshotReceiver<StateKey, HotStateValue>>> {
    let value_writer = Arc::new(HotStateValueWriter { hot_state_kv_db });
    Ok(Box::new(StateSnapshotRestore::new(
        hot_state_merkle_db,
        &value_writer,
        version,
        expected_root_hash,
        false, /* async_commit */
        StateSnapshotRestoreMode::Default,
    )?))
}
