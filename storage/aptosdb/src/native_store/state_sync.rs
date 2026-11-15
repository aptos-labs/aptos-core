// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Producer/receiver for a native store's state sync, built on the generic
//! `StateSnapshotRestore` machinery.

#![forbid(unsafe_code)]

use crate::{
    ledger_db::ledger_metadata_db::LedgerMetadataDb,
    native_store::{
        kv_db::NativeKvDb, merkle_db::NativeMerkleDb, pruner::NativeStorePruner, NativeStoreSpec,
    },
    pruner::PrunerManager,
    schema::db_metadata::{DbMetadataSchema, DbMetadataValue},
    state_restore::{
        StateSnapshotRestore, StateSnapshotRestoreMode, StateValueBatch, StateValueWriter,
    },
    state_value_chunk::{build_value_chunk_proof, value_chunk_with_proof},
};
use aptos_crypto::{
    hash::{CryptoHash, SPARSE_MERKLE_PLACEHOLDER_HASH},
    HashValue,
};
use aptos_db_indexer_schemas::metadata::StateSnapshotProgress;
use aptos_jellyfish_merkle::JellyfishMerkleTree;
use aptos_schemadb::batch::SchemaBatch;
use aptos_storage_interface::{AptosDbError, Result, StateSnapshotReceiver};
use aptos_types::{
    proof::SparseMerkleRangeProof,
    state_store::{
        state_key::StateKey,
        state_storage_usage::StateStorageUsage,
        state_value::{StateValue, StateValueChunkWithProof},
    },
    transaction::Version,
};
use std::sync::Arc;

/// Number of live leaves at `version` (0 for an empty tree).
pub fn get_native_state_item_count<T: NativeStoreSpec>(
    merkle_db: &Arc<NativeMerkleDb<T>>,
    version: Version,
) -> Result<usize> {
    // An empty tree has no root node; report 0 rather than erroring.
    if merkle_db.get_root_hash(version)? == *SPARSE_MERKLE_PLACEHOLDER_HASH {
        return Ok(0);
    }
    let tree = JellyfishMerkleTree::<_, StateKey>::new(merkle_db.as_ref());
    tree.get_leaf_count(version)
}

/// `StateValueWriter` for a native store's value CF, so `StateSnapshotRestore`
/// drives its restore.
pub struct NativeStateValueWriter<T: NativeStoreSpec> {
    kv_db: Arc<NativeKvDb<T>>,
}

impl<T: NativeStoreSpec> NativeStateValueWriter<T> {
    pub fn new(kv_db: &Arc<NativeKvDb<T>>) -> Self {
        Self {
            kv_db: Arc::clone(kv_db),
        }
    }
}

impl<T: NativeStoreSpec> StateValueWriter<StateKey, StateValue> for NativeStateValueWriter<T> {
    fn write_kv_batch(
        &self,
        version: Version,
        kv_batch: &StateValueBatch<StateKey, Option<StateValue>>,
        progress: StateSnapshotProgress,
    ) -> Result<()> {
        let per_shard =
            NativeKvDb::<T>::shard_value_writes(kv_batch.iter().map(
                |((state_key, ver), maybe_value)| (state_key.hash(), *ver, maybe_value.clone()),
            ))?;
        let mut metadata_batch = SchemaBatch::new();
        metadata_batch.put::<DbMetadataSchema>(
            &T::snapshot_kv_restore_progress_key(version),
            &DbMetadataValue::StateSnapshotProgress(progress),
        )?;
        self.kv_db.commit(version, Some(metadata_batch), per_shard)
    }

    fn kv_finish(&self, _version: Version, _usage: StateStorageUsage) -> Result<()> {
        // Usage is already carried in the persisted StateSnapshotProgress.
        Ok(())
    }

    fn get_progress(&self, version: Version) -> Result<Option<StateSnapshotProgress>> {
        Ok(self
            .kv_db
            .metadata_db()
            .get::<DbMetadataSchema>(&T::snapshot_kv_restore_progress_key(version))?
            .map(|v| v.expect_state_snapshot_progress()))
    }
}

/// Returns a snapshot receiver that verifies each chunk's range proof against
/// `expected_root_hash` and writes to the store's value CF.
pub fn get_native_snapshot_receiver<T: NativeStoreSpec>(
    kv_db: &Arc<NativeKvDb<T>>,
    merkle_db: &Arc<NativeMerkleDb<T>>,
    version: Version,
    expected_root_hash: HashValue,
) -> Result<Box<dyn StateSnapshotReceiver<StateKey, StateValue>>> {
    Ok(Box::new(new_native_snapshot_restore(
        kv_db,
        merkle_db,
        version,
        expected_root_hash,
    )?))
}

