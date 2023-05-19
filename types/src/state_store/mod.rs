// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::{state_key::StateKey, state_value::StateValue};
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
