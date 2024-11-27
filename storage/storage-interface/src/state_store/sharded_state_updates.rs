// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::NUM_STATE_SHARDS;
use aptos_experimental_runtimes::thread_manager::THREAD_MANAGER;
use aptos_types::state_store::{state_key::StateKey, state_value::StateValue};
use arr_macro::arr;
use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator,
    IntoParallelRefMutIterator, ParallelIterator,
};
use std::collections::HashMap;

// FIXME(aldenhu): rename DeduppedStateWrites
// FIXME(aldenhu): change to [LayeredMap; 16]
#[derive(Clone, Debug)]
pub struct ShardedStateUpdates {
    pub shards: [HashMap<StateKey, Option<StateValue>>; NUM_STATE_SHARDS],
}

impl ShardedStateUpdates {
    pub fn new_empty() -> Self {
        Self {
            shards: arr![HashMap::new(); 16],
        }
    }

    pub fn all_shards_empty(&self) -> bool {
        self.shards.iter().all(|shard| shard.is_empty())
    }

    pub fn total_len(&self) -> usize {
        self.shards.iter().map(|shard| shard.len()).sum()
    }

    pub fn merge(&mut self, other: Self) {
        THREAD_MANAGER.get_exe_cpu_pool().install(|| {
            self.shards
                .par_iter_mut()
                .zip_eq(other.shards.into_par_iter())
                .for_each(|(l, r)| {
                    l.extend(r);
                })
        })
    }

    pub fn clone_merge(&mut self, other: &Self) {
        THREAD_MANAGER.get_exe_cpu_pool().install(|| {
            self.shards
                .par_iter_mut()
                .zip_eq(other.shards.par_iter())
                .for_each(|(l, r)| {
                    l.extend(r.clone());
                })
        })
    }

    pub fn insert(
        &mut self,
        key: StateKey,
        value: Option<StateValue>,
    ) -> Option<Option<StateValue>> {
        self.shards[key.get_shard_id() as usize].insert(key, value)
    }
}
