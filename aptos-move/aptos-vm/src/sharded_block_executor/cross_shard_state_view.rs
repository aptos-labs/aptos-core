// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0
use anyhow::Result;
use aptos_state_view::{StateView, TStateView};
use aptos_types::state_store::{
    state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
};
use std::{
    collections::HashMap,
    sync::{Arc, Condvar, Mutex},
};

#[derive(Clone)]
enum CrossShardValueStatus {
    /// The state value is available as a result of cross shard execution
    Ready(StateValue),
    /// We are still waiting for remote shard to push the state value
    Waiting,
}

#[derive(Clone)]
struct CrossShardStateValue {
    value_condition: Arc<(Mutex<CrossShardValueStatus>, Condvar)>,
}

impl CrossShardStateValue {
    pub fn waiting() -> Self {
        Self {
            value_condition: Arc::new((Mutex::new(CrossShardValueStatus::Waiting), Condvar::new())),
        }
    }

    pub fn set_value(&self, value: StateValue) {
        let (lock, cvar) = &*self.value_condition;
        let mut status = lock.lock().unwrap();
        *status = CrossShardValueStatus::Ready(value);
        cvar.notify_all();
    }

    pub fn get_value(&self) -> StateValue {
        let (lock, cvar) = &*self.value_condition;
        let mut status = lock.lock().unwrap();
        while let CrossShardValueStatus::Waiting = *status {
            status = cvar.wait(status).unwrap();
        }
        match &*status {
            CrossShardValueStatus::Ready(value) => value.clone(),
            CrossShardValueStatus::Waiting => unreachable!(),
        }
    }
}

/// A state view for reading cross shard state values. It is backed by a state view
/// and a hashmap of cross shard state keys. When a cross shard state value is not
/// available in the hashmap, it will be fetched from the underlying state view.
#[derive(Clone)]
pub struct CrossShardStateView<'a, S: StateView + Sync + Send> {
    cross_shard_data: HashMap<StateKey, CrossShardStateValue>,
    state_view: &'a S,
}

impl<'a, S: StateView + Sync + Send> CrossShardStateView<'a, S> {
    pub fn new(cross_shard_keys: Vec<StateKey>, state_view: &'a S) -> Self {
        let mut cross_shard_data = HashMap::new();
        for key in cross_shard_keys {
            cross_shard_data.insert(key, CrossShardStateValue::waiting());
        }
        Self {
            cross_shard_data,
            state_view,
        }
    }
}

impl<'a, S: StateView + Sync + Send> TStateView for CrossShardStateView<'a, S> {
    type Key = StateKey;

    fn get_state_value(&self, state_key: &StateKey) -> Result<Option<StateValue>> {
        let may_be_value_cond = self.cross_shard_data.get(state_key);
        if may_be_value_cond.is_none() {
            return self.state_view.get_state_value(state_key);
        }
        Ok(Some(may_be_value_cond.unwrap().get_value()))
    }

    fn is_genesis(&self) -> bool {
        unimplemented!("is_genesis is not implemented for InMemoryStateView")
    }

    fn get_usage(&self) -> Result<StateStorageUsage> {
        Ok(StateStorageUsage::new_untracked())
    }
}
