// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Commit-time applier for native-position writes. Dispatches Position
//! entries (drained from `WriteSet`'s native-positions sibling bucket
//! at commit) into the durable [`PositionDb`] and the in-memory
//! [`NativeStateStore`].
//!
//! Invariants:
//! - Position writes are persisted to `position_db.position_value`
//!   AND mirrored into the in-memory store. JMT leaf-update tuples
//!   are returned for the position-tree applier.

#![forbid(unsafe_code)]

use crate::{
    native_state_store::NativeStateStore,
    position_db::{PositionDb, NUM_NATIVE_VALUE_SHARDS},
    position_merkle_db::PositionMerkleDb,
    position_metrics::POSITION_WRITES,
    schema::{
        position_value::PositionValueSchema,
        stale_position_value_index::{StalePositionValueIndex, StalePositionValueIndexSchema},
    },
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
use move_core_types::account_address::AccountAddress;
use std::sync::Arc;

/// Per-leaf JMT update produced by `apply`. Caller drives
/// `JellyfishMerkleTree::put_value_set` with these tuples to compute
/// the new subtree root and the corresponding `TreeUpdateBatch`,
/// then writes the batch to the position merkle DB in the same commit.
#[derive(Clone, Debug)]
pub struct MerkleLeafUpdate {
    pub state_key_hash: HashValue,
    pub state_key: StateKey,
    pub value_hash: Option<HashValue>,
}

/// Bundle of per-subtree JMT leaf updates returned from `apply`.
#[derive(Clone, Debug, Default)]
pub struct NativeMerkleLeafUpdates {
    pub position: Vec<MerkleLeafUpdate>,
}

/// Applies native-mirror writes from a block's `WriteSet` at commit.
/// Owns Arc handles to the durable Position RocksDB instance, the
/// optional merkle DB, and the in-memory state.
pub struct NativeStateCommitter {
    position_db: Arc<PositionDb>,
    #[allow(dead_code)]
    position_merkle_db: Option<Arc<PositionMerkleDb>>,
    in_memory: Arc<NativeStateStore>,
}

impl NativeStateCommitter {
    pub fn new(position_db: Arc<PositionDb>, in_memory: Arc<NativeStateStore>) -> Self {
        Self {
            position_db,
            position_merkle_db: None,
            in_memory,
        }
    }

    pub fn with_position_merkle_db(mut self, merkle_db: Arc<PositionMerkleDb>) -> Self {
        self.position_merkle_db = Some(merkle_db);
        self
    }

    /// Apply a block's Position writes. Returns the JMT leaf updates so
    /// the caller can drive the merkle-tree applier.
    pub fn apply<P>(&self, version: Version, position_writes: P) -> Result<NativeMerkleLeafUpdates>
    where
        P: IntoIterator<Item = (StateKey, WriteOp)>,
    {
        // Per-shard fan-out: each shard accumulates its own SchemaBatch
        // and writes independently so commit-side parallelism scales
        // with the 16-way partition.
        let mut pos_batches: [Option<SchemaBatch>; NUM_NATIVE_VALUE_SHARDS] =
            std::array::from_fn(|_| None);
        let mut in_memory_position_ops: Vec<(
            AccountAddress,
            AccountAddress,
            AccountAddress,
            Option<StateValue>,
        )> = Vec::new();
        let mut position_merkle: Vec<MerkleLeafUpdate> = Vec::new();
        for (state_key, op) in position_writes {
            let (exchange, account, market) = match state_key.inner() {
                StateKeyInner::TradingNative(TradingNativeKey::Position {
                    exchange,
                    account,
                    market,
                }) => (*exchange, *account, *market),
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
            let shard = crate::sharded_kv_db::ShardedKvDb::shard_of_state_key(&state_key);
            let pos_batch = pos_batches[shard].get_or_insert_with(SchemaBatch::new);
            // Emit a stale-index entry pointing at the previous version
            // (if any). The pruner uses this index to garbage-collect
            // superseded rows from `position_value`.
            if let Some(prior_v) = self
                .position_db
                .find_prior_version(state_key_hash, version)
                .map_err(|e| AptosDbError::Other(format!("find_prior_version: {e}")))?
            {
                pos_batch
                    .put::<StalePositionValueIndexSchema>(
                        &StalePositionValueIndex {
                            stale_since_version: version,
                            version: prior_v,
                            state_key_hash,
                        },
                        &(),
                    )
                    .map_err(|e| {
                        AptosDbError::Other(format!("stale_position_value_index put: {e}"))
                    })?;
            }
            pos_batch
                .put::<PositionValueSchema>(&(state_key_hash, version), &maybe_value)
                .map_err(|e| {
                    AptosDbError::Other(format!("position_value batch put failed: {e}"))
                })?;
            let value_hash = maybe_value.as_ref().map(StateValue::hash);
            position_merkle.push(MerkleLeafUpdate {
                state_key_hash,
                state_key: state_key.clone(),
                value_hash,
            });
            in_memory_position_ops.push((exchange, account, market, maybe_value));
        }

        // Hand off to `PositionDb::commit`: writes shards in parallel
        // on the IO pool, stamps per-shard + top-level progress
        // markers. In-memory updates land below, AFTER the durable
        // write, so a crash between the two restarts from the
        // canonical DB.
        self.position_db
            .commit(version, /* metadata_batch = */ None, pos_batches)
            .map_err(|e| AptosDbError::Other(format!("position_db commit failed: {e}")))?;

        // In-memory updates land AFTER the durable write so a crash
        // between the two restarts from the canonical DB.
        for (eid, account, market, value) in in_memory_position_ops {
            self.in_memory
                .apply_position_write(eid, account, market, value);
        }

        Ok(NativeMerkleLeafUpdates {
            position: position_merkle,
        })
    }
}
