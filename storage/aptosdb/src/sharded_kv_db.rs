// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Shared sharded KV DB substrate.
//!
//! Holds the 16-shard RocksDB layout plus a separate metadata DB for
//! commit-progress / pruner-progress bookkeeping. Parallels
//! [`crate::sharded_jmt_merkle_db::ShardedJmtMerkleDb`] in shape — same
//! "shards + metadata DB" layout — but without the JMT-specific cache
//! layers (`VersionedNodeCache` / `LruNodeCache`); a KV DB is a flat
//! per-key store with no internal-node-cache machinery.
//!
//! Both `StateKvDb` (main-state value/hot CFs) and `PositionDb`
//! (position-pipeline value CF) wrap this substrate via `Deref` for
//! the shared layout/routing concerns. Schema-specific reads, writes,
//! and CF-set definitions live on the outer wrappers; this substrate
//! is intentionally schema-agnostic.
//!
//! Domain-specific concerns — how shards/metadata are opened (paths,
//! CFDs, hot/cold split, truncation-on-startup), checkpoint creation,
//! per-pipeline write helpers — are left to the outer wrappers.

#![forbid(unsafe_code)]

use aptos_crypto::HashValue;
use aptos_schemadb::DB;
use aptos_types::state_store::{state_key::StateKey, NUM_STATE_SHARDS};
use std::sync::Arc;

/// Sharded KV DB substrate. See module docs.
#[derive(Debug)]
pub struct ShardedKvDb {
    /// Holds commit-progress / pruner-progress bookkeeping. Schema is
    /// the outer wrapper's concern (e.g. `DbMetadataSchema` keyed by
    /// `DbMetadataKey::*PrunerProgress` variants).
    metadata_db: Arc<DB>,
    /// 16 RocksDB instances, partitioned by `state_key.get_shard_id()`
    /// (leading nibble of the state-key hash — matches JMT convention).
    shards: [Arc<DB>; NUM_STATE_SHARDS],
}

impl ShardedKvDb {
    /// Construct from already-opened metadata + shard DBs. Outer
    /// wrappers handle path resolution / CFD setup.
    pub fn new(metadata_db: Arc<DB>, shards: [Arc<DB>; NUM_STATE_SHARDS]) -> Self {
        Self {
            metadata_db,
            shards,
        }
    }

    /// Borrow the metadata DB.
    pub fn metadata_db(&self) -> &Arc<DB> {
        &self.metadata_db
    }

    /// Borrow shard `idx`. Panics if `idx >= NUM_STATE_SHARDS`.
    pub fn shard(&self, idx: usize) -> &Arc<DB> {
        &self.shards[idx]
    }

    /// All shards — useful for cross-shard fan-out (scan, prune,
    /// rayon-parallelize).
    pub fn shards(&self) -> &[Arc<DB>; NUM_STATE_SHARDS] {
        &self.shards
    }

    /// Shard chosen for `state_key`. Constant-time — uses the
    /// pre-computed hash already cached on the `StateKey`.
    pub fn shard_of_state_key(state_key: &StateKey) -> usize {
        state_key.get_shard_id()
    }

    /// Shard for a precomputed `state_key_hash`. Matches
    /// `StateKey::get_shard_id`: leading nibble of the hash.
    pub fn shard_of_hash(state_key_hash: HashValue) -> usize {
        usize::from(state_key_hash.nibble(0))
    }
}
