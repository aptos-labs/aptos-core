// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! This file defines the state snapshot committer running in background thread within StateStore.

use crate::{
    common::MerkleBatch,
    metrics::OTHER_TIMERS_SECONDS,
    state_store::{state_merkle_batch_committer::StateMerkleCommit, StateDb},
    versioned_node_cache::VersionedNodeCache,
};
use aptos_crypto::hash::CryptoHash;
use aptos_metrics_core::TimerHelper;
use aptos_storage_interface::state_store::{
    jmt_pipeline::leaf_entry_to_jmt_update, state_with_summary::StateWithSummary,
    HotStateShardUpdates,
};
use aptos_types::state_store::NUM_STATE_SHARDS;
use static_assertions::const_assert;

/// Payload sent to the state snapshot committer thread.
pub(crate) struct SnapshotToCommit {
    pub snapshot: StateWithSummary,
    pub hot_state_updates: [HotStateShardUpdates; NUM_STATE_SHARDS],
}

/// Channel capacity between the two stages of the main-state commit
/// pipeline. Rendezvous channel — must stay below the JMT node cache
/// depth.
pub(crate) const STATE_BATCH_CHANNEL_SIZE: usize = 0;
const_assert!(STATE_BATCH_CHANNEL_SIZE < VersionedNodeCache::NUM_VERSIONS_TO_CACHE);

/// Compute the `StateMerkleCommit` for a snapshot. Called from the
/// `SnapshotCommitter::run` closure in `BufferedState::new_at_snapshot`.
/// Advances `*last_snapshot` on success.
pub(crate) fn merklize_main_state(
    state_db: &StateDb,
    last_snapshot: &mut StateWithSummary,
    SnapshotToCommit {
        snapshot,
        hot_state_updates,
    }: SnapshotToCommit,
) -> StateMerkleCommit {
    let version = snapshot.version().expect("Cannot be empty");
    let base_version = last_snapshot.version();
    let previous_epoch_ending_version = state_db
        .ledger_db
        .metadata_db()
        .get_previous_epoch_ending(version)
        .unwrap()
        .map(|(v, _e)| v);
    let min_version = last_snapshot.next_version();

    // Element format: (key_hash, Option<(value_hash, key)>). Routes
    // through the shared `LeafEntry`-based extractor — same shape
    // position-shaped pipelines use. Main state's per-slot filter
    // (`passes_jmt_filter`, which checks `value_version`/
    // `hot_since_version >= min_version`) skips entries that haven't
    // changed since the last snapshot; position-shaped pipelines'
    // default `passes_jmt_filter` returns `true`.
    let all_updates: Vec<_> = snapshot
        .make_delta(last_snapshot)
        .shards
        .iter()
        .map(|updates| {
            let _timer = OTHER_TIMERS_SECONDS.timer_with(&["hash_jmt_updates"]);
            updates
                .iter()
                .filter(|(_key_hash, slot)| slot.passes_jmt_filter(min_version))
                .map(|(key_hash, slot)| leaf_entry_to_jmt_update(key_hash, &slot))
                .collect::<Vec<_>>()
        })
        .collect();

    let hot_updates: Vec<_> = hot_state_updates
        .into_iter()
        .map(|shard| {
            let _timer = OTHER_TIMERS_SECONDS.timer_with(&["hash_hot_jmt_updates"]);
            shard
                .insertions
                .into_iter()
                .map(|(key_hash, op)| (key_hash, Some((op.value.hash(), op.state_key))))
                .chain(shard.evictions.into_keys().map(|key_hash| (key_hash, None)))
                .collect()
        })
        .collect();

    // TODO(HotState): for now we use `is_descendant_of` to determine if hot state
    // summary is computed at all. When it's not enabled everything is
    // `SparseMerkleTree::new_empty()`.
    let hot_pair = snapshot
        .summary()
        .hot_state_summary
        .as_ref()
        .zip(last_snapshot.summary().hot_state_summary.as_ref());
    let hot_state_merkle_batch_opt = match hot_pair {
        Some((snap_hot, last_hot)) if snap_hot.is_descendant_of(last_hot) => {
            state_db.hot_state_merkle_db.as_ref().map(|db| {
                let (_root, _leaf_count, top_levels_batch, batches_for_shards) = db
                    .merklize_pass(
                        base_version,
                        version,
                        last_hot,
                        snap_hot,
                        hot_updates.try_into().expect("Must be 16 shards."),
                        previous_epoch_ending_version,
                    )
                    .expect("Failed to compute JMT commit batch for hot state.");
                MerkleBatch {
                    top_levels_batch,
                    batches_for_shards,
                }
            })
        },
        // TODO(HotState): this means that the relevant code path isn't enabled yet.
        _ => None,
    };
    let (_root, leaf_count, top_levels_batch, batches_for_shards) = state_db
        .state_merkle_db
        .merklize_pass(
            base_version,
            version,
            &last_snapshot.summary().global_state_summary,
            &snapshot.summary().global_state_summary,
            all_updates.try_into().expect("Must be 16 shards."),
            previous_epoch_ending_version,
        )
        .expect("Failed to compute JMT commit batch.");
    let state_merkle_batch = MerkleBatch {
        top_levels_batch,
        batches_for_shards,
    };
    let usage = snapshot.state().usage();
    if !usage.is_untracked() {
        assert_eq!(
            leaf_count,
            usage.items(),
            "Num of state items mismatch: jmt: {}, state: {}",
            leaf_count,
            usage.items(),
        );
    }

    *last_snapshot = snapshot.clone();

    StateMerkleCommit {
        snapshot,
        hot_batch: hot_state_merkle_batch_opt,
        cold_batch: state_merkle_batch,
    }
}
