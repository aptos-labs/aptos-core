// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::sharded_block_executor::remote_state_value::RemoteStateValue;
use anyhow::Result;
use aptos_logger::trace;
use aptos_types::{
    block_executor::partitioner::TransactionWithDependencies,
    state_store::{
        errors::StateViewError, state_key::StateKey, state_storage_usage::StateStorageUsage,
        state_value::StateValue, StateView, TStateView,
    },
    transaction::analyzed_transaction::AnalyzedTransaction,
};
use std::collections::{HashMap, HashSet};

/// A state view for reading cross shard state values. It is backed by a state view
/// and a hashmap of cross shard state keys. When a cross shard state value is not
/// available in the hashmap, it will be fetched from the underlying base view.
#[derive(Clone)]
pub struct CrossShardStateView<'a, S> {
    cross_shard_data: HashMap<StateKey, RemoteStateValue>,
    base_view: &'a S,
}

impl<'a, S: StateView + Sync + Send> CrossShardStateView<'a, S> {
    pub fn new(cross_shard_keys: HashSet<StateKey>, base_view: &'a S) -> Self {
        let mut cross_shard_data = HashMap::new();
        trace!(
            "Initializing cross shard state view with {} keys",
            cross_shard_keys.len(),
        );
        for key in cross_shard_keys {
            cross_shard_data.insert(key, RemoteStateValue::waiting());
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
            .filter(|v| !v.is_ready())
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

impl<S: StateView + Sync + Send> TStateView for CrossShardStateView<'_, S> {
    type Key = StateKey;

    fn get_state_value(&self, state_key: &StateKey) -> Result<Option<StateValue>, StateViewError> {
        if let Some(value) = self.cross_shard_data.get(state_key) {
            return Ok(value.get_value());
        }
        self.base_view.get_state_value(state_key)
    }

    fn get_usage(&self) -> Result<StateStorageUsage, StateViewError> {
        Ok(StateStorageUsage::new_untracked())
    }
}

#[cfg(test)]
mod tests {
    use crate::sharded_block_executor::cross_shard_state_view::CrossShardStateView;
    use aptos_types::state_store::{
        errors::StateViewError, state_key::StateKey, state_storage_usage::StateStorageUsage,
        state_value::StateValue, TStateView,
    };
    use std::{collections::HashSet, sync::Arc, thread, time::Duration};

    struct EmptyView;

    impl TStateView for EmptyView {
        type Key = StateKey;

        fn get_state_value(
            &self,
            _state_key: &StateKey,
        ) -> Result<Option<StateValue>, StateViewError> {
            Ok(None)
        }

        fn get_usage(&self) -> Result<StateStorageUsage, StateViewError> {
            unreachable!()
        }
    }

    #[test]
    fn test_cross_shard_state_view_get_state_value() {
        let state_key = StateKey::raw(b"key1");
        let state_value = StateValue::from("value1".as_bytes().to_owned());
        let state_value_clone = state_value.clone();
        let state_key_clone = state_key.clone();

        let mut state_keys = HashSet::new();
        state_keys.insert(state_key.clone());

        let cross_shard_state_view = Arc::new(CrossShardStateView::new(state_keys, &EmptyView));
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
