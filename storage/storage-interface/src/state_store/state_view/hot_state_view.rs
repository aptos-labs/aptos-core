// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_crypto::HashValue;
use aptos_types::state_store::state_slot::StateSlot;

/// A view into the hot state store, whose content overlays on top of the cold state store content.
pub trait HotStateView: Send + Sync {
    fn get_state_slot(&self, key_hash: &HashValue) -> Option<StateSlot>;
}

pub struct EmptyHotState;

impl HotStateView for EmptyHotState {
    fn get_state_slot(&self, _key_hash: &HashValue) -> Option<StateSlot> {
        None
    }
}
