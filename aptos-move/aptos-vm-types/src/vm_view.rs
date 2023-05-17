// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_state_view::StateViewId;
use aptos_types::state_store::{state_key::StateKey, state_storage_usage::StateStorageUsage};

/// Cache view available to the VM.
pub trait VMView {
    type Key;

    fn id(&self) -> StateViewId {
        StateViewId::Miscellaneous
    }

    fn get_move_module(&self, state_key: &Self::Key) -> anyhow::Result<Option<Vec<u8>>>;

    fn get_move_resource(&self, state_key: &Self::Key) -> anyhow::Result<Option<Vec<u8>>>;

    fn get_aggregator_value(&self, state_key: &Self::Key) -> anyhow::Result<Option<Vec<u8>>>;

    fn get_storage_usage_at_epoch_end(&self) -> anyhow::Result<StateStorageUsage>;
}

pub trait AptosVMView: VMView<Key = StateKey> {}
