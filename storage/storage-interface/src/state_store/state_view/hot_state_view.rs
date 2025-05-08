// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::state_slot::StateSlot;
use aptos_types::state_store::{state_key::StateKey, StateViewResult};

/// A view into the hot state store, whose content overlays on top of the cold state store content.
pub trait HotStateView: Send + Sync {
    fn get_state_slot(&self, state_key: &StateKey) -> StateViewResult<Option<StateSlot>>;
}

pub struct EmptyHotState;

impl HotStateView for EmptyHotState {
    fn get_state_slot(&self, _state_key: &StateKey) -> StateViewResult<Option<StateSlot>> {
        Ok(None)
    }
}
