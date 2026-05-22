// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Shared primitives for "JMT-sharded pipelines" — subsystems whose
//! durable state is a 16-shard JMT and whose in-memory speculative
//! state is a 16-shard `MapLayer` chain plus a scratchpad
//! `SparseMerkleTree` summary (provided via
//! [`crate::state_store::state_summary::StateSummary`]).
//!
//! - [`ShardedJmtState`] — `next_version` + `[MapLayer<HashValue, Slot>; 16]`
//!   keyed by `state_key_hash`, sharded on the leading nibble.
//! - [`LeafEntry`] — minimum read-shape implemented by any
//!   leaf-style slot ([`LeafSlot`] for position-shaped pipelines,
//!   `aptos_types::state_store::state_slot::StateSlot` for main
//!   state's hot-aware case). Consumers (snapshot committer, JMT
//!   pre-shard helper) read slot data through this trait so the same
//!   helper code works for both pipelines.
//! - [`LeafSlot`] — generic concrete slot for position-shaped
//!   pipelines: `{ state_key, value_hash, value }` parameterized over
//!   the value-payload type `V` (use `V = ()` when the slot doesn't
//!   carry the value in-line; `V = SomeValue` once block-STM reads
//!   land for the pipeline).

use crate::{
    state_store::{state_summary::StateSummary, state_with_summary::StateAndSummary},
    AptosDbError, Result,
};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_experimental_layered_map::MapLayer;
use aptos_scratchpad::ProofRead;
use aptos_types::{
    state_store::{
        state_key::StateKey, state_slot::StateSlot, state_value::StateValue, NUM_STATE_SHARDS,
    },
    transaction::Version,
};
use arr_macro::arr;
use std::sync::Arc;

/// Read-shape implemented by any "leaf-style" slot that can show up
/// in a [`ShardedJmtState`] (or in main state's `State.shards`).
///
/// Consumers that need to walk slot data — chiefly the JMT pre-shard
/// helper that builds `(key_hash, Option<(value_hash, state_key)>)`
/// tuples — read through this trait so they work uniformly across
/// pipelines.
///
/// `state_key()` returns `Option<&StateKey>` because main state's
/// `StateSlot` can carry `None` (slots loaded from the hot KV DB
/// store only the key hash). Position-shaped slots always have a
/// concrete key.
pub trait LeafEntry: Clone {
    /// The in-slot value payload type. `()` for pipelines that don't
    /// carry the value in-line (current position, future order/
    /// collateral pre-block-STM-integration). `StateValue` for main
    /// state. `SomePositionValue` for post-block-STM position.
    type Value;

    fn state_key(&self) -> Option<&StateKey>;
    fn value(&self) -> Option<&Self::Value>;
    fn value_hash(&self) -> Option<HashValue>;

    /// Should this slot contribute a JMT-pass entry for a snapshot
    /// whose previous snapshot was at `min_version`?
    ///
    /// The default impl returns `true` — appropriate for
    /// position-shaped pipelines whose `make_delta` already returns
    /// only entries that changed in the current snapshot.
    ///
    /// Main state's `StateSlot` overrides this to filter slots that
    /// haven't changed since `min_version` (only LRU pointer updates,
    /// stale hot evictions, etc.).
    fn passes_jmt_filter(&self, _min_version: Version) -> bool {
        true
    }
}

/// Generic concrete slot for position-shaped pipelines. Carries the
/// in-memory leaf data for one entry: the original `state_key` (so
/// the JMT row can be written with `(key_hash, key, value_hash)`),
/// the precomputed `value_hash` (so the JMT pass doesn't have to
/// recompute), and an optional `value` payload (used when the
/// pipeline reads values from the slot directly — e.g. once block-STM
/// integration lands; `V = ()` until then, with `value` always
/// `None`).
///
/// Implements [`LeafEntry`] so generic JMT-pipeline helpers work over
/// both `LeafSlot<V>` and main state's `StateSlot`.
#[derive(Clone, Debug)]
pub struct LeafSlot<V: Clone + Send + Sync + 'static = ()> {
    pub state_key: StateKey,
    pub value_hash: Option<HashValue>,
    pub value: Option<V>,
}

impl<V: Clone + Send + Sync + 'static> LeafEntry for LeafSlot<V> {
    type Value = V;

    fn state_key(&self) -> Option<&StateKey> {
        Some(&self.state_key)
    }

    fn value(&self) -> Option<&V> {
        self.value.as_ref()
    }

    fn value_hash(&self) -> Option<HashValue> {
        self.value_hash
    }
}

/// Main state's hot-aware [`StateSlot`] also presents the
/// [`LeafEntry`] read-shape — `state_key()` is the inner
/// `Option<StateKey>` (None when loaded from the hot KV DB which
/// stores only key hashes), `value()` returns the in-slot
/// `StateValue` for occupied variants, and `value_hash()` computes
/// the hash on the fly from that value. The hot-specific bits
/// (`hot_since_version`, `lru_info`) stay accessed via the concrete
/// `StateSlot` type, not through this trait.
impl LeafEntry for StateSlot {
    type Value = StateValue;

