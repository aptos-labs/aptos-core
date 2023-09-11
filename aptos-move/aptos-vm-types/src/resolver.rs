// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::{aggregator_extension::AggregatorID, resolver::TAggregatorView};
use aptos_state_view::StateViewId;
use aptos_types::state_store::{
    state_key::StateKey,
    state_storage_usage::StateStorageUsage,
    state_value::{StateValue, StateValueMetadata},
};
use move_core_types::{language_storage::StructTag, value::MoveTypeLayout};

/// Allows to query resources from the state storage.
pub trait TResourceView {
    type Key;
    type Layout;

    fn get_resource_state_value(
        &self,
        state_key: &Self::Key,
        maybe_layout: Option<&Self::Layout>,
    ) -> anyhow::Result<Option<StateValue>>;

    fn get_resource_bytes(
        &self,
        state_key: &Self::Key,
        maybe_layout: Option<&Self::Layout>,
    ) -> anyhow::Result<Option<Vec<u8>>> {
        let maybe_state_value = self.get_resource_state_value(state_key, maybe_layout)?;
        Ok(maybe_state_value.map(StateValue::into_bytes))
    }

    fn get_resource_state_value_metadata(
        &self,
        state_key: &Self::Key,
    ) -> anyhow::Result<Option<StateValueMetadataKind>> {
        // For metadata, layouts are not important.
        let maybe_state_value = self.get_resource_state_value(state_key, None)?;
        Ok(maybe_state_value.map(StateValue::into_metadata))
    }
}

pub trait ResourceResolver: TResourceView<Key = StateKey, Layout = MoveTypeLayout> {}

impl<T: TResourceView<Key = StateKey, Layout = MoveTypeLayout>> ResourceResolver for T {}

/// Allows to query modules from the state storage.
pub trait TModuleView {
    type Key;

    fn get_module_state_value(&self, state_key: &Self::Key) -> anyhow::Result<Option<StateValue>>;

    fn get_module_bytes(&self, state_key: &Self::Key) -> anyhow::Result<Option<Vec<u8>>> {
        let maybe_state_value = self.get_module_state_value(state_key)?;
        Ok(maybe_state_value.map(StateValue::into_bytes))
    }

    fn get_module_state_value_metadata(
        &self,
        state_key: &Self::Key,
    ) -> anyhow::Result<Option<StateValueMetadataKind>> {
        let maybe_state_value = self.get_module_state_value(state_key)?;
        Ok(maybe_state_value.map(StateValue::into_metadata))
    }
}

pub trait ModuleResolver: TModuleView<Key = StateKey> {}

impl<T: TModuleView<Key = StateKey>> ModuleResolver for T {}

/// Allows to query state information, e.g. its usage.
pub trait StateStorageView {
    fn id(&self) -> StateViewId;

    fn get_usage(&self) -> anyhow::Result<StateStorageUsage>;
}

/// A fine-grained view of the state during execution.
pub trait ExecutorView:
    TResourceView<Key = StateKey, Layout = MoveTypeLayout>
    + TModuleView<Key = StateKey>
    + TAggregatorView<Identifier = AggregatorID>
    + StateStorageView
{
}

impl<
        T: TResourceView<Key = StateKey, Layout = MoveTypeLayout>
            + TModuleView<Key = StateKey>
            + TAggregatorView<Identifier = AggregatorID>
            + StateStorageView,
    > ExecutorView for T
{
}

/// Any state value can have metadata associated with it (Some(..) or None).
/// Having a type alias allows to avoid having nested options.
pub type StateValueMetadataKind = Option<StateValueMetadata>;

/// Allows to query storage metadata in the VM session. Needed for storage refunds.
pub trait StateValueMetadataResolver {
    fn get_module_state_value_metadata(
        &self,
        state_key: &StateKey,
    ) -> anyhow::Result<Option<StateValueMetadataKind>>;

    fn get_resource_state_value_metadata(
        &self,
        state_key: &StateKey,
    ) -> anyhow::Result<Option<StateValueMetadataKind>>;
}

pub trait TResourceGroupResolver {
    type Key;
    type Tag;

    fn get_resource_from_group(
        &self,
        key: &Self::Key,
        resource_tag: &Self::Tag,
        return_group_size: bool,
    ) -> anyhow::Result<(Option<Vec<u8>>, Option<usize>)>;

    /// Needed for backwards compatibility with the additional safety mechanism for resource
    /// groups, where the violation of the following invariant causes transaction failure:
    /// - if a resource is modified or deleted it must already exist within a group,
    /// and if it is created, it must not previously exist.
    ///
    /// For normal resources, this is asserted, but for resource groups the behavior (that
    /// we maintain) is for the transaction to fail with INVARIANT_VIOLATION_ERROR.
    /// This ensures state does not change and blockchain does not halt while the underlying
    /// issue is addressed. In order to maintain the behavior we check for resource existence,
    /// which in the context of parallel execution does not cause a full R/W conflict.
    ///
    /// Note: If and when we start using the method in other use-cases, in particular, if it
    /// may access a resource group for the first time, we should also incorporate the size
    /// charge for such access.
    fn resource_exists_in_group(
        &self,
        key: &Self::Key,
        resource_tag: &Self::Tag,
    ) -> anyhow::Result<bool> {
        self.get_resource_from_group(key, resource_tag, false)
            .map(|(res, _)| res.is_some())
    }
}

pub trait ResourceGroupResolver: TResourceGroupResolver<Key = StateKey, Tag = StructTag> {}

impl<T: TResourceGroupResolver<Key = StateKey, Tag = StructTag>> ResourceGroupResolver for T {}
