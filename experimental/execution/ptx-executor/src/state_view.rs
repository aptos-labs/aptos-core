// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::common::HashMap;
use velor_types::state_store::{
    state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
    StateView, StateViewResult, TStateView,
};
pub struct OverlayedStateView<'view> {
    base_view: &'view dyn StateView,
    overlay: HashMap<StateKey, Option<StateValue>>,
}

impl<'view> OverlayedStateView<'view> {
    pub fn new_with_overlay(
        base_view: &'view dyn StateView,
        overlay: HashMap<StateKey, Option<StateValue>>,
    ) -> Self {
        Self { base_view, overlay }
    }

    pub fn new(base_view: &'view dyn StateView) -> Self {
        Self::new_with_overlay(base_view, HashMap::new())
    }

    pub fn overwrite(&mut self, key: StateKey, value: Option<StateValue>) {
        self.overlay.insert(key, value);
    }
}

impl TStateView for OverlayedStateView<'_> {
    type Key = StateKey;

    fn get_state_value(&self, state_key: &Self::Key) -> StateViewResult<Option<StateValue>> {
        // TODO(ptx): reject non-module reads once block_metadata is analyzed for R/W set
        // TODO(ptx): remove base_view reads once module reads are dealt with
        self.overlay
            .get(state_key)
            .cloned()
            .map(Ok)
            .unwrap_or_else(|| self.base_view.get_state_value(state_key))
    }

    fn get_usage(&self) -> StateViewResult<StateStorageUsage> {
        // TODO(aldenhu): maybe remove get_usage() from StateView
        unimplemented!()
    }
}
