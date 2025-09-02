// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::{
    state_store::{
        state_key::StateKey, state_slot::StateSlot, state_storage_usage::StateStorageUsage,
        state_value::StateValue, StateView, StateViewResult, TStateView,
    },
    transaction::Version,
};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents the read-set obtained when executing transactions.
#[derive(Serialize, Deserialize)]
pub(crate) struct ReadSet {
    data: HashMap<StateKey, StateValue>,
}

impl TStateView for ReadSet {
    type Key = StateKey;

    fn next_version(&self) -> Version {
        0
    }

    fn get_state_slot(&self, state_key: &Self::Key) -> StateViewResult<StateSlot> {
        let slot = match self.data.get(state_key) {
            Some(state_value) => StateSlot::ColdOccupied {
                value_version: 0,
                value: state_value.clone(),
            },
            None => StateSlot::ColdVacant,
        };
        Ok(slot)
    }

    fn get_usage(&self) -> StateViewResult<StateStorageUsage> {
        unreachable!("Should not be called when benchmarking")
    }
}

/// [StateView] implementation that records all execution reads. Captured reads can be converted
/// into a [ReadSet].
pub(crate) struct ReadSetCapturingStateView<'s, S> {
    captured_reads: Mutex<HashMap<StateKey, StateValue>>,
    state_view: &'s S,
}

impl<'s, S: StateView> ReadSetCapturingStateView<'s, S> {
    pub(crate) fn new(
        state_view: &'s S,
        mut initial_read_set: HashMap<StateKey, StateValue>,
    ) -> Self {
        // For generating and capturing reads, benchmark uses sequential execution. This means that
        // at runtime, due to speculation in parallel execution, it is possible to access something
        // outside the read-set. This is ok, but we preload framework to avoid unexpected logs and
        // issues in the VM where prologue cannot find modules like 0x1::error.
        for package in &aptos_framework::testnet_release_bundle().packages {
            for (_, module) in package.sorted_code_and_modules() {
                let state_key = StateKey::module(module.self_addr(), module.self_name());
                if let Some(state_value) =
                    state_view
                        .get_state_value(&state_key)
                        .unwrap_or_else(|err| {
                            panic!("Failed to fetch module for {:?}: {:?}", state_key, err)
                        })
                {
                    // If there is an override, we do not want to insert the value.
                    initial_read_set.entry(state_key).or_insert(state_value);
                }
            }
        }

        Self {
            captured_reads: Mutex::new(initial_read_set),
            state_view,
        }
    }

    pub(crate) fn into_read_set(self) -> ReadSet {
        ReadSet {
            data: self.captured_reads.into_inner(),
        }
    }
}

impl<S: StateView> TStateView for ReadSetCapturingStateView<'_, S> {
    type Key = StateKey;

    fn get_state_slot(&self, state_key: &Self::Key) -> StateViewResult<StateSlot> {
        // Check the read-set first.
        if let Some(state_value) = self.captured_reads.lock().get(state_key) {
            return Ok(StateSlot::ColdOccupied {
                value_version: 0,
                value: state_value.clone(),
            });
        }

        // We do not allow failures because then benchmarking will not be correct (we miss a read).
        // Plus, these failures should not happen when replaying past transactions.
        let slot = self
            .state_view
            .get_state_slot(state_key)
            .unwrap_or_else(|err| {
                panic!("Failed to fetch state value for {:?}: {:?}", state_key, err)
            });

        // Capture the read on first access.
        if let Some(state_value) = slot.as_state_value_opt() {
            let mut captured_reads = self.captured_reads.lock();
            if !captured_reads.contains_key(state_key) {
                captured_reads.insert(state_key.clone(), state_value.clone());
            }
        }

        Ok(slot)
    }

    fn next_version(&self) -> Version {
        0
    }

    fn get_usage(&self) -> StateViewResult<StateStorageUsage> {
        unreachable!("Should not be called when benchmarking")
    }
}
