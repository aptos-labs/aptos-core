// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_state_view::TStateView;
use aptos_types::state_store::state_key::StateKey;
use move_vm_types::resolver::{ModuleRef, ResourceRef};
use std::ops::Deref;

pub trait TRemoteCache {
    type Key;

    fn get_move_module(&self, state_key: &Self::Key) -> anyhow::Result<Option<ModuleRef>>;

    fn get_move_resource(&self, state_key: &Self::Key) -> anyhow::Result<Option<ResourceRef>>;

    fn get_aggregator_value(&self, state_key: &Self::Key) -> anyhow::Result<Option<u128>>;
}

pub trait RemoteCache: TRemoteCache<Key = StateKey> {}

impl<T: TRemoteCache<Key = StateKey>> RemoteCache for T {}

impl<R, S, K> TRemoteCache for R
where
    R: Deref<Target = S>,
    S: TRemoteCache<Key = K>,
{
    type Key = K;

    fn get_move_module(&self, state_key: &Self::Key) -> anyhow::Result<Option<ModuleRef>> {
        self.deref().get_move_module(state_key)
    }

    fn get_move_resource(&self, state_key: &Self::Key) -> anyhow::Result<Option<ResourceRef>> {
        self.deref().get_move_resource(state_key)
    }

    fn get_aggregator_value(&self, state_key: &Self::Key) -> anyhow::Result<Option<u128>> {
        self.deref().get_aggregator_value(state_key)
    }
}

pub trait TStateViewWithRemoteCache:
    TStateView<Key = Self::CommonKey> + TRemoteCache<Key = Self::CommonKey>
{
    type CommonKey;
}

pub trait StateViewWithRemoteCache: TStateViewWithRemoteCache<CommonKey = StateKey> {}

impl<T: TStateViewWithRemoteCache<CommonKey = StateKey>> StateViewWithRemoteCache for T {}

impl<R, S, K> TStateViewWithRemoteCache for R
where
    R: Deref<Target = S>,
    S: TStateViewWithRemoteCache<CommonKey = K>,
{
    type CommonKey = K;
}
