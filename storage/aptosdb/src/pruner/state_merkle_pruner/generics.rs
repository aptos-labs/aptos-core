// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Abstracts the stale-node-index CF, progress keys, and pruner name a
//! merkle pruner operates on. The stale-node-index entry type
//! (`StaleNodeIndex`) is shared by every implementor; only the column
//! family, progress keys, and name differ.

use crate::{
    position_merkle_db::PositionMerkleDb,
    pruner::state_merkle_pruner::state_merkle_pruner_manager::StateMerklePrunerManager,
    schema::{
        db_metadata::DbMetadataKey, stale_node_index::StaleNodeIndexSchema,
        stale_node_index_cross_epoch::StaleNodeIndexCrossEpochSchema,
    },
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

/// Main-state regular merkle pruner (cold and hot share this identity,
/// distinguished by their DB).
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

/// Main-state epoch-snapshot merkle pruner.
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

pub(crate) type PositionStateMerklePrunerManager =
    StateMerklePrunerManager<PositionStateMerkle, PositionMerkleDb>;
pub(crate) type PositionEpochSnapshotPrunerManager =
    StateMerklePrunerManager<PositionEpochSnapshot, PositionMerkleDb>;