    fn state_key(&self) -> Option<&StateKey> {
        StateSlot::state_key(self)
    }

    fn value(&self) -> Option<&StateValue> {
        self.as_state_value_opt()
    }

    fn value_hash(&self) -> Option<HashValue> {
        self.as_state_value_opt().map(CryptoHash::hash)
    }

    fn passes_jmt_filter(&self, min_version: Version) -> bool {
        StateSlot::passes_jmt_filter(self, min_version)
    }
}

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

/// JMT-sharded pipeline `{ state, summary }` pair helpers. Pipelines
/// compose a [`crate::state_store::state_with_summary::StateAndSummary`]
/// directly over a [`ShardedJmtState`] slot type; this inherent impl
/// provides pipeline-shaped operations (`new_empty(family)`,
/// `extend`, `make_delta`, version accessors).
///
/// For position-shaped pipelines the summary's `hot_state_summary`
/// is `None` and only `global_state_summary` is advanced.
impl<Slot: Clone + Send + Sync + LeafEntry + 'static> StateAndSummary<ShardedJmtState<Slot>> {
    /// Pre-genesis empty pair for a pipeline with no hot-state
    /// companion (position-shaped).
    pub fn new_empty(family: &'static str) -> Self {
        Self::new(
            ShardedJmtState::new_empty(family),
            StateSummary::new_empty_global_only(),
        )
    }

    pub fn version(&self) -> Option<Version> {
        self.summary().version()
    }

    pub fn next_version(&self) -> Version {
        self.summary().next_version()
    }

    pub fn root_hash(&self) -> HashValue {
        self.summary().root_hash()
    }

    /// Build a new pair at `new_version` advancing both the SMT chain
    /// (for proofs / JMT node-hash precomputation) and the MapLayer
    /// chain (for `make_delta`). SMT leaf hashes are derived from
    /// `updates` via [`LeafEntry::value_hash`]. Only the
    /// `global_state_summary` half is advanced; position-shaped
    /// pipelines have no hot companion.
    pub fn extend(
        &self,
        new_version: Version,
        updates: Vec<(HashValue, Slot)>,
        proof_reader: &impl ProofRead,
    ) -> Result<Self> {
        let smt_updates: Vec<(HashValue, Option<HashValue>)> =
            updates.iter().map(|(k, s)| (*k, s.value_hash())).collect();
        let new_global = if smt_updates.is_empty() {
            self.summary().global_state_summary.clone()
        } else {
            self.summary()
                .global_state_summary
                .freeze(&self.summary().global_state_summary)
                .batch_update(smt_updates.iter(), proof_reader)
                .map_err(|e| {
                    AptosDbError::Other(format!("scratchpad SMT batch_update failed: {e:?}"))
                })?
                .unfreeze()
        };
        let new_summary = StateSummary::new_global_only(new_version, new_global);
        let new_state = self.state().extend(new_version, updates);
        Ok(Self::new(new_state, new_summary))
    }

    /// Per-leaf updates that produced `self` from `base`. Mirrors
    /// `ShardedJmtState::make_delta`.
    pub fn make_delta(&self, base: &Self) -> Vec<(HashValue, Slot)> {
        self.state().make_delta(base.state())
    }
}

/// Pre-shard a stream of pre-filtered JMT-input tuples by the
/// leading nibble of `key_hash` into the `[Vec<...>; NUM_STATE_SHARDS]`
/// shape `ShardedJmtMerkleDb::merklize_pass` expects.
///
/// Used by both `merklize_main_state` and `merklize_position` (and
/// future position-shaped pipelines). Callers do the pipeline-specific
/// filtering up front (main state filters by `value_version` via
/// `StateSlot::maybe_update_jmt`; position-shaped pipelines extract
/// straight from each `LeafEntry`) and hand the resulting flat
/// iterator here for shard routing.
pub fn pre_shard_jmt_updates<I>(
    flat: I,
) -> [Vec<(HashValue, Option<(HashValue, StateKey)>)>; NUM_STATE_SHARDS]
where
    I: IntoIterator<Item = (HashValue, Option<(HashValue, StateKey)>)>,
{
    let mut shards: [Vec<(HashValue, Option<(HashValue, StateKey)>)>; NUM_STATE_SHARDS] =
        std::array::from_fn(|_| Vec::new());
    for (key_hash, leaf) in flat {
        shards[usize::from(key_hash.nibble(0))].push((key_hash, leaf));
    }
    shards
}

/// Build the JMT-input tuple `(key_hash, Option<(value_hash, state_key)>)`
/// for one delta entry, via the [`LeafEntry`] read-shape. Returns
/// `None` if the slot has no concrete state key (e.g. main state's
/// `StateSlot` loaded from hot KV DB without the original key — such
/// entries should have been filtered out upstream).
pub fn leaf_entry_to_jmt_update<S: LeafEntry>(
    key_hash: HashValue,
    slot: &S,
) -> (HashValue, Option<(HashValue, StateKey)>) {
    let leaf = slot
        .value_hash()
        .and_then(|h| slot.state_key().map(|k| (h, k.clone())));
    (key_hash, leaf)
}
