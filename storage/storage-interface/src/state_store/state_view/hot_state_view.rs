// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::Result;
use aptos_types::state_store::{state_key::StateKey, state_slot::StateSlot};

#[derive(Debug, thiserror::Error)]
#[error("HotState view revoked")]
pub struct HotStateRevoked;

/// A view into the hot state store, whose content overlays on top of the cold state store content.
pub trait HotStateView: Send + Sync {
    fn get_state_slot(&self, state_key: &StateKey) -> Result<Option<StateSlot>>;
}

pub struct EmptyHotState;

impl HotStateView for EmptyHotState {
    fn get_state_slot(&self, _state_key: &StateKey) -> Result<Option<StateSlot>> {
        Ok(None)
    }
}
