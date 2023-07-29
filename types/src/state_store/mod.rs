// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::{state_key::StateKey, state_value::StateValue};
use arr_macro::arr;
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

pub mod state_key;
pub mod state_key_prefix;
pub mod state_storage_usage;
pub mod state_value;
pub mod table;

pub type ShardedStateUpdatesInner = [HashMap<StateKey, Option<StateValue>>; 256];

#[derive(PartialEq, Clone, Eq, Debug)]
pub struct ShardedStateUpdates(ShardedStateUpdatesInner);

impl Default for ShardedStateUpdates {
    fn default() -> Self {
        create_empty_sharded_state_updates()
    }
}

impl Deref for ShardedStateUpdates {
    type Target = ShardedStateUpdatesInner;
    fn deref(&self) -> &ShardedStateUpdatesInner {
        &self.0
    }
}

impl DerefMut for ShardedStateUpdates {
    fn deref_mut(&mut self) -> &mut ShardedStateUpdatesInner {
        &mut self.0
    }
}

impl From<ShardedStateUpdatesInner> for ShardedStateUpdates {
    fn from(inner: ShardedStateUpdatesInner) -> Self {
        Self(inner)
    }
}

impl IntoIterator for ShardedStateUpdates {
    type Item = HashMap<StateKey, Option<StateValue>>;
    type IntoIter = <ShardedStateUpdatesInner as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

pub fn create_empty_sharded_state_updates() -> ShardedStateUpdates {
    ShardedStateUpdates(arr![HashMap::new(); 256])
}
