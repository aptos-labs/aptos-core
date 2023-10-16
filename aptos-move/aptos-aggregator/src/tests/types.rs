// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    delta_change_set::serialize,
    resolver::{DelayedFieldReadMode, TDelayedFieldView},
    types::{AggregatorVersionedID, DelayedFieldID, DelayedFieldValue},
};
use aptos_types::state_store::{state_key::StateKey, state_value::StateValue};
use std::{cell::RefCell, collections::HashMap};

pub fn aggregator_v1_id_for_test(key: u128) -> AggregatorVersionedID {
    AggregatorVersionedID::V1(aggregator_v1_state_key_for_test(key))
}

pub fn aggregator_v1_state_key_for_test(key: u128) -> StateKey {
    StateKey::raw(key.to_le_bytes().to_vec())
}

pub struct FakeAggregatorView {
    // TODO: consider adding deltas to test different read modes.
    v1_store: HashMap<StateKey, StateValue>,
    v2_store: HashMap<DelayedFieldID, DelayedFieldValue>,
    counter: RefCell<u32>,
}

impl Default for FakeAggregatorView {
    fn default() -> Self {
        Self {
            v1_store: HashMap::new(),
            v2_store: HashMap::new(),
            // Put some recognizable number, to easily spot missed exchanges
            counter: RefCell::new(87654321),
        }
    }
}

impl FakeAggregatorView {
    pub fn set_from_state_key(&mut self, state_key: StateKey, value: u128) {
        let state_value = StateValue::new_legacy(serialize(&value).into());
        self.v1_store.insert(state_key, state_value);
    }

    pub fn set_from_aggregator_id(&mut self, id: DelayedFieldID, value: u128) {
        self.v2_store
            .insert(id, DelayedFieldValue::Aggregator(value));
    }
}

impl TDelayedFieldView for FakeAggregatorView {
    type IdentifierV1 = StateKey;
    type IdentifierV2 = DelayedFieldID;

    fn get_aggregator_v1_state_value(
        &self,
        state_key: &Self::IdentifierV1,
        _mode: DelayedFieldReadMode,
    ) -> anyhow::Result<Option<StateValue>> {
        Ok(self.v1_store.get(state_key).cloned())
    }

    fn get_delayed_field_value(
        &self,
        id: &Self::IdentifierV2,
        _mode: DelayedFieldReadMode,
    ) -> anyhow::Result<DelayedFieldValue> {
        self.v2_store
            .get(id)
            .cloned()
            .ok_or_else(|| anyhow::Error::msg(format!("Value does not exist for id {:?}", id)))
    }

    fn generate_delayed_field_id(&self) -> Self::IdentifierV2 {
        let mut counter = self.counter.borrow_mut();
        let id = Self::IdentifierV2::new(*counter as u64);
        *counter += 1;
        id
    }
}
