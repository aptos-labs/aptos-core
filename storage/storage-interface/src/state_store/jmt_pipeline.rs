// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Shared primitives for "JMT-sharded pipelines" — subsystems whose
//! durable state is a 16-shard JMT and whose in-memory speculative
//! state is a 16-shard `MapLayer` chain.
//!
//! - [`ShardedJmtState`] — `next_version` + `[MapLayer<HashValue, Slot>; 16]`
//!   keyed by `state_key_hash`, sharded on the leading nibble.

use aptos_crypto::HashValue;
use aptos_experimental_layered_map::MapLayer;
use aptos_types::{state_store::NUM_STATE_SHARDS, transaction::Version};
use arr_macro::arr;
use std::sync::Arc;

/// Versioned in-memory state for a JMT-sharded pipeline. Holds 16
/// `MapLayer` chains keyed by `state_key_hash`, sharded by the
/// leading nibble (matching the JMT shard split). Pipelines declare a
/// type alias over their slot type.
#[derive(Clone, Debug)]
pub struct ShardedJmtState<Slot: Clone + Send + Sync + 'static> {
    next_version: Version,
    shards: Arc<[MapLayer<HashValue, Slot>; NUM_STATE_SHARDS]>,
}

impl<Slot: Clone + Send + Sync + 'static> ShardedJmtState<Slot> {
    /// Pre-genesis empty state — 16 brand-new family roots tagged
    /// with `family` for telemetry / disambiguation.
    pub fn new_empty(family: &'static str) -> Self {
        Self {
            next_version: 0,
            shards: Arc::new(arr![MapLayer::new_family(family); 16]),
        }
    }

    /// Empty state at a specific known version (used when
    /// materializing a state at a JMT snapshot version with no
    /// in-memory layers above it).
    pub fn new_at_version(version: Option<Version>, family: &'static str) -> Self {
        Self {
            next_version: version.map_or(0, |v| v + 1),
            shards: Arc::new(arr![MapLayer::new_family(family); 16]),
        }
    }

    pub fn next_version(&self) -> Version {
        self.next_version
    }

    pub fn version(&self) -> Option<Version> {
        self.next_version.checked_sub(1)
    }

    pub fn shards(&self) -> &[MapLayer<HashValue, Slot>; NUM_STATE_SHARDS] {
        &self.shards
    }

    /// True iff `self` and `rhs` share a chain family.
    pub fn is_descendant_of(&self, rhs: &Self) -> bool {
        self.shards[0].is_descendant_of(&rhs.shards[0])
    }

    /// Add a new layer on top of `self` carrying `updates`. Updates
    /// are bucketed into the 16 shards by `state_key_hash.nibble(0)`.
    pub fn extend(&self, new_version: Version, updates: Vec<(HashValue, Slot)>) -> Self {
        let mut per_shard: [Vec<(HashValue, Slot)>; NUM_STATE_SHARDS] = arr![Vec::new(); 16];
        for (key_hash, slot) in updates {
            per_shard[usize::from(key_hash.nibble(0))].push((key_hash, slot));
        }
        let new_shards: Vec<MapLayer<HashValue, Slot>> = self
            .shards
            .iter()
            .enumerate()
            .map(|(shard_id, base_layer)| {
                let view = base_layer.view_layers_after(base_layer);
                view.new_layer(&per_shard[shard_id])
            })
            .collect();
        let new_shards: [MapLayer<HashValue, Slot>; NUM_STATE_SHARDS] = new_shards
            .try_into()
            .unwrap_or_else(|_| panic!("Known to be 16 shards"));
        Self {
            next_version: new_version + 1,
            shards: Arc::new(new_shards),
        }
    }

    /// Emit per-leaf `(state_key_hash, Slot)` updates that, applied
    /// to `base`, produce `self`. Walks each shard's MapLayer view
    /// between `base.shards[i]` and `self.shards[i]`.
    pub fn make_delta(&self, base: &Self) -> Vec<(HashValue, Slot)> {
        assert!(
            self.is_descendant_of(base),
            "make_delta requires self to descend from base in the same MapLayer family"
        );
        let mut out = Vec::new();
        for shard_id in 0..NUM_STATE_SHARDS {
            let view = self.shards[shard_id].view_layers_after(&base.shards[shard_id]);
            for (key_hash, slot) in view.iter() {
                out.push((key_hash, slot));
            }
        }
        out
    }
}
