// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Speculative overlay of a native index: one `MapLayer` per shard per
//! chunk, chained from the parent's, read over a resident base supplied at
//! read time. Generic over the key and value; each key names its shard.

#![forbid(unsafe_code)]

use aptos_experimental_layered_map::{Key, MapLayer, Value};
use aptos_types::{state_store::NUM_STATE_SHARDS, transaction::Version};
use rayon::prelude::*;
use std::sync::Arc;

pub const NUM_NATIVE_INDEX_SHARDS: usize = NUM_STATE_SHARDS;

/// A key of a native index. Keys that a base groups together must share a
/// shard, so the base can fold each shard on its own.
pub trait ShardedKey: Key + Send + Sync + 'static {
    fn shard(&self) -> usize;
}

pub trait NativeValue: Value + Send + Sync + 'static {}

impl<T: Value + Send + Sync + 'static> NativeValue for T {}

pub type NativeLayer<K, V> = MapLayer<K, Option<V>>;
pub type ShardedNativeLayers<K, V> = [NativeLayer<K, V>; NUM_NATIVE_INDEX_SHARDS];

/// A chunk's writes. `None` deletes.
pub type NativeWrites<K, V> = Vec<(K, Option<V>)>;

/// Holds the layers only; the base they are read over is supplied at read
/// time, so an overlay cannot name a base other than the one it is merged
/// with. Like main state's `State`.
#[derive(Clone, Debug)]
pub struct NativeOverlay<K: ShardedKey, V: NativeValue> {
    next_version: Version,
    tops: Arc<ShardedNativeLayers<K, V>>,
}

/// What a chunk's overlay extends from: the parent overlay and the base
/// layers it was checked against, captured together at the top of the
/// execution stage so the new layer is viewable from that base.
pub struct NativeParent<'a, K: ShardedKey, V: NativeValue> {
    pub overlay: &'a NativeOverlay<K, V>,
    pub floor: &'a ShardedNativeLayers<K, V>,
}

impl<K: ShardedKey, V: NativeValue> Clone for NativeParent<'_, K, V> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<K: ShardedKey, V: NativeValue> Copy for NativeParent<'_, K, V> {}

impl<K: ShardedKey, V: NativeValue> NativeOverlay<K, V> {
    /// An empty overlay directly on a base's `layers`, at the base's
    /// `next_version`.
    pub fn new(next_version: Version, layers: ShardedNativeLayers<K, V>) -> Self {
        Self {
            next_version,
            tops: Arc::new(layers),
        }
    }

    pub fn next_version(&self) -> Version {
        self.next_version
    }

    pub fn version(&self) -> Option<Version> {
        self.next_version.checked_sub(1)
    }

    pub fn shards(&self) -> &ShardedNativeLayers<K, V> {
        &self.tops
    }

    pub fn is_descendant_of(&self, rhs: &Self) -> bool {
        self.descends_from(&rhs.tops)
    }

    /// Whether every shard's top descends from `layers`. Shards move in
    /// lockstep, so one predicate covers all of them.
    pub fn descends_from(&self, layers: &ShardedNativeLayers<K, V>) -> bool {
        (0..NUM_NATIVE_INDEX_SHARDS).all(|shard| self.tops[shard].is_descendant_of(&layers[shard]))
    }

    /// Layers between `floor` and the top, i.e. chunks not yet folded into a
    /// base at `floor`.
    pub fn layers_above(&self, floor: &ShardedNativeLayers<K, V>) -> u64 {
        (0..NUM_NATIVE_INDEX_SHARDS)
            .map(|shard| {
                self.tops[shard]
                    .layer()
                    .saturating_sub(floor[shard].layer())
            })
            .max()
            .unwrap_or(0)
    }

    /// Writes above `floor` — what the overlay shadows when read over a
    /// base at `floor`. Walks layers only, so it is safe inside a base's
    /// read lock.
    pub fn writes_since(&self, floor: &ShardedNativeLayers<K, V>) -> NativeWrites<K, V> {
        (0..NUM_NATIVE_INDEX_SHARDS)
            .into_par_iter()
            .flat_map_iter(|shard| {
                self.tops[shard]
                    .view_layers_after(&floor[shard])
                    .iter()
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    fn split_by_shard(writes: NativeWrites<K, V>) -> Vec<NativeWrites<K, V>> {
        let mut per_shard = vec![NativeWrites::new(); NUM_NATIVE_INDEX_SHARDS];
        for (key, write) in writes {
            per_shard[key.shard()].push((key, write));
        }
        per_shard
    }

    /// Push one layer per shard atop `self`, built over `floor` — the
    /// base's layers at the time. A layer links to nothing at or below
    /// its floor, so this is the lowest base it can ever be read from;
    /// the base's current layers are the right floor because the base
    /// only moves forward.
    pub fn extend(
        &self,
        floor: &ShardedNativeLayers<K, V>,
        new_version: Version,
        writes: NativeWrites<K, V>,
    ) -> Self {
        // The new layer covers `[self.next_version(), new_version]`, so a
        // lower version means the caller is extending from the wrong
        // parent. A base's fold makes skip/advance decisions off this
        // stamp, so a bad one propagates. Main state guards the same
        // thing in `State::update`.
        assert!(
            new_version >= self.next_version(),
            "extend at version {new_version} but overlay is already at {}",
            self.next_version(),
        );
        // A sibling of the floor satisfies `can_view_after` yet shares no
        // ancestry; building over it would mix branches.
        assert!(
            self.descends_from(floor),
            "extending an overlay over a floor it does not descend from",
        );
        let per_shard = Self::split_by_shard(writes);
        let tops: Vec<NativeLayer<K, V>> = (0..NUM_NATIVE_INDEX_SHARDS)
            .into_par_iter()
            .map(|shard| {
                self.tops[shard]
                    .view_layers_after(&floor[shard])
                    .new_layer(&per_shard[shard])
            })
            .collect();
        Self {
            next_version: new_version + 1,
            tops: Arc::new(
                tops.try_into()
                    .unwrap_or_else(|_| panic!("Known to be NUM_NATIVE_INDEX_SHARDS shards.")),
            ),
        }
    }
}
