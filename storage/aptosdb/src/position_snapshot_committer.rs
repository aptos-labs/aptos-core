// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![forbid(unsafe_code)]

use crate::{
    ledger_db::LedgerDb, position_buffered_state::PositionStateWithSummary,
    position_merkle_batch_committer::PositionMerkleCommit, position_merkle_db::PositionMerkleDb,
};
use aptos_storage_interface::{
    state_store::{leaf_entry::leaf_entry_to_jmt_update, sharded_jmt_state::pre_shard_jmt_updates},
    AptosDbError, Result,
};

/// Rendezvous channel.
pub(crate) const POSITION_BATCH_CHANNEL_SIZE: usize = 0;

pub(crate) struct PositionSnapshotToCommit {
    pub snapshot: PositionStateWithSummary,
}

/// Advances `*last_snapshot` on success.
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
    let base_version = last_snapshot.version();

    let updates = new_state.make_delta(last_snapshot);

    let all_updates = pre_shard_jmt_updates(
        updates
            .iter()
            .map(|(key_hash, slot)| leaf_entry_to_jmt_update(*key_hash, slot)),
    );

    let (root_hash, _leaf_count, top_levels_batch, batches_for_shards) = merkle_db
        .merklize_snapshot(
            base_version,
            version,
            &last_snapshot.summary().global_state_summary,
            &new_state.summary().global_state_summary,
            all_updates,
            previous_epoch_ending_version,
        )
        .map_err(|e| AptosDbError::Other(format!("position JMT merklize_snapshot failed: {e}")))?;

    *last_snapshot = new_state.clone();

    Ok(PositionMerkleCommit {
        version,
        root_hash,
        batch: crate::common::MerkleBatch {
            top_levels_batch,
            batches_for_shards,
        },
        snapshot: new_state,
    })
}
