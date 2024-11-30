// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::NUM_STATE_SHARDS;
use aptos_experimental_layered_map::LayeredMap;
use aptos_types::state_store::{state_key::StateKey, state_value::StateValue};

// FIXME(aldenhu): rename DeduppedStateWrites?
// FIXME(aldenhu): shall we inline this type into StateDelta?
#[derive(Clone, Debug)]
pub struct ShardedStateUpdates {
    pub shards: [LayeredMap<StateKey, Option<StateValue>>; NUM_STATE_SHARDS],
}

impl ShardedStateUpdates {
    pub fn new_empty() -> Self {
        /* FIXME(aldenhu)
        Self {
            shards: arr![HashMap::new(); 16],
        }
         */
        todo!()
    }

    pub fn all_shards_empty(&self) -> bool {
        /* FIXME(aldenhu)
        self.shards.iter().all(|shard| shard.is_empty())
         */
        todo!()
    }

    pub fn total_len(&self) -> usize {
        /* FIXME(aldenhu)
        self.shards.iter().map(|shard| shard.len()).sum()
         */
        todo!()
    }

    /* FIXME(aldenhu): remove
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
     */
}
