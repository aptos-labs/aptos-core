// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    state_store::{
        leaf_entry::LeafEntry, state_summary::StateSummary, state_with_summary::StateAndSummary,
    },
    AptosDbError, Result,
};
use aptos_crypto::HashValue;
use aptos_experimental_layered_map::MapLayer;
use aptos_scratchpad::ProofRead;
use aptos_types::{
    state_store::{state_key::StateKey, NUM_STATE_SHARDS},
    transaction::Version,
};
use arr_macro::arr;
use std::sync::Arc;

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

#[derive(Clone, Debug)]
pub struct ShardedJmtState<Slot: Clone + Send + Sync + 'static> {
    next_version: Version,
    shards: Arc<[MapLayer<HashValue, Slot>; NUM_STATE_SHARDS]>,
}

impl<Slot: Clone + Send + Sync + 'static> ShardedJmtState<Slot> {
    pub fn new_empty(family: &'static str) -> Self {
        Self {
            next_version: 0,
            shards: Arc::new(arr![MapLayer::new_family(family); 16]),
        }
    }

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

    pub fn is_descendant_of(&self, rhs: &Self) -> bool {
        self.shards[0].is_descendant_of(&rhs.shards[0])
    }

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

impl<Slot: Clone + Send + Sync + LeafEntry + 'static> StateAndSummary<ShardedJmtState<Slot>> {
    pub fn new_empty(family: &'static str) -> Self {
        Self::new(
            ShardedJmtState::new_empty(family),
            StateSummary::new_empty_global_only(),
        )
    }

    pub fn is_descendant_of(&self, other: &Self) -> bool {
        self.state().is_descendant_of(other.state())
            && self.summary().is_descendant_of(other.summary())
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

    pub fn make_delta(&self, base: &Self) -> Vec<(HashValue, Slot)> {
        self.state().make_delta(base.state())
    }
}

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