/// Native stores have no KV-only/tree-only split: the value rows and the
/// tree land together, always in `Default` mode.
pub(crate) fn new_native_snapshot_restore<T: NativeStoreSpec>(
    kv_db: &Arc<NativeKvDb<T>>,
    merkle_db: &Arc<NativeMerkleDb<T>>,
    version: Version,
    expected_root_hash: HashValue,
) -> Result<StateSnapshotRestore<StateKey, StateValue>> {
    StateSnapshotRestore::new(
        Arc::clone(merkle_db),
        Arc::new(NativeStateValueWriter::new(kv_db)),
        version,
        expected_root_hash,
        false, /* async_commit */
        StateSnapshotRestoreMode::Default,
    )
}

/// The read side of a native store's state sync: item counts, value chunks
/// and range proofs at the latest snapshot at or before a version. Snapshots
/// are sparse, so every query first resolves the requested ledger version.
pub(crate) struct NativeStoreReader<'a, T: NativeStoreSpec> {
    pub kv_db: &'a Arc<NativeKvDb<T>>,
    pub merkle_db: &'a Arc<NativeMerkleDb<T>>,
    /// Bounds reads to unpruned versions; `None` skips the check (readonly,
    /// or backup, which serves only retained versions).
    pub pruner: Option<&'a NativeStorePruner<T>>,
    pub ledger_metadata_db: &'a LedgerMetadataDb,
}

impl<T: NativeStoreSpec> NativeStoreReader<'_, T> {
    /// Errors if `version` is pruned: below the regular merkle pruner's
    /// window, unless it is an epoch ending still kept by the
    /// epoch-snapshot pruner.
    pub fn error_if_pruned(&self, version: Version) -> Result<()> {
        let Some(pruner) = self.pruner else {
            return Ok(());
        };
        let min_readable_version = pruner.state_merkle_pruner.get_min_readable_version();
        if version >= min_readable_version {
            return Ok(());
        }
        let min_readable_epoch_snapshot_version =
            pruner.epoch_snapshot_pruner.get_min_readable_version();
        if version >= min_readable_epoch_snapshot_version {
            self.ledger_metadata_db.ensure_epoch_ending(version)
        } else {
            Err(AptosDbError::Other(format!(
                "{} state merkle at version {version} is pruned. {} snapshots are available at \
                 >= {min_readable_version}, epoch snapshots at >= \
                 {min_readable_epoch_snapshot_version}",
                T::NAME,
                T::NAME,
            )))
        }
    }

    fn snapshot_at_or_before(&self, version: Version) -> Result<Option<Version>> {
        self.error_if_pruned(version)?;
        self.merkle_db.latest_snapshot_version_at_or_before(version)
    }

    fn expect_snapshot_at_or_before(&self, version: Version) -> Result<Version> {
        self.snapshot_at_or_before(version)?.ok_or_else(|| {
            AptosDbError::Other(format!(
                "no {} snapshot at or before version {version}",
                T::NAME
            ))
        })
    }

    /// No snapshot at or before `version` means an empty tree.
    pub fn item_count(&self, version: Version) -> Result<usize> {
        match self.snapshot_at_or_before(version)? {
            Some(snapshot_version) => get_native_state_item_count(self.merkle_db, snapshot_version),
            None => Ok(0),
        }
    }

    pub fn value_chunk_with_proof(
        &self,
        version: Version,
        first_index: usize,
        chunk_size: usize,
    ) -> Result<StateValueChunkWithProof> {
        let snapshot_version = self.expect_snapshot_at_or_before(version)?;
        let kv_db = Arc::clone(self.kv_db);
        value_chunk_with_proof(
            Arc::clone(self.merkle_db),
            snapshot_version,
            first_index,
            chunk_size,
            move |key, version| kv_db.expect_value_by_version(key, version),
        )
    }

    /// No snapshot at or before `version` means an empty tree.
    pub fn value_chunk_iter(
        &self,
        version: Version,
        first_index: usize,
        chunk_size: usize,
    ) -> Result<Box<dyn Iterator<Item = Result<(StateKey, StateValue)>> + Send + Sync>> {
        Ok(match self.snapshot_at_or_before(version)? {
            Some(snapshot_version) => Box::new(self.merkle_db.iter_active_leaves_chunk(
                Arc::clone(self.kv_db),
                snapshot_version,
                first_index,
                chunk_size,
            )?),
            None => Box::new(std::iter::empty()),
        })
    }

    /// Proof for the range of leaves ending at `rightmost_key`.
    pub fn range_proof(
        &self,
        rightmost_key: HashValue,
        version: Version,
    ) -> Result<SparseMerkleRangeProof> {
        let snapshot_version = self.snapshot_at_or_before(version)?.ok_or_else(|| {
            AptosDbError::NotFound(format!(
                "no {} snapshot at or before version {version}",
                T::NAME
            ))
        })?;
        self.merkle_db
            .get_range_proof(rightmost_key, snapshot_version)
    }

    pub fn value_chunk_proof(
        &self,
        version: Version,
        first_index: usize,
        state_key_values: Vec<(StateKey, StateValue)>,
    ) -> Result<StateValueChunkWithProof> {
        let snapshot_version = self.expect_snapshot_at_or_before(version)?;
        build_value_chunk_proof(
            self.merkle_db.as_ref(),
            snapshot_version,
            first_index,
            state_key_values,
        )
    }
}
