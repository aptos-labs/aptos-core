// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::resolver::AggregatorResolver;
use aptos_state_view::StateViewId;
use aptos_types::state_store::{
    state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValueMetadata,
};
use move_core_types::value::MoveTypeLayout;

/// Any state value can have metadata associated with it (Some(..) or None).
/// Having a type alias allows to avoid having nested options.
pub type StateValueMetadataKind = Option<StateValueMetadata>;

pub trait TResourceResolver {
    type Key;
    type Layout;

    fn get_resource_bytes(
        &self,
        state_key: &Self::Key,
        maybe_layout: Option<&Self::Layout>,
    ) -> anyhow::Result<Option<Vec<u8>>>;

    fn get_resource_state_value_metadata(
        &self,
        state_key: &Self::Key,
    ) -> anyhow::Result<Option<StateValueMetadataKind>>;
}

impl TResourceResolver for () {
    type Key = StateKey;
    type Layout = MoveTypeLayout;

    fn get_resource_bytes(
        &self,
        _state_key: &Self::Key,
        _maybe_layout: Option<&Self::Layout>,
    ) -> anyhow::Result<Option<Vec<u8>>> {
        unimplemented!()
    }

    fn get_resource_state_value_metadata(
        &self,
        _state_key: &Self::Key,
    ) -> anyhow::Result<Option<StateValueMetadataKind>> {
        unimplemented!()
    }
}

pub trait ResourceResolver: TResourceResolver<Key = StateKey, Layout = MoveTypeLayout> {}

impl<T: TResourceResolver<Key = StateKey, Layout = MoveTypeLayout>> ResourceResolver for T {}

pub trait TModuleResolver {
    type Key;

    fn get_module_bytes(&self, state_key: &Self::Key) -> anyhow::Result<Option<Vec<u8>>>;

    fn get_module_state_value_metadata(
        &self,
        state_key: &Self::Key,
    ) -> anyhow::Result<Option<StateValueMetadataKind>>;
}

impl TModuleResolver for () {
    type Key = StateKey;

    fn get_module_bytes(&self, _state_key: &Self::Key) -> anyhow::Result<Option<Vec<u8>>> {
        unimplemented!()
    }

    fn get_module_state_value_metadata(
        &self,
        _state_key: &Self::Key,
    ) -> anyhow::Result<Option<StateValueMetadataKind>> {
        unimplemented!()
    }
}

pub trait ModuleResolver: TModuleResolver<Key = StateKey> {}

impl<T: TModuleResolver<Key = StateKey>> ModuleResolver for T {}

pub trait StateStorageResolver {
    fn id(&self) -> StateViewId;

    fn get_usage(&self) -> anyhow::Result<StateStorageUsage>;
}

impl StateStorageResolver for () {
    fn id(&self) -> StateViewId {
        unimplemented!()
    }

    fn get_usage(&self) -> anyhow::Result<StateStorageUsage> {
        unimplemented!()
    }
}

pub trait ExecutorResolver:
    TResourceResolver<Key = StateKey, Layout = MoveTypeLayout>
    + TModuleResolver<Key = StateKey>
    + StateStorageResolver
    + AggregatorResolver
{
}

impl<
        T: TResourceResolver<Key = StateKey, Layout = MoveTypeLayout>
            + TModuleResolver<Key = StateKey>
            + StateStorageResolver
            + AggregatorResolver,
    > ExecutorResolver for T
{
}
