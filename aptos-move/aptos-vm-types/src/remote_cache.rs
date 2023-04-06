// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::write::AptosWrite;
use aptos_state_view::TStateView;
use aptos_types::state_store::state_key::StateKey;
use std::ops::Deref;

/// Snapshot of memory available to the VM. Note that this trait explicitly hides
/// all interaction with the global memory. Implementors of this trait can redirect
/// calls to the `StateView` to obtain blobs or add custom logic.
pub trait TRemoteCache {
    type Key;

    /// Gets the module for a given state key.
    /// TODO: Change to a new Move type when available instead of a blob.
    /// TODO: Remove from AptosWrite because it is a module!
    fn get_cached_module(&self, state_key: &Self::Key) -> anyhow::Result<Option<AptosWrite>>;

    /// Gets the resource for a given state key.
    fn get_cached_resource(&self, state_key: &Self::Key) -> anyhow::Result<Option<AptosWrite>>;
}

pub trait RemoteCache: TRemoteCache<Key = StateKey> {}

impl<T: TRemoteCache<Key = StateKey>> RemoteCache for T {}

impl<R, S, K> TRemoteCache for R
where
    R: Deref<Target = S>,
    S: TRemoteCache<Key = K>,
{
    type Key = K;

    fn get_cached_module(&self, state_key: &Self::Key) -> anyhow::Result<Option<AptosWrite>> {
        self.deref().get_cached_module(state_key)
    }

    fn get_cached_resource(&self, state_key: &Self::Key) -> anyhow::Result<Option<AptosWrite>> {
        self.deref().get_cached_resource(state_key)
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
