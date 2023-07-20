// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0
#![forbid(unsafe_code)]
use crate::TStateView;
use anyhow::Result;
use aptos_types::state_store::{
    state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use dashmap::DashMap;

// A State view backed by in-memory hashmap.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct InMemoryStateView {
    state_data: DashMap<StateKey, StateValue>,
}

impl InMemoryStateView {
    pub fn new(state_data: DashMap<StateKey, StateValue>) -> Self {
        Self { state_data }
    }
}

impl TStateView for InMemoryStateView {
    type Key = StateKey;

    fn get_state_value(&self, state_key: &StateKey) -> Result<Option<StateValue>> {
        let state_value = self.state_data.get(state_key).map(|v| v.value().clone());
        Ok(state_value)
    }

    fn is_genesis(&self) -> bool {
        unimplemented!("is_genesis is not implemented for InMemoryStateView")
    }

    fn get_usage(&self) -> Result<StateStorageUsage> {
        Ok(StateStorageUsage::new_untracked())
    }

    fn as_in_memory_state_view(&self) -> InMemoryStateView {
        self.clone()
    }
}
