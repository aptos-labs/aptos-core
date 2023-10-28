// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::{
    resolver::{TAggregatorV1View, TDelayedFieldView},
    types::DelayedFieldID,
};
use aptos_state_view::{StateView, StateViewId};
use aptos_types::state_store::{
    state_key::StateKey,
    state_storage_usage::StateStorageUsage,
    state_value::{StateValue, StateValueMetadataKind},
};
use bytes::Bytes;
use move_core_types::{language_storage::StructTag, value::MoveTypeLayout};
use std::collections::{BTreeMap, HashMap};

/// Allows to query resources from the state.
pub trait TResourceView {
    type Key;
    type Layout;

    /// Returns
    ///   -  Ok(None)         if the resource is not in storage,
    ///   -  Ok(Some(...))    if the resource exists in storage,
    ///   -  Err(...)         otherwise (e.g. storage error).
    fn get_resource_state_value(
        &self,
        state_key: &Self::Key,
        maybe_layout: Option<&Self::Layout>,
    ) -> anyhow::Result<Option<StateValue>>;

    fn get_resource_bytes(
        &self,
        state_key: &Self::Key,
        maybe_layout: Option<&Self::Layout>,
    ) -> anyhow::Result<Option<Bytes>> {
        let maybe_state_value = self.get_resource_state_value(state_key, maybe_layout)?;
        Ok(maybe_state_value.map(|state_value| state_value.bytes().clone()))
    }

    fn get_resource_state_value_metadata(
        &self,
        state_key: &Self::Key,
    ) -> anyhow::Result<Option<StateValueMetadataKind>> {
        // For metadata, layouts are not important.
        self.get_resource_state_value(state_key, None)
            .map(|maybe_state_value| maybe_state_value.map(StateValue::into_metadata))
    }

    fn resource_exists(&self, state_key: &Self::Key) -> anyhow::Result<bool> {
        // For existence, layouts are not important.
        self.get_resource_state_value(state_key, None)
            .map(|maybe_state_value| maybe_state_value.is_some())
    }
}

/// Metadata and exists queries for the resource group, determined by a key, must be resolved
/// via TResourceView's corresponding interfaces w. key (get_resource_state_value_metadata &
/// resource_exists). This simplifies interfaces for now, TODO: revisit later.
pub trait TResourceGroupView {
    type GroupKey;
    type ResourceTag;
    type Layout;

    /// Some resolvers might not be capable of the optimization, and should return false.
    /// Others might return based on the config or the run paramaters.
    fn is_resource_group_split_in_change_set_capable(&self) -> bool {
        false
    }

    /// The size of the resource group, based on the sizes of the latest entries at observed
    /// tags. During parallel execution, this is an estimated value that will get validated,
    /// but as long as it is assumed correct, the transaction can deterministically derive
    /// its behavior, e.g. charge the first access or write-related gas accordingly. The
    /// implementation ensures that resource_group_size, resource_exists, and .._metadata
    /// methods return somewhat consistent values (e.g. size != 0 if exists is true), and
    /// otherwise return an error as the validation is guaranteed to fail.
    ///
    /// The collected size is only guaranteed to correspond to the correct size when executed
    /// from a quiescent, correct state. The result can be viewed as a branch prediction in
    /// the parallel execution setting, as a wrong value will be (later) caught by validation.
    /// Thus, R/W conflicts are avoided, as long as the estimates are correct (e.g. updating
    /// struct members of a fixed size).
    fn resource_group_size(&self, group_key: &Self::GroupKey) -> anyhow::Result<u64>;

    fn get_resource_from_group(
        &self,
        group_key: &Self::GroupKey,
        resource_tag: &Self::ResourceTag,
        maybe_layout: Option<&Self::Layout>,
    ) -> anyhow::Result<Option<Bytes>>;

    /// Needed for charging storage fees for a resource group write, as that requires knowing
    /// the size of the resource group AFTER the changeset of the transaction is applied (while
    /// the resource_group_size method provides the total group size BEFORE). To compute the
    /// AFTER size, for each modified resources within the group, the prior size can be
    /// determined by the following method (returns 0 for non-existent / deleted resources).
    fn resource_size_in_group(
        &self,
        group_key: &Self::GroupKey,
        resource_tag: &Self::ResourceTag,
    ) -> anyhow::Result<u64> {
        Ok(self
            .get_resource_from_group(group_key, resource_tag, None)?
            .map_or(0, |bytes| bytes.len() as u64))
    }

    /// Needed for backwards compatibility with the additional safety mechanism for resource
    /// groups, where the violation of the following invariant causes transaction failure:
    /// - if a resource is modified or deleted it must already exist within a group,
    /// and if it is created, it must not previously exist.
    ///
    /// For normal resources, this is asserted, but for resource groups the behavior (that
    /// we maintain) is for the transaction to fail with INVARIANT_VIOLATION_ERROR.
    /// Thus, the state does not change and blockchain does not halt while the underlying
    /// issue is addressed. In order to maintain the behavior we check for resource existence,
    /// which in the context of parallel execution does not cause a full R/W conflict.
    fn resource_exists_in_group(
        &self,
        group_key: &Self::GroupKey,
        resource_tag: &Self::ResourceTag,
    ) -> anyhow::Result<bool> {
        self.get_resource_from_group(group_key, resource_tag, None)
            .map(|maybe_bytes| maybe_bytes.is_some())
    }

