// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![forbid(unsafe_code)]

use crate::{
    native_store::{
        kv_db::{NativeKvDb, NUM_NATIVE_VALUE_SHARDS},
        NativeStoreSpec,
    },
    schema::{
        native_stale_value_index::{NativeStaleValueIndex, NativeStaleValueIndexSchema},
        native_value::NativeValueSchema,
    },
    sharded_kv_db::ShardedKvDb,
};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_schemadb::batch::SchemaBatch;
use aptos_storage_interface::{AptosDbError, Result};
use aptos_types::{state_store::state_key::StateKey, transaction::Version, write_set::WriteOp};
use std::{collections::HashMap, sync::Arc};

/// Per-chunk batches accumulated by `NativeStateCommitter::apply`.
/// Allocate once per chunk, drive `apply` per transaction, then
/// `NativeKvDb::commit` once at chunk end.
pub type NativeShardedKvBatches = [Option<SchemaBatch>; NUM_NATIVE_VALUE_SHARDS];

/// Per-chunk map of latest version seen for each key hash; used to
/// emit stale-index entries without re-reading the DB.
pub type InChunkPriorVersions = HashMap<HashValue, Version>;

pub fn new_sharded_kv_batches() -> NativeShardedKvBatches {
    std::array::from_fn(|_| None)
}

pub(crate) struct NativeStateCommitter<T: NativeStoreSpec> {
    kv_db: Arc<NativeKvDb<T>>,
}

impl<T: NativeStoreSpec> NativeStateCommitter<T> {
    pub(crate) fn new(kv_db: Arc<NativeKvDb<T>>) -> Self {
        Self { kv_db }
    }

    /// Accumulate one transaction's writes to the store into the per-chunk
    /// kv batches.
    pub(crate) fn apply<'a, P>(
        &self,
        version: Version,
        writes: P,
        sharded_kv_batches: &mut NativeShardedKvBatches,
        in_chunk_prior: &mut InChunkPriorVersions,
    ) -> Result<()>
    where
        P: IntoIterator<Item = (&'a StateKey, &'a WriteOp)>,
    {
        for (state_key, op) in writes {
            let maybe_value = op.as_state_value_opt().cloned();
            let kind_label = if maybe_value.is_some() {
                "upsert"
            } else {
                "delete"
            };
            T::writes_counter().with_label_values(&[kind_label]).inc();
            let state_key_hash = state_key.hash();
            let shard = ShardedKvDb::shard_of_state_key(state_key);
            let batch = sharded_kv_batches[shard].get_or_insert_with(SchemaBatch::new);

            // In-chunk map first (same-chunk earlier writes), then DB.
            let prior_v = match in_chunk_prior.get(&state_key_hash) {
                Some(&v) => Some(v),
                None => self
                    .kv_db
                    .find_prior_version(state_key_hash, version)
                    .map_err(|e| AptosDbError::Other(format!("find_prior_version: {e}")))?,
            };
            // Always emit a stale-index row — first writes use
            // `NO_PREV_VERSION` and the pruner skips them via
            // `is_first_write()`. Lets truncation iterate this CF
            // alone to reach every kv row.
            batch
                .put::<NativeStaleValueIndexSchema>(
                    &NativeStaleValueIndex {
                        stale_since_version: version,
                        version: prior_v.unwrap_or(NativeStaleValueIndex::NO_PREV_VERSION),
                        state_key_hash,
                    },
                    &(),
                )
                .map_err(|e| {
                    AptosDbError::Other(format!("{} stale value index put: {e}", T::NAME))
                })?;
            batch
                .put::<NativeValueSchema>(&(state_key_hash, version), &maybe_value)
                .map_err(|e| AptosDbError::Other(format!("{} value batch put: {e}", T::NAME)))?;

            in_chunk_prior.insert(state_key_hash, version);
        }
        Ok(())
    }
}
