// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Abstracts the (value CF, stale-index CF, progress keys, name) a
//! state-value pruner operates on. The stale-index entry type
//! (`StaleStateValueByKeyHashIndex`) and value key type
//! (`(HashValue, Version)`) are shared by every implementor; only the
//! column families and progress keys differ.

use crate::{
    position_db::PositionDb,
    pruner::state_kv_pruner::state_kv_pruner_manager::StateKvPrunerManager,
    schema::{
        db_metadata::DbMetadataKey, hot_state_value_by_key_hash::HotStateValueByKeyHashSchema,
        position_value::PositionValueSchema,
        stale_position_value_index::StalePositionValueIndexSchema,
        stale_state_value_index_by_key_hash::StaleStateValueIndexByKeyHashSchema,
        state_value_by_key_hash::StateValueByKeyHashSchema,
    },
};
use aptos_crypto::HashValue;
use aptos_schemadb::schema::Schema;
use aptos_types::{state_store::state_value::StaleStateValueByKeyHashIndex, transaction::Version};

pub(crate) trait StateValuePrunerSchema: 'static + Send + Sync {
    /// Stale-index CF, keyed by [`StaleStateValueByKeyHashIndex`].
    type StaleIndexSchema: Schema<Key = StaleStateValueByKeyHashIndex, Value = ()>;
    /// Value CF, keyed by `(state_key_hash, version)`. The pruner only
    /// ever deletes by key, so the value type is unconstrained.
    type ValueSchema: Schema<Key = (HashValue, Version)>;

    fn name() -> &'static str;
    fn worker_name() -> &'static str;
    fn shard_progress_key(shard_id: usize) -> DbMetadataKey;
    fn pruner_progress_key() -> DbMetadataKey;
}

/// Main-state cold value pruner.
pub(crate) enum ColdStateKv {}
impl StateValuePrunerSchema for ColdStateKv {
    type StaleIndexSchema = StaleStateValueIndexByKeyHashSchema;
    type ValueSchema = StateValueByKeyHashSchema;

    fn name() -> &'static str {
        "state_kv_pruner"
    }

    fn worker_name() -> &'static str {
        "state_kv"
    }

    fn shard_progress_key(shard_id: usize) -> DbMetadataKey {
        DbMetadataKey::StateKvShardPrunerProgress(shard_id)
    }

    fn pruner_progress_key() -> DbMetadataKey {
        DbMetadataKey::StateKvPrunerProgress
    }
}

/// Main-state hot value pruner.
pub(crate) enum HotStateKv {}
impl StateValuePrunerSchema for HotStateKv {
    type StaleIndexSchema = StaleStateValueIndexByKeyHashSchema;
    type ValueSchema = HotStateValueByKeyHashSchema;

    fn name() -> &'static str {
        "hot_state_kv_pruner"
    }

    fn worker_name() -> &'static str {
        "hot_state_kv"
    }

    fn shard_progress_key(shard_id: usize) -> DbMetadataKey {
        DbMetadataKey::StateKvShardPrunerProgress(shard_id)
    }

    fn pruner_progress_key() -> DbMetadataKey {
        DbMetadataKey::StateKvPrunerProgress
    }
}

/// Native-position value pruner.
pub(crate) enum PositionValue {}
impl StateValuePrunerSchema for PositionValue {
    type StaleIndexSchema = StalePositionValueIndexSchema;
    type ValueSchema = PositionValueSchema;

    fn name() -> &'static str {
        "position_value_pruner"
    }

    fn worker_name() -> &'static str {
        "position_value"
    }

    fn shard_progress_key(shard_id: usize) -> DbMetadataKey {
        DbMetadataKey::PositionValueShardPrunerProgress(shard_id)
    }

    fn pruner_progress_key() -> DbMetadataKey {
        DbMetadataKey::PositionValuePrunerProgress
    }
}

/// The native-position value pruner manager.
pub(crate) type PositionValuePrunerManager = StateKvPrunerManager<PositionValue, PositionDb>;
