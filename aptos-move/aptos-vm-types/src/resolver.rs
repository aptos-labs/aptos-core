// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::resolver::{TAggregatorV1View, TDelayedFieldView};
use aptos_table_natives::TableResolver;
use aptos_types::{
    on_chain_config::ConfigStorage,
    state_store::{
        errors::StateViewError,
        state_key::StateKey,
        state_storage_usage::StateStorageUsage,
        state_value::{StateValue, StateValueMetadata},
        StateView, StateViewId,
    },
    vm::{
        resource_groups::{GroupSizeKind, ResourceGroupSize},
        state_view_adapter::ExecutorViewAdapter,
    },
    write_set::WriteOp,
};
use bytes::Bytes;
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{language_storage::StructTag, value::MoveTypeLayout, vm_status::StatusCode};
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use std::collections::{BTreeMap, HashMap};

/// Allows requesting an immediate interrupt to ongoing transaction execution. For example, this
/// allows an early return from a useless speculative execution when block execution has already
/// halted (e.g. due to gas limit, committing only a block prefix).
pub trait BlockSynchronizationKillSwitch {
    fn interrupt_requested(&self) -> bool;
}

pub struct NoopBlockSynchronizationKillSwitch {}

impl BlockSynchronizationKillSwitch for NoopBlockSynchronizationKillSwitch {
    fn interrupt_requested(&self) -> bool {
        false
    }
}

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
    ) -> PartialVMResult<Option<StateValue>>;

    fn get_resource_bytes(
        &self,
        state_key: &Self::Key,
        maybe_layout: Option<&Self::Layout>,
    ) -> PartialVMResult<Option<Bytes>> {
        let maybe_state_value = self.get_resource_state_value(state_key, maybe_layout)?;
        Ok(maybe_state_value.map(|state_value| state_value.bytes().clone()))
    }

    fn get_resource_state_value_metadata(
        &self,
        state_key: &Self::Key,
    ) -> PartialVMResult<Option<StateValueMetadata>> {
        // For metadata, layouts are not important.
        self.get_resource_state_value(state_key, None)
            .map(|maybe_state_value| maybe_state_value.map(StateValue::into_metadata))
    }

    fn get_resource_state_value_size(&self, state_key: &Self::Key) -> PartialVMResult<Option<u64>> {
        self.get_resource_state_value(state_key, None)
            .map(|maybe_state_value| maybe_state_value.map(|state_value| state_value.size() as u64))
    }

    fn resource_exists(&self, state_key: &Self::Key) -> PartialVMResult<bool> {
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
    /// Others might return based on the config or the run parameters.
    fn is_resource_groups_split_in_change_set_capable(&self) -> bool {
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
    fn resource_group_size(&self, group_key: &Self::GroupKey)
        -> PartialVMResult<ResourceGroupSize>;

    fn get_resource_from_group(
        &self,
        group_key: &Self::GroupKey,
        resource_tag: &Self::ResourceTag,
        maybe_layout: Option<&Self::Layout>,
    ) -> PartialVMResult<Option<Bytes>>;

    /// Needed for charging storage fees for a resource group write, as that requires knowing
    /// the size of the resource group AFTER the changeset of the transaction is applied (while
    /// the resource_group_size method provides the total group size BEFORE). To compute the
    /// AFTER size, for each modified resources within the group, the prior size can be
    /// determined by the following method (returns 0 for non-existent / deleted resources).
    fn resource_size_in_group(
        &self,
        group_key: &Self::GroupKey,
        resource_tag: &Self::ResourceTag,
    ) -> PartialVMResult<usize> {
        Ok(self
            .get_resource_from_group(group_key, resource_tag, None)?
            .map_or(0, |bytes| bytes.len()))
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
    ) -> PartialVMResult<bool> {
        self.get_resource_from_group(group_key, resource_tag, None)
            .map(|maybe_bytes| maybe_bytes.is_some())
    }

    fn release_group_cache(
        &self,
    ) -> Option<HashMap<Self::GroupKey, BTreeMap<Self::ResourceTag, Bytes>>>;
}

/// Allows to query state information, e.g. its usage.
pub trait StateStorageView {
    type Key;

    fn id(&self) -> StateViewId;

    /// Reads the state value from the DB. Used to enforce read-before-write for module writes.
    fn read_state_value(&self, state_key: &Self::Key) -> Result<(), StateViewError>;

    fn get_usage(&self) -> Result<StateStorageUsage, StateViewError>;
}

/// A fine-grained view of the state during execution.
///
/// - The `StateView` trait should be used by the storage backend, e.g. a DB.
///   It only allows a generic key-value access and always returns bytes or
///   state values.
/// - The `ExecutorView` trait is used at executor level, e.g. BlockSTM. When
///   a block is executed, the types of accesses are always known (for example,
///   whether a resource is accessed). Fine-grained structure of `ExecutorView`
///   allows to:
///     1. Specialize on access type,
///     2. Separate execution and storage abstractions.
///
/// StateView currently has a basic implementation of the ExecutorView trait,
/// which is used across tests and basic applications in the system.
/// TODO: audit and reconsider the default implementation (e.g. should not
/// resolve AggregatorV2 via the state-view based default implementation, as it
/// doesn't provide a value exchange functionality).
pub trait TExecutorView<K, T, L, V>:
    TResourceView<Key = K, Layout = L>
    + TAggregatorV1View<Identifier = K>
    + TDelayedFieldView<Identifier = DelayedFieldID, ResourceKey = K, ResourceGroupTag = T>
    + TResourceGroupView<GroupKey = K, ResourceTag = T, Layout = L>
    + StateStorageView<Key = K>
{
}

impl<A, K, T, L, V> TExecutorView<K, T, L, V> for A where
    A: TResourceView<Key = K, Layout = L>
        + TAggregatorV1View<Identifier = K>
        + TDelayedFieldView<Identifier = DelayedFieldID, ResourceKey = K, ResourceGroupTag = T>
        + TResourceGroupView<GroupKey = K, ResourceTag = T, Layout = L>
        + StateStorageView<Key = K>
{
}

pub trait ExecutorView:
    TExecutorView<StateKey, StructTag, MoveTypeLayout, WriteOp> + TableResolver + ConfigStorage
{
}

impl<T> ExecutorView for T where
    T: TExecutorView<StateKey, StructTag, MoveTypeLayout, WriteOp> + TableResolver + ConfigStorage
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

impl<'s, S> TResourceView for ExecutorViewAdapter<'s, S>
where
    S: StateView,
{
    type Key = StateKey;
    type Layout = MoveTypeLayout;

    fn get_resource_state_value(
        &self,
        state_key: &Self::Key,
        _maybe_layout: Option<&Self::Layout>,
    ) -> PartialVMResult<Option<StateValue>> {
        self.state_view().get_state_value(state_key).map_err(|e| {
            PartialVMError::new(StatusCode::STORAGE_ERROR).with_message(format!(
                "Unexpected storage error for resource at {:?}: {:?}",
                state_key, e
            ))
        })
    }
}

impl<'s, S> TResourceGroupView for ExecutorViewAdapter<'s, S>
where
    S: StateView,
{
    type GroupKey = StateKey;
    type Layout = MoveTypeLayout;
    type ResourceTag = StructTag;

    fn resource_group_size(
        &self,
        group_key: &Self::GroupKey,
    ) -> PartialVMResult<ResourceGroupSize> {
        if self.group_size_kind() == GroupSizeKind::None {
            return Ok(ResourceGroupSize::zero_concrete());
        }

        self.load_group_to_cache(group_key)?;
        Ok(self
            .group_cache()
            .borrow()
            .get(group_key)
            .expect("Must be cached")
            .1)
    }

    fn get_resource_from_group(
        &self,
        group_key: &Self::GroupKey,
        resource_tag: &Self::ResourceTag,
        _maybe_layout: Option<&Self::Layout>,
    ) -> PartialVMResult<Option<Bytes>> {
        self.load_group_to_cache(group_key)?;
        Ok(self
            .group_cache()
            .borrow()
            .get(group_key)
            .expect("Must be cached")
            .0
            .get(resource_tag)
            .cloned())
    }

    fn release_group_cache(
        &self,
    ) -> Option<HashMap<Self::GroupKey, BTreeMap<Self::ResourceTag, Bytes>>> {
        // Returning the contents to the caller leads to preparing the change set in the backwards
        // compatible way (containing the whole group update).
        Some(
            self.group_cache()
                .borrow_mut()
                .drain()
                .map(|(k, v)| (k, v.0))
                .collect(),
        )
    }
}

impl<'s, S> StateStorageView for ExecutorViewAdapter<'s, S>
where
    S: StateView,
{
    type Key = StateKey;

    fn id(&self) -> StateViewId {
        self.state_view().id()
    }

    fn read_state_value(&self, state_key: &Self::Key) -> Result<(), StateViewError> {
        self.state_view().get_state_value(state_key)?;
        Ok(())
    }

    fn get_usage(&self) -> Result<StateStorageUsage, StateViewError> {
        self.state_view().get_usage().map_err(Into::into)
    }
}

impl<'s, S> BlockSynchronizationKillSwitch for ExecutorViewAdapter<'s, S>
where
    S: StateView,
{
    fn interrupt_requested(&self) -> bool {
        false
    }
}