    /// Executor view may internally implement a naive resource group cache when:
    /// - ExecutorView is not based on block executor, such as ExecutorViewBase
    /// - providing backwards compatibility (older gas versions) in storage adapter.
    ///
    /// The trait allows releasing the cache in such cases. Otherwise (default behavior),
    /// if naive cache is not implemeneted (e.g. in block executor), None is returned.
    fn release_group_cache(
        &self,
    ) -> Option<HashMap<Self::GroupKey, BTreeMap<Self::ResourceTag, Bytes>>> {
        None
    }
}

/// Allows to query modules from the state.
pub trait TModuleView {
    type Key;

    /// Returns
    ///   -  Ok(None)         if the module is not in storage,
    ///   -  Ok(Some(...))    if the module exists in storage,
    ///   -  Err(...)         otherwise (e.g. storage error).
    fn get_module_state_value(&self, state_key: &Self::Key) -> anyhow::Result<Option<StateValue>>;

    fn get_module_bytes(&self, state_key: &Self::Key) -> anyhow::Result<Option<Bytes>> {
        let maybe_state_value = self.get_module_state_value(state_key)?;
        Ok(maybe_state_value.map(|state_value| state_value.bytes().clone()))
    }

    fn get_module_state_value_metadata(
        &self,
        state_key: &Self::Key,
    ) -> anyhow::Result<Option<StateValueMetadataKind>> {
        let maybe_state_value = self.get_module_state_value(state_key)?;
        Ok(maybe_state_value.map(StateValue::into_metadata))
    }

    fn module_exists(&self, state_key: &Self::Key) -> anyhow::Result<bool> {
        self.get_module_state_value(state_key)
            .map(|maybe_state_value| maybe_state_value.is_some())
    }
}

/// Allows to query state information, e.g. its usage.
pub trait StateStorageView {
    fn id(&self) -> StateViewId;

    fn get_usage(&self) -> anyhow::Result<StateStorageUsage>;
}

/// A fine-grained view of the state during execution.
///
/// - The `StateView` trait should be used by the storage backend, e.g. a DB.
///   It only allows a generic key-value access and always returns bytes or
///   state values.
/// - The `ExecutorView` trait is used at executor level, e.g. BlockSTM. When
///   a block is executed, the types of accesses are always known (for example,
///   whether a resource is accessed or a module). Fine-grained structure of
///   `ExecutorView` allows to:
///     1. Specialize on access type,
///     2. Separate execution and storage abstractions.
///
/// StateView currently has a basic implementation of the ExecutorView trait,
/// which is used across tests and basic applications in the system.
/// TODO: audit and reconsider the default implementation (e.g. should not
/// resolve AggregatorV2 via the state-view based default implementation, as it
/// doesn't provide a value exchange functionality).
pub trait TExecutorView<K, T, L, I>:
    TResourceView<Key = K, Layout = L>
    + TModuleView<Key = K>
    + TAggregatorV1View<Identifier = K>
    + TDelayedFieldView<Identifier = I>
    + StateStorageView
{
}

impl<A, K, T, L, I> TExecutorView<K, T, L, I> for A where
    A: TResourceView<Key = K, Layout = L>
        + TModuleView<Key = K>
        + TAggregatorV1View<Identifier = K>
        + TDelayedFieldView<Identifier = I>
        + StateStorageView
{
}

pub trait ExecutorView: TExecutorView<StateKey, StructTag, MoveTypeLayout, DelayedFieldID> {}

impl<T> ExecutorView for T where
    T: TExecutorView<StateKey, StructTag, MoveTypeLayout, DelayedFieldID>
{
}

pub trait ResourceGroupView:
    TResourceGroupView<GroupKey = StateKey, ResourceTag = StructTag, Layout = MoveTypeLayout>
{
}

impl<T> ResourceGroupView for T where
    T: TResourceGroupView<GroupKey = StateKey, ResourceTag = StructTag, Layout = MoveTypeLayout>
{
}

/// Direct implementations for StateView.
impl<S> TResourceView for S
where
    S: StateView,
{
    type Key = StateKey;
    type Layout = MoveTypeLayout;

    fn get_resource_state_value(
        &self,
        state_key: &Self::Key,
        _maybe_layout: Option<&Self::Layout>,
    ) -> anyhow::Result<Option<StateValue>> {
        self.get_state_value(state_key)
    }
}

impl<S> TModuleView for S
where
    S: StateView,
{
    type Key = StateKey;

    fn get_module_state_value(&self, state_key: &Self::Key) -> anyhow::Result<Option<StateValue>> {
        self.get_state_value(state_key)
    }
}

impl<S> StateStorageView for S
where
    S: StateView,
{
    fn id(&self) -> StateViewId {
        self.id()
    }

    fn get_usage(&self) -> anyhow::Result<StateStorageUsage> {
        self.get_usage()
    }
}

/// Allows to query storage metadata in the VM session. Needed for storage refunds.
/// - Result being Err means storage error or some incostistency (e.g. during speculation,
/// needing to abort/halt the transaction with an error status).
/// - Ok(None) means that the corresponding data does not exist / was deleted.
/// - Ok(Some(_ : MetadataKind)) may be internally None (within Kind) if the metadata was
/// not previously provided (e.g. Legacy WriteOps).
pub trait StateValueMetadataResolver {
    fn get_module_state_value_metadata(
        &self,
        state_key: &StateKey,
    ) -> anyhow::Result<Option<StateValueMetadataKind>>;

    fn get_resource_state_value_metadata(
        &self,
        state_key: &StateKey,
    ) -> anyhow::Result<Option<StateValueMetadataKind>>;

    fn get_resource_group_state_value_metadata(
        &self,
        state_key: &StateKey,
    ) -> anyhow::Result<Option<StateValueMetadataKind>>;
}
