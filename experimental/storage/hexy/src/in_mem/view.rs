// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    hashing::HexyHashBuilder,
    in_mem::{base::HexyBase, overlay::HexyOverlay},
    metrics::TIMER,
    utils::sort_dedup,
    LeafIdx, NodePosition, ARITY,
};
use anyhow::Result;
use aptos_crypto::{hash::HOT_STATE_PLACE_HOLDER_HASH, HashValue};
use aptos_experimental_layered_map::LayeredMap;
use aptos_metrics_core::TimerHelper;
use itertools::Itertools;
use std::sync::{atomic, atomic::Ordering, Arc};

pub struct HexyView {
    base: Arc<HexyBase>,
    overlay: LayeredMap<NodePosition, HashValue>,
}

impl HexyView {
    pub fn new(base: Arc<HexyBase>, overlay: LayeredMap<NodePosition, HashValue>) -> Self {
        Self { base, overlay }
    }

    pub fn new_overlay(&self, leaves: Vec<(LeafIdx, HashValue)>) -> Result<HexyOverlay> {
        // Make sure mutations to the base from last merge is synced to this thread.
        // N.B. assuming not sending work to other threads, otherwise every thread needs a fence.
        atomic::fence(Ordering::Acquire);

        // Now it's safe even if there's ongoing merge, because all the cells being mutated should
        // be in the overlay.
        unsafe { self.unsafe_new_overlay(leaves) }
    }

    unsafe fn unsafe_new_overlay(&self, leaves: Vec<(LeafIdx, HashValue)>) -> Result<HexyOverlay> {
        // Sort and dedup leaves.
        let sorted_leaves = sort_dedup(leaves);

        // Reserve space for all hashes to be updated.
        let estimated_total_hashes =
            Self::estimate_total_hashes(sorted_leaves.len(), self.base.num_levels());
        let mut updates = Vec::with_capacity(estimated_total_hashes);
        let mut this_level_updates = Vec::with_capacity(sorted_leaves.len());

        // Fill in leaf level updates.
        updates.extend(
            sorted_leaves
                .into_iter()
                .map(|(leaf_idx, hash)| (NodePosition::leaf(leaf_idx), hash)),
        );

        // Iteratively calculate updates by level.
        let mut prev_level_begin = 0;
        for height in 1..self.base.num_levels() {
            for (parent_idx_in_level, updated_children) in &updates[prev_level_begin..]
                .iter()
                .chunk_by(|upd| upd.0.parent_index_in_level())
            {
                let mut hasher = HexyHashBuilder::default();

                let parent_position =
                    NodePosition::height_and_index(height, parent_idx_in_level as usize);
                let mut next_child = 0;
                for (updated_child_position, updated_child_hash) in updated_children.into_iter() {
                    let updated_child = updated_child_position.index_in_siblings();
                    for child in next_child..updated_child {
                        unsafe {
                            hasher.add_child(
                                &self.unsafe_expect_hash(parent_position.child(child)),
                            )?;
                        }
                    }
                    // n.b. There's the rule of "16 placeholders hash to the placeholder", and
                    // it seems a waste to detect that.
                    assert_ne!(updated_child_hash, &*HOT_STATE_PLACE_HOLDER_HASH);
                    hasher.add_child(updated_child_hash)?;
                    next_child = updated_child + 1;
                }
                for child in next_child..ARITY {
                    unsafe {
                        hasher.add_child(&self.unsafe_expect_hash(parent_position.child(child)))?;
                    }
                }
                this_level_updates.push((parent_position, hasher.finish()?))
            } // end for children per parent

            prev_level_begin = updates.len();
            updates.append(&mut this_level_updates); // note: this drains `this_level_updates`
        } // end for each level above leaf level

        let root_hash = updates.last().map_or_else(
            || {
                self.overlay
                    .get(&self.base.root_position())
                    .unwrap_or(self.base.root_hash())
            },
            |(pos, hash)| {
                assert_eq!(pos, &self.base.root_position());
                *hash
            },
        );

        let overlay = self.overlay.new_layer(&updates);
        Self::maybe_log_hashing_under_estimation(estimated_total_hashes, updates);

        Ok(HexyOverlay { overlay, root_hash })
    }

    fn maybe_log_hashing_under_estimation(
        estimated_total_hashes: usize,
        updates: Vec<(NodePosition, HashValue)>,
    ) {
        if updates.len() > estimated_total_hashes {
            TIMER.observe_with(
                &["num_hashing_under_est"],
                (updates.len() - estimated_total_hashes) as f64,
            )
        }
    }

    /// To avoid re-allocation (hence copying of `HashValue`s), we estimate the total number of
    /// hashes that will be updated given the size of the update batch and allocate at once.
    fn estimate_total_hashes(num_leaves: usize, height: usize) -> usize {
        if num_leaves == 0 {
            return 0;
        }
        let fully_updated_levels = (num_leaves.ilog(ARITY) + 1) as usize;
        let remaining_levels = height.max(fully_updated_levels) - fully_updated_levels;

        num_leaves * (remaining_levels + 1)
    }

    pub fn num_leaves(&self) -> usize {
        self.base.num_leaves()
    }

    unsafe fn unsafe_expect_hash(&self, position: NodePosition) -> HashValue {
        match self.overlay.get(&position) {
            Some(hash) => hash,
            None => unsafe { self.base.unsafe_expect_hash(position) },
        }
    }
}
