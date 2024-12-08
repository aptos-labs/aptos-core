// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::{state_update::StateUpdateRef, NUM_STATE_SHARDS};
use aptos_types::{state_store::state_key::StateKey, transaction::Version};
use arr_macro::arr;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct BatchedStateUpdateRefs<'kv> {
    pub first_version: Version,
    pub num_versions: usize,
    pub shards: [HashMap<&'kv StateKey, StateUpdateRef<'kv>>; NUM_STATE_SHARDS],
}

impl<'kv> BatchedStateUpdateRefs<'kv> {
    pub fn new_empty(first_version: Version, num_versions: usize) -> Self {
        Self {
            first_version,
            num_versions,
            shards: arr![HashMap::new(); 16],
        }
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

    pub fn first_version(&self) -> Version {
        self.first_version
    }

    pub fn next_version(&self) -> Version {
        self.first_version + self.num_versions as Version
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
