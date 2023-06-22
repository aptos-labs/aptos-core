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

pub struct CompositeStateView<'a> {
    state_views: Vec<&'a dyn TStateView<Key = StateKey>>,
}

impl<'a> CompositeStateView<'a> {
    pub fn new(state_views: Vec<&'a dyn TStateView<Key = StateKey>>) -> Self {
        Self { state_views }
    }
}

impl<'a> TStateView for CompositeStateView<'a> {
    type Key = StateKey;

    fn get_state_value(&self, state_key: &Self::Key) -> Result<Option<StateValue>> {
        for state_view in &self.state_views {
            if let Some(state_value) = state_view.get_state_value(state_key)? {
                return Ok(Some(state_value));
            }
        }
        Ok(None)
    }

    fn is_genesis(&self) -> bool {
        unimplemented!("is_genesis is not implemented for CompositeStateView")
    }

    fn get_usage(&self) -> Result<StateStorageUsage> {
        Ok(StateStorageUsage::new_untracked())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_state_view::in_memory_state_view::InMemoryStateView;
    use std::collections::HashMap;

    #[test]
    fn test_composite_state_view_get_state_value() {
        let state_view1 = InMemoryStateView::new({
            let mut values = HashMap::new();
            values.insert(
                StateKey::raw("key1".as_bytes().to_owned()),
                StateValue::from("value1".as_bytes().to_owned()),
            );
            // insert more
            values.insert(
                StateKey::raw("key2".as_bytes().to_owned()),
                StateValue::from("value2".as_bytes().to_owned()),
            );
            values.insert(
                StateKey::raw("key3".as_bytes().to_owned()),
                StateValue::from("value3".as_bytes().to_owned()),
            );

            values
        });

        let state_view2 = InMemoryStateView::new({
            let mut values = HashMap::new();
            // insert more
            values.insert(
                StateKey::raw("key2".as_bytes().to_owned()),
                StateValue::from("value2_2".as_bytes().to_owned()),
            );
            values.insert(
                StateKey::raw("key3".as_bytes().to_owned()),
                StateValue::from("value3_2".as_bytes().to_owned()),
            );

            values
        });

        let composite_state_view = CompositeStateView::new(vec![&state_view2, &state_view1]);

        // Returned from state_view 1
        assert_eq!(
            composite_state_view
                .get_state_value(&StateKey::raw("key1".as_bytes().to_owned()))
                .unwrap()
                .unwrap(),
            StateValue::from("value1".as_bytes().to_owned())
        );
        // Returned from state_view 2
        assert_eq!(
            composite_state_view
                .get_state_value(&StateKey::raw("key2".as_bytes().to_owned()))
                .unwrap()
                .unwrap(),
            StateValue::from("value2_2".as_bytes().to_owned())
        );
        assert_eq!(
            composite_state_view
                .get_state_value(&StateKey::raw("key3".as_bytes().to_owned()))
                .unwrap()
                .unwrap(),
            StateValue::from("value3_2".as_bytes().to_owned())
        );
    }
}
