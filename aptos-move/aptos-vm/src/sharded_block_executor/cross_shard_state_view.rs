// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0
use anyhow::Result;
use aptos_logger::trace;
use aptos_state_view::{StateView, TStateView};
use aptos_types::{
    block_executor::partitioner::TransactionWithDependencies,
    state_store::{
        state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
    },
    transaction::analyzed_transaction::AnalyzedTransaction,
};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Condvar, Mutex},
};

#[derive(Clone)]
enum CrossShardValueStatus {
    /// The state value is available as a result of cross shard execution
    Ready(Option<StateValue>),
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

    pub fn set_value(&self, value: Option<StateValue>) {
        let (lock, cvar) = &*self.value_condition;
        let mut status = lock.lock().unwrap();
        // We only allow setting the value once and it must be in the waiting state
        assert!(matches!(*status, CrossShardValueStatus::Waiting));
        *status = CrossShardValueStatus::Ready(value);
        cvar.notify_all();
    }

    pub fn get_value(&self) -> Option<StateValue> {
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
/// available in the hashmap, it will be fetched from the underlying base view.
#[derive(Clone)]
pub struct CrossShardStateView<'a, S> {
    cross_shard_data: HashMap<StateKey, CrossShardStateValue>,
    base_view: &'a S,
}

impl<'a, S: StateView + Sync + Send> CrossShardStateView<'a, S> {
    pub fn new(cross_shard_keys: HashSet<StateKey>, base_view: &'a S) -> Self {
        let mut cross_shard_data = HashMap::new();
        trace!(
            "Initalizing cross shard state view with {} keys",
            cross_shard_keys.len(),
        );
        for key in cross_shard_keys {
            cross_shard_data.insert(key, CrossShardStateValue::waiting());
        }
        Self {
            cross_shard_data,
            base_view,
        }
    }

    #[cfg(test)]
    fn waiting_count(&self) -> usize {
        self.cross_shard_data
            .values()
            .filter(|v| {
                matches!(
                    v.value_condition.0.lock().unwrap().clone(),
                    CrossShardValueStatus::Waiting
                )
            })
            .count()
    }

    pub fn set_value(&self, state_key: &StateKey, state_value: Option<StateValue>) {
        self.cross_shard_data
            .get(state_key)
            .unwrap()
            .set_value(state_value);
        // uncomment the following line to debug waiting count
        // trace!("waiting count for shard id {} is {}", self.shard_id, self.waiting_count());
    }

    pub fn create_cross_shard_state_view(
        base_view: &'a S,
        transactions: &[TransactionWithDependencies<AnalyzedTransaction>],
    ) -> CrossShardStateView<'a, S> {
        let mut cross_shard_state_key = HashSet::new();
        for txn in transactions {
            for (_, storage_locations) in txn.cross_shard_dependencies.required_edges_iter() {
                for storage_location in storage_locations {
                    cross_shard_state_key.insert(storage_location.clone().into_state_key());
                }
            }
        }
        CrossShardStateView::new(cross_shard_state_key, base_view)
    }
}

impl<'a, S: StateView + Sync + Send> TStateView for CrossShardStateView<'a, S> {
    type Key = StateKey;

    fn get_state_value(&self, state_key: &StateKey) -> Result<Option<StateValue>> {
        if let Some(value) = self.cross_shard_data.get(state_key) {
            return Ok(value.get_value());
        }
        self.base_view.get_state_value(state_key)
    }

    fn is_genesis(&self) -> bool {
        unimplemented!("is_genesis is not implemented for InMemoryStateView")
    }

    fn get_usage(&self) -> Result<StateStorageUsage> {
        Ok(StateStorageUsage::new_untracked())
    }
}

#[cfg(test)]
mod tests {
    use crate::sharded_block_executor::cross_shard_state_view::CrossShardStateView;
    use aptos_state_view::{in_memory_state_view::InMemoryStateView, TStateView};
    use aptos_types::state_store::{state_key::StateKey, state_value::StateValue};
    use once_cell::sync::Lazy;
    use std::{
        collections::{HashMap, HashSet},
        sync::Arc,
        thread,
        time::Duration,
    };

    pub static EMPTY_VIEW: Lazy<Arc<InMemoryStateView>> =
        Lazy::new(|| Arc::new(InMemoryStateView::new(HashMap::new())));

    #[test]
    fn test_cross_shard_state_view_get_state_value() {
        let state_key = StateKey::raw("key1".as_bytes().to_owned());
        let state_value = StateValue::from("value1".as_bytes().to_owned());
        let state_value_clone = state_value.clone();
        let state_key_clone = state_key.clone();

        let mut state_keys = HashSet::new();
        state_keys.insert(state_key.clone());

        let cross_shard_state_view = Arc::new(CrossShardStateView::new(state_keys, &EMPTY_VIEW));
        let cross_shard_state_view_clone = cross_shard_state_view.clone();

        let wait_thread = thread::spawn(move || {
            let value = cross_shard_state_view_clone.get_state_value(&state_key_clone);
            assert_eq!(value.unwrap(), Some(state_value_clone));
        });

        // Simulate some processing time before setting the value
        thread::sleep(Duration::from_millis(100));

        cross_shard_state_view.set_value(&state_key, Some(state_value));
        assert_eq!(cross_shard_state_view.waiting_count(), 0);

        wait_thread.join().unwrap();
    }
}
