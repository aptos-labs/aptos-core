// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_state_view::{StateView, TStateView};
use aptos_types::{
    state_store::{
        state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
    },
    write_set::TOTAL_SUPPLY_STATE_KEY,
};

pub const TOTAL_SUPPLY_AGGR_BASE_VAL: u128 = u128::MAX >> 1;
#[derive(Clone)]
pub struct AggregatorOverriddenStateView<'a, S> {
    base_view: &'a S,
    total_supply_aggr_base_val: u128,
}

impl<'a, S: StateView + Sync + Send> AggregatorOverriddenStateView<'a, S> {
    pub fn new(base_view: &'a S, total_supply_aggr_base_val: u128) -> Self {
        Self {
            base_view,
            total_supply_aggr_base_val,
        }
    }

    fn total_supply_base_view_override(&self) -> Result<Option<StateValue>> {
        Ok(Some(StateValue::new_legacy(
            bcs::to_bytes(&self.total_supply_aggr_base_val)
                .unwrap()
                .into(),
        )))
    }
}

impl<'a, S: StateView + Sync + Send> TStateView for AggregatorOverriddenStateView<'a, S> {
    type Key = StateKey;

    fn get_state_value(&self, state_key: &StateKey) -> Result<Option<StateValue>> {
        if *state_key == *TOTAL_SUPPLY_STATE_KEY {
            self.base_view.get_state_value(state_key)?;
            return self.total_supply_base_view_override();
        }
        self.base_view.get_state_value(state_key)
    }

    fn get_usage(&self) -> Result<StateStorageUsage> {
        self.base_view.get_usage()
    }
}
