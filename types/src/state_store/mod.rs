// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::{state_key::StateKey, state_value::StateValue};
use aptos_experimental_runtimes::thread_manager::THREAD_MANAGER;
use arr_macro::arr;
use std::collections::HashMap;

pub mod state_key;
pub mod state_key_prefix;
pub mod state_storage_usage;
pub mod state_value;
pub mod table;

pub type ShardedStateUpdates = [HashMap<StateKey, Option<StateValue>>; 16];

pub fn create_empty_sharded_state_updates() -> ShardedStateUpdates {
    arr![HashMap::new(); 16]
}

pub fn combine_or_add_sharded_state_updates(
    lhs: &mut Option<ShardedStateUpdates>,
    rhs: ShardedStateUpdates,
) {
    if let Some(lhs) = lhs {
        combine_sharded_state_updates(lhs, rhs);
    } else {
        *lhs = Some(rhs);
    }
}

pub fn combine_sharded_state_updates(lhs: &mut ShardedStateUpdates, rhs: ShardedStateUpdates) {
    use rayon::prelude::*;

    THREAD_MANAGER.get_exe_cpu_pool().install(|| {
        lhs.par_iter_mut()
            .zip_eq(rhs.into_par_iter())
            .for_each(|(l, r)| {
                l.extend(r);
            })
    })
}
