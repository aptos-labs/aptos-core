// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! First-stage merklize logic for the position async commit pipeline.
//! Mirrors `state_store::state_snapshot_committer::merklize_main_state`.
//!
//! Receives [`PositionSnapshotToCommit`] messages from
//! [`crate::position_buffered_state::PositionBufferedState`], extracts
//! the leaf delta via [`PositionStateWithSummary::make_delta`] against
//! its own `last_snapshot`, runs the 16-shard JMT pipeline through
//! `ShardedJmtMerkleDb::merklize_value_set_for_shard × 16` and
//! `calculate_top_levels`, and hands the resulting
//! [`crate::position_merkle_batch_committer::PositionMerkleCommit`] to
//! the second-stage committer over an internal mpsc channel.
//!
//! Looks up `previous_epoch_ending_version` from `ledger_db` itself
//! (matches `merklize_main_state`) so callers don't have to thread it
//! through.
//!
//! Like `merklize_main_state`, runs the full pipeline regardless of
//! whether the snapshot has any leaf updates. The JMT produces a root
//! at every committed snapshot version; callers that want the position
//! root at an arbitrary version must first resolve to the nearest
//! snapshot via `ShardedJmtMerkleDb::get_state_snapshot_version_before`
//! (inherited via `Deref` on `PositionMerkleDb`).

#![forbid(unsafe_code)]

use crate::{
    ledger_db::LedgerDb, position_buffered_state::PositionStateWithSummary,
    position_merkle_batch_committer::PositionMerkleCommit, position_merkle_db::PositionMerkleDb,
};
use aptos_storage_interface::{
    state_store::jmt_pipeline::{leaf_entry_to_jmt_update, pre_shard_jmt_updates},
    AptosDbError, Result,
};

/// Channel capacity between the two stages of the position commit
/// pipeline. Matches main state's `STATE_BATCH_CHANNEL_SIZE = 0`
/// (rendezvous).
pub(crate) const POSITION_BATCH_CHANNEL_SIZE: usize = 0;

/// Payload sent from the buffered state to the snapshot committer when
/// a snapshot flush fires. Carries the new state; `merklize_position`
/// extracts the delta against the committer's `last_snapshot` and looks
/// up the epoch boundary itself.
pub(crate) struct PositionSnapshotToCommit {
    pub snapshot: PositionStateWithSummary,
}

/// Build a `PositionMerkleCommit` from the snapshot delta against
/// `last_snapshot`. Looks up `previous_epoch_ending_version` from
/// `ledger_db`. Advances `*last_snapshot` on success.
pub(crate) fn merklize_position(
    merkle_db: &PositionMerkleDb,
    ledger_db: &LedgerDb,
    last_snapshot: &mut PositionStateWithSummary,
    snapshot: PositionSnapshotToCommit,
) -> Result<PositionMerkleCommit> {
    let new_state = snapshot.snapshot;
    let version = new_state
        .version()
        .expect("snapshot enqueued for merklize must have a concrete version");
    let previous_epoch_ending_version = ledger_db
        .metadata_db()
        .get_previous_epoch_ending(version)?
        .map(|(v, _e)| v);
    let base_version = version.checked_sub(1);

    let updates = new_state.make_delta(last_snapshot);

    // Pre-shard the flat update stream by leading-nibble of state_key_hash
    // using the shared `LeafEntry`-based helper. Main state and all
    // position-shaped pipelines route their delta through the same
    // helper; pipeline-specific filtering (when any) happens upstream
    // of this call.
    let all_updates = pre_shard_jmt_updates(
        updates
            .iter()
            .map(|(key_hash, slot)| leaf_entry_to_jmt_update(*key_hash, slot)),
    );

    // Hand off to the shared JMT pass — same `merklize_pass` main state
    // calls. Walks 16 shards in parallel on the non-exec CPU pool, feeds
    // each shard the precomputed `new_node_hashes_since` from the SMT,
    // and asserts JMT root == SMT root.
    let (root_hash, _leaf_count, top_levels_batch, batches_for_shards) = merkle_db
        .merklize_pass(
            base_version,
            version,
            &last_snapshot.summary().global_state_summary,
            &new_state.summary().global_state_summary,
            all_updates,
            previous_epoch_ending_version,
        )
        .map_err(|e| AptosDbError::Other(format!("position JMT merklize_pass failed: {e}")))?;

    *last_snapshot = new_state;

    Ok(PositionMerkleCommit {
        version,
        root_hash,
        batch: crate::common::MerkleBatch {
            top_levels_batch,
            batches_for_shards,
        },
    })
}
