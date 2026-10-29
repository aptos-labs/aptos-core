// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Abstracts the stale-node-index CF, progress keys, and pruner name a
//! merkle pruner operates on. The stale-node-index entry type
//! (`StaleNodeIndex`) is shared by every implementor; only the column
//! family, progress keys, and name differ.

use crate::schema::{
    db_metadata::DbMetadataKey, stale_node_index::StaleNodeIndexSchema,
    stale_node_index_cross_epoch::StaleNodeIndexCrossEpochSchema,
};
use aptos_jellyfish_merkle::StaleNodeIndex;
use aptos_schemadb::schema::Schema;

pub(crate) trait MerklePrunerSchema: 'static + Send + Sync {
    /// Stale-node-index CF, keyed by [`StaleNodeIndex`].
    type StaleIndexSchema: Schema<Key = StaleNodeIndex, Value = ()>;

    fn name() -> &'static str;
    fn worker_name() -> &'static str;
    fn shard_progress_key(shard_id: usize) -> DbMetadataKey;
    fn pruner_progress_key() -> DbMetadataKey;
}

/// Main-state cold regular merkle pruner.
pub(crate) enum StateMerkle {}
impl MerklePrunerSchema for StateMerkle {
    type StaleIndexSchema = StaleNodeIndexSchema;

    fn name() -> &'static str {
        "state_merkle_pruner"
    }

    fn worker_name() -> &'static str {
        "state_merkle"
    }

    fn shard_progress_key(shard_id: usize) -> DbMetadataKey {
        DbMetadataKey::StateMerkleShardPrunerProgress(shard_id)
    }

    fn pruner_progress_key() -> DbMetadataKey {
        DbMetadataKey::StateMerklePrunerProgress
    }
}

/// Main-state cold epoch-snapshot merkle pruner.
pub(crate) enum EpochSnapshot {}
impl MerklePrunerSchema for EpochSnapshot {
    type StaleIndexSchema = StaleNodeIndexCrossEpochSchema;

    fn name() -> &'static str {
        "epoch_snapshot_pruner"
    }

    fn worker_name() -> &'static str {
        "state_merkle"
    }

    fn shard_progress_key(shard_id: usize) -> DbMetadataKey {
        DbMetadataKey::EpochEndingStateMerkleShardPrunerProgress(shard_id)
    }

    fn pruner_progress_key() -> DbMetadataKey {
        DbMetadataKey::EpochEndingStateMerklePrunerProgress
    }
}

/// Main-state hot regular merkle pruner. Reuses the cold progress keys, as
/// they live in the hot DB.
pub(crate) enum HotStateMerkle {}
impl MerklePrunerSchema for HotStateMerkle {
    type StaleIndexSchema = StaleNodeIndexSchema;

    fn name() -> &'static str {
        "hot_state_merkle_pruner"
    }

    fn worker_name() -> &'static str {
        "hot_jmt"
    }

    fn shard_progress_key(shard_id: usize) -> DbMetadataKey {
        DbMetadataKey::StateMerkleShardPrunerProgress(shard_id)
    }

    fn pruner_progress_key() -> DbMetadataKey {
        DbMetadataKey::StateMerklePrunerProgress
    }
}

/// Main-state hot epoch-snapshot merkle pruner. Reuses the cold progress keys,
/// as they live in the hot DB.
pub(crate) enum HotEpochSnapshot {}
impl MerklePrunerSchema for HotEpochSnapshot {
    type StaleIndexSchema = StaleNodeIndexCrossEpochSchema;

    fn name() -> &'static str {
        "hot_epoch_snapshot_pruner"
    }

    fn worker_name() -> &'static str {
        "hot_ep"
    }

    fn shard_progress_key(shard_id: usize) -> DbMetadataKey {
        DbMetadataKey::EpochEndingStateMerkleShardPrunerProgress(shard_id)
    }

    fn pruner_progress_key() -> DbMetadataKey {
        DbMetadataKey::EpochEndingStateMerklePrunerProgress
    }
}

/// Native-position regular merkle pruner.
pub(crate) enum PositionStateMerkle {}
impl MerklePrunerSchema for PositionStateMerkle {
    type StaleIndexSchema = StaleNodeIndexSchema;

    fn name() -> &'static str {
        "position_state_merkle_pruner"
    }

    fn worker_name() -> &'static str {
        "position_state_merkle"
    }

    fn shard_progress_key(shard_id: usize) -> DbMetadataKey {
        DbMetadataKey::PositionStateMerkleShardPrunerProgress(shard_id)
    }

    fn pruner_progress_key() -> DbMetadataKey {
        DbMetadataKey::PositionStateMerklePrunerProgress
    }
}

/// Native-position epoch-snapshot merkle pruner.
pub(crate) enum PositionEpochSnapshot {}
impl MerklePrunerSchema for PositionEpochSnapshot {
    type StaleIndexSchema = StaleNodeIndexCrossEpochSchema;

    fn name() -> &'static str {
        "position_epoch_snapshot_pruner"
    }

    fn worker_name() -> &'static str {
        "position_epoch_snapshot"
    }

    fn shard_progress_key(shard_id: usize) -> DbMetadataKey {
        DbMetadataKey::PositionEpochSnapshotShardPrunerProgress(shard_id)
    }

    fn pruner_progress_key() -> DbMetadataKey {
        DbMetadataKey::PositionEpochSnapshotPrunerProgress
    }
}

/// Native-order regular merkle pruner.
// Used once the order store is opened by the write path (PR 6).
#[allow(dead_code)]
pub(crate) enum OrderStateMerkle {}
impl MerklePrunerSchema for OrderStateMerkle {
    type StaleIndexSchema = StaleNodeIndexSchema;

    fn name() -> &'static str {
        "order_state_merkle_pruner"
    }

    fn worker_name() -> &'static str {
        "order_state_merkle"
    }

    fn shard_progress_key(shard_id: usize) -> DbMetadataKey {
        DbMetadataKey::OrderStateMerkleShardPrunerProgress(shard_id)
    }

    fn pruner_progress_key() -> DbMetadataKey {
        DbMetadataKey::OrderStateMerklePrunerProgress
    }
}

/// Native-order epoch-snapshot merkle pruner.
// Used once the order store is opened by the write path (PR 6).
#[allow(dead_code)]
pub(crate) enum OrderEpochSnapshot {}
impl MerklePrunerSchema for OrderEpochSnapshot {
    type StaleIndexSchema = StaleNodeIndexCrossEpochSchema;

    fn name() -> &'static str {
        "order_epoch_snapshot_pruner"
    }

    fn worker_name() -> &'static str {
        "order_epoch_snapshot"
    }

    fn shard_progress_key(shard_id: usize) -> DbMetadataKey {
        DbMetadataKey::OrderEpochSnapshotShardPrunerProgress(shard_id)
    }

    fn pruner_progress_key() -> DbMetadataKey {
        DbMetadataKey::OrderEpochSnapshotPrunerProgress
    }
}
