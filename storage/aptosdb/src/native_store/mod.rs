// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Storage for a trading-native store: a sharded value DB plus a sharded
//! JMT, with its commit pipeline, pruners, truncation and snapshot
//! restore. Each store lives in its own DBs, so all stores share the value,
//! stale-index and JMT column families; a [`NativeStoreSpec`] names the
//! store's folders, metadata keys, pruners and threads. [`PositionStore`]
//! is the native positions.

#![forbid(unsafe_code)]

pub(crate) mod buffered_state;
pub(crate) mod kv_db;
pub(crate) mod merkle_batch_committer;
pub(crate) mod merkle_db;
pub(crate) mod pruner;
pub(crate) mod snapshot_committer;
pub(crate) mod state_store;
pub(crate) mod state_sync;

use crate::{
    common::{POSITION_DB_FOLDER_NAME, POSITION_MERKLE_DB_FOLDER_NAME},
    pruner::{
        MerklePrunerSchema, PositionEpochSnapshot, PositionStateMerkle, PositionValue,
        StateValuePrunerSchema,
    },
    schema::{
        db_metadata::DbMetadataKey, position_value::PositionValueSchema,
        stale_node_index::StaleNodeIndexSchema,
        stale_node_index_cross_epoch::StaleNodeIndexCrossEpochSchema,
        stale_position_value_index::StalePositionValueIndexSchema,
    },
};
use aptos_config::config::StorageDirPaths;
use aptos_types::transaction::Version;
use std::path::PathBuf;

pub(crate) trait NativeStoreSpec: 'static + Send + Sync {
    type ValuePruner: StateValuePrunerSchema<
        ValueSchema = PositionValueSchema,
        StaleIndexSchema = StalePositionValueIndexSchema,
    >;
    type StateMerklePruner: MerklePrunerSchema<StaleIndexSchema = StaleNodeIndexSchema>;
    type EpochSnapshotPruner: MerklePrunerSchema<StaleIndexSchema = StaleNodeIndexCrossEpochSchema>;

    /// Store name in errors, logs and the JMT metrics tag.
    const NAME: &'static str;
    const KV_DB_FOLDER_NAME: &'static str;
    const MERKLE_DB_FOLDER_NAME: &'static str;
    const KV_METADATA_DB_NAME: &'static str;
    const KV_SHARD_DB_NAME_PREFIX: &'static str;
    const MERKLE_METADATA_DB_NAME: &'static str;
    const MERKLE_SHARD_DB_NAME_PREFIX: &'static str;
    const SNAPSHOT_THREAD_NAME: &'static str;
    const BATCH_THREAD_NAME: &'static str;

    fn kv_db_metadata_root_path(db_paths: &StorageDirPaths) -> &PathBuf;
    fn kv_db_shard_root_path(db_paths: &StorageDirPaths, shard_id: usize) -> &PathBuf;
    fn merkle_db_metadata_root_path(db_paths: &StorageDirPaths) -> &PathBuf;
    fn merkle_db_shard_root_path(db_paths: &StorageDirPaths, shard_id: usize) -> &PathBuf;

    fn commit_progress_key() -> DbMetadataKey;
    fn shard_commit_progress_key(shard_id: usize) -> DbMetadataKey;
    fn snapshot_kv_restore_progress_key(version: Version) -> DbMetadataKey;
}

/// Native positions, keyed by `(exchange, account, market)`.
pub(crate) enum PositionStore {}

impl NativeStoreSpec for PositionStore {
    type EpochSnapshotPruner = PositionEpochSnapshot;
    type StateMerklePruner = PositionStateMerkle;
    type ValuePruner = PositionValue;

    const BATCH_THREAD_NAME: &'static str = "position_merkle_batch_committer";
    const KV_DB_FOLDER_NAME: &'static str = POSITION_DB_FOLDER_NAME;
    const KV_METADATA_DB_NAME: &'static str = "position_db_metadata";
    const KV_SHARD_DB_NAME_PREFIX: &'static str = "position_db_shard";
    const MERKLE_DB_FOLDER_NAME: &'static str = POSITION_MERKLE_DB_FOLDER_NAME;
    const MERKLE_METADATA_DB_NAME: &'static str = "position_merkle_db_metadata";
    const MERKLE_SHARD_DB_NAME_PREFIX: &'static str = "position_merkle_db_shard";
    const NAME: &'static str = "position";
    const SNAPSHOT_THREAD_NAME: &'static str = "position_snapshot_committer";

    fn kv_db_metadata_root_path(db_paths: &StorageDirPaths) -> &PathBuf {
        db_paths.position_db_metadata_root_path()
    }

    fn kv_db_shard_root_path(db_paths: &StorageDirPaths, shard_id: usize) -> &PathBuf {
        db_paths.position_db_shard_root_path(shard_id)
    }

    fn merkle_db_metadata_root_path(db_paths: &StorageDirPaths) -> &PathBuf {
        db_paths.position_merkle_db_metadata_root_path()
    }

    fn merkle_db_shard_root_path(db_paths: &StorageDirPaths, shard_id: usize) -> &PathBuf {
        db_paths.position_merkle_db_shard_root_path(shard_id)
    }

    fn commit_progress_key() -> DbMetadataKey {
        DbMetadataKey::PositionCommitProgress
    }

    fn shard_commit_progress_key(shard_id: usize) -> DbMetadataKey {
        DbMetadataKey::PositionShardCommitProgress(shard_id)
    }

    fn snapshot_kv_restore_progress_key(version: Version) -> DbMetadataKey {
        DbMetadataKey::PositionSnapshotKvRestoreProgress(version)
    }
}
