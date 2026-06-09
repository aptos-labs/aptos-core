// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![forbid(unsafe_code)]

use crate::{
    position_db::{PositionDb, NUM_NATIVE_VALUE_SHARDS},
    position_metrics::POSITION_WRITES,
    schema::{
        position_value::PositionValueSchema,
        stale_position_value_index::{StalePositionValueIndex, StalePositionValueIndexSchema},
    },
    sharded_kv_db::ShardedKvDb,
};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_schemadb::batch::SchemaBatch;
use aptos_storage_interface::{AptosDbError, Result};
use aptos_types::{
    state_store::{
        state_key::{
            inner::{StateKeyInner, TradingNativeKey},
            StateKey,
        },
        state_value::StateValue,
    },
    transaction::Version,
    write_set::WriteOp,
};
use std::{collections::HashMap, sync::Arc};

#[derive(Clone, Debug)]
pub struct MerkleLeafUpdate {
    pub state_key_hash: HashValue,
    pub state_key: StateKey,
    pub value_hash: Option<HashValue>,
}

#[derive(Clone, Debug, Default)]
pub struct NativeMerkleLeafUpdates {
    pub position: Vec<MerkleLeafUpdate>,
}

/// Per-chunk batches accumulated by `NativeStateCommitter::apply`.
/// Allocate once per chunk, drive `apply` per transaction, then
/// `PositionDb::commit` once at chunk end.
pub type PositionShardedKvBatches = [Option<SchemaBatch>; NUM_NATIVE_VALUE_SHARDS];

/// Per-chunk map of latest version seen for each key hash; used to
/// emit stale-index entries without re-reading the DB.
pub type InChunkPriorVersions = HashMap<HashValue, Version>;

pub fn new_sharded_kv_batches() -> PositionShardedKvBatches {
    std::array::from_fn(|_| None)
}

pub struct NativeStateCommitter {
    position_db: Arc<PositionDb>,
}

impl NativeStateCommitter {
    pub fn new(position_db: Arc<PositionDb>) -> Self {
        Self { position_db }
    }

    /// Accumulate one transaction's Position writes into the
    /// per-chunk batches. The caller commits once per chunk via
    /// `PositionDb::commit`.
    pub fn apply<P>(
        &self,
        version: Version,
        position_writes: P,
        sharded_kv_batches: &mut PositionShardedKvBatches,
        in_chunk_prior: &mut InChunkPriorVersions,
    ) -> Result<NativeMerkleLeafUpdates>
    where
        P: IntoIterator<Item = (StateKey, WriteOp)>,
    {
        let mut position_merkle: Vec<MerkleLeafUpdate> = Vec::new();
        for (state_key, op) in position_writes {
            match state_key.inner() {
                StateKeyInner::TradingNative(TradingNativeKey::Position { .. }) => (),
                other => {
                    return Err(AptosDbError::Other(format!(
                        "position_write_set contained non-Position StateKey: {other:?}"
                    )));
                },
            };
            let maybe_value = op.as_state_value_opt().cloned();
            let kind_label = if maybe_value.is_some() {
                "upsert"
            } else {
                "delete"
            };
            POSITION_WRITES.with_label_values(&[kind_label]).inc();
            let state_key_hash = state_key.hash();
            let shard = ShardedKvDb::shard_of_state_key(&state_key);
            let pos_batch = sharded_kv_batches[shard].get_or_insert_with(SchemaBatch::new);

            // In-chunk map first (same-chunk earlier writes), then DB.
            let prior_v = match in_chunk_prior.get(&state_key_hash) {
                Some(&v) => Some(v),
                None => self
                    .position_db
                    .find_prior_version(state_key_hash, version)
                    .map_err(|e| AptosDbError::Other(format!("find_prior_version: {e}")))?,
            };
            // Always emit a stale-index row — first writes use
            // `NO_PREV_VERSION` and the pruner skips them via
            // `is_first_write()`. Lets truncation iterate this CF
            // alone to reach every kv row.
            pos_batch
                .put::<StalePositionValueIndexSchema>(
                    &StalePositionValueIndex {
                        stale_since_version: version,
                        version: prior_v.unwrap_or(StalePositionValueIndex::NO_PREV_VERSION),
                        state_key_hash,
                    },
                    &(),
                )
                .map_err(|e| AptosDbError::Other(format!("stale_position_value_index put: {e}")))?;
            pos_batch
                .put::<PositionValueSchema>(&(state_key_hash, version), &maybe_value)
                .map_err(|e| {
                    AptosDbError::Other(format!("position_value batch put failed: {e}"))
                })?;

            in_chunk_prior.insert(state_key_hash, version);

            let value_hash = maybe_value.as_ref().map(StateValue::hash);
            position_merkle.push(MerkleLeafUpdate {
                state_key_hash,
                state_key: state_key.clone(),
                value_hash,
            });
        }

        Ok(NativeMerkleLeafUpdates {
            position: position_merkle,
        })
    }
}
