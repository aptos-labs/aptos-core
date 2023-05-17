// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_state_view::StateViewId;
use aptos_types::state_store::{state_key::StateKey, state_storage_usage::StateStorageUsage};
use std::ops::Deref;

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

impl<T: VMView<Key = StateKey>> AptosVMView for T {}

impl<R, S, K> VMView for R
where
    R: Deref<Target = S>,
    S: VMView<Key = K>,
{
    type Key = K;

    fn id(&self) -> StateViewId {
        self.deref().id()
    }

    fn get_move_module(&self, state_key: &Self::Key) -> anyhow::Result<Option<Vec<u8>>> {
        self.deref().get_move_module(state_key)
    }

    fn get_move_resource(&self, state_key: &Self::Key) -> anyhow::Result<Option<Vec<u8>>> {
        self.deref().get_move_resource(state_key)
    }

    fn get_aggregator_value(&self, state_key: &Self::Key) -> anyhow::Result<Option<Vec<u8>>> {
        self.deref().get_move_resource(state_key)
    }

    fn get_storage_usage_at_epoch_end(&self) -> anyhow::Result<StateStorageUsage> {
        self.deref().get_storage_usage_at_epoch_end()
    }
}
