// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    delta_change_set::serialize,
    resolver::{AggregatorReadMode, TAggregatorView},
    types::{AggregatorID, AggregatorValue, AggregatorVersionedID},
};
use aptos_types::state_store::{state_key::StateKey, state_value::StateValue};
use std::collections::HashMap;

pub fn aggregator_v1_id_for_test(key: u128) -> AggregatorVersionedID {
    AggregatorVersionedID::V1(aggregator_v1_state_key_for_test(key))
}

pub fn aggregator_v1_state_key_for_test(key: u128) -> StateKey {
    StateKey::raw(key.to_le_bytes().to_vec())
}

#[derive(Default)]
pub struct FakeAggregatorView {
    // TODO: consider adding deltas to test different read modes.
    v1_store: HashMap<StateKey, StateValue>,
    v2_store: HashMap<AggregatorID, AggregatorValue>,
}

impl FakeAggregatorView {
    pub fn set_from_state_key(&mut self, state_key: StateKey, value: u128) {
        let state_value = StateValue::new_legacy(serialize(&value).into());
        self.v1_store.insert(state_key, state_value);
    }

    pub fn set_from_aggregator_id(&mut self, id: AggregatorID, value: u128) {
        self.v2_store.insert(id, AggregatorValue::Aggregator(value));
    }
}

impl TAggregatorView for FakeAggregatorView {
    type IdentifierV1 = StateKey;
    type IdentifierV2 = AggregatorID;

    fn get_aggregator_v1_state_value(
        &self,
        state_key: &Self::IdentifierV1,
        _mode: AggregatorReadMode,
    ) -> anyhow::Result<Option<StateValue>> {
        Ok(self.v1_store.get(state_key).cloned())
    }

    fn get_aggregator_v2_value(
        &self,
        id: &Self::IdentifierV2,
        _mode: AggregatorReadMode,
    ) -> anyhow::Result<AggregatorValue> {
        self.v2_store
            .get(id)
            .cloned()
            .ok_or_else(|| anyhow::Error::msg(format!("Value does not exist for id {:?}", id)))
    }
}
