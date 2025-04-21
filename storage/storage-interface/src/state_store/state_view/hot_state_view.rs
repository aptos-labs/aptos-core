// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::versioned_state_value::DbStateUpdate;
use aptos_types::state_store::{state_key::StateKey, StateViewResult};

/// A view into the hot state store, whose content overlays on top of the cold state store content.
pub trait HotStateView: Send + Sync {
    fn get_state_update(&self, state_key: &StateKey) -> StateViewResult<Option<DbStateUpdate>>;
}

pub struct EmptyHotState;

impl HotStateView for EmptyHotState {
    fn get_state_update(&self, _state_key: &StateKey) -> StateViewResult<Option<DbStateUpdate>> {
        Ok(None)
    }
}
