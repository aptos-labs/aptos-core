// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::{
    state_store::{
        errors::StateViewError, state_key::StateKey, state_storage_usage::StateStorageUsage,
        state_value::StateValue, StateView, TStateView,
    },
    write_set::TOTAL_SUPPLY_STATE_KEY,
};

type Result<T, E = StateViewError> = std::result::Result<T, E>;

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

impl<S: StateView + Sync + Send> TStateView for AggregatorOverriddenStateView<'_, S> {
    type Key = StateKey;

    fn get_state_value(&self, state_key: &StateKey) -> Result<Option<StateValue>> {
        if *state_key == *TOTAL_SUPPLY_STATE_KEY {
            // TODO: Remove this when we have aggregated total supply implementation for remote
            //       sharding. For now we need this because after all the txns are executed, the
            //       proof checker expects the total_supply to read/written to the tree.
            self.base_view.get_state_value(state_key)?;
            return self.total_supply_base_view_override();
        }
        self.base_view.get_state_value(state_key)
    }

    fn get_usage(&self) -> Result<StateStorageUsage> {
        self.base_view.get_usage()
    }
}
