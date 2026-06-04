// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! VM-boundary recorder for hot-state access observation.
//!
//! [`HotnessReadRecorder`] wraps the block executor's `ExecutorView` + `ResourceGroupView` and
//! records every state key it observes (value, metadata, exists, size, resource-group, and
//! aggregator-v1 reads) into a deterministic `BTreeSet`. The recorded set is the authoritative
//! hotness-read signal for a committed transaction; it is intentionally separate from Block-STM's
//! conflict-oriented captured reads, which (a) are speculative, (b) use non-deterministic hash
//! ordering, and (c) deliberately exclude metadata/exists/size reads.
//!
//! Module reads are NOT observed here: modules are resolved through the code storage / module cache
//! (a blanket-implemented trait that cannot be wrapped without re-implementing the cache layers),
//! and module loads served from the warm cross-block cache never reach this view. Module hotness is
//! therefore unioned in from the block executor's module-read tracking at feed time. Delayed-field
//! IDs do not map to hot-state KV keys and are intentionally not recorded.

use aptos_aggregator::{
    bounded_math::SignedU128,
    resolver::{TAggregatorV1View, TDelayedFieldView},
    types::{DelayedFieldValue, DelayedFieldsSpeculativeError},
};
use aptos_types::{
    error::{PanicError, PanicOr},
    state_store::{
        errors::StateViewError,
        state_key::StateKey,
        state_storage_usage::StateStorageUsage,
        state_value::{StateValue, StateValueMetadata},
        StateViewId,
    },
};
use aptos_vm_types::resolver::{
    ExecutorView, ResourceGroupSize, ResourceGroupView, StateStorageView, TResourceGroupView,
    TResourceView,
};
use bytes::Bytes;
use move_binary_format::errors::PartialVMResult;
use move_core_types::{language_storage::StructTag, value::MoveTypeLayout};
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    mem,
};
use triomphe::Arc as TriompheArc;

/// See module documentation.
pub(crate) struct HotnessReadRecorder<'a, R> {
    inner: &'a R,
    reads: RefCell<BTreeSet<StateKey>>,
}

impl<'a, R> HotnessReadRecorder<'a, R> {
    pub(crate) fn new(inner: &'a R) -> Self {
        Self {
            inner,
            reads: RefCell::new(BTreeSet::new()),
        }
    }

    /// Drains and returns the keys observed so far. Called once after VM execution finishes.
    pub(crate) fn take_reads(&self) -> BTreeSet<StateKey> {
        mem::take(&mut self.reads.borrow_mut())
    }

    fn record(&self, state_key: &StateKey) {
        self.reads.borrow_mut().insert(state_key.clone());
    }
}

impl<R: ExecutorView + ResourceGroupView> TResourceView for HotnessReadRecorder<'_, R> {
    type Key = StateKey;
    type Layout = MoveTypeLayout;

    fn get_resource_state_value(
        &self,
        state_key: &StateKey,
        maybe_layout: Option<&MoveTypeLayout>,
    ) -> PartialVMResult<Option<StateValue>> {
        self.record(state_key);
        self.inner.get_resource_state_value(state_key, maybe_layout)
    }

    fn get_resource_state_value_metadata(
        &self,
        state_key: &StateKey,
    ) -> PartialVMResult<Option<StateValueMetadata>> {
        self.record(state_key);
        self.inner.get_resource_state_value_metadata(state_key)
    }

    fn get_resource_state_value_size(&self, state_key: &StateKey) -> PartialVMResult<u64> {
        self.record(state_key);
        self.inner.get_resource_state_value_size(state_key)
    }

    fn resource_exists(&self, state_key: &StateKey) -> PartialVMResult<bool> {
        self.record(state_key);
        self.inner.resource_exists(state_key)
    }
}

impl<R: ExecutorView + ResourceGroupView> TAggregatorV1View for HotnessReadRecorder<'_, R> {
    type Identifier = StateKey;

    fn get_aggregator_v1_state_value(&self, id: &StateKey) -> PartialVMResult<Option<StateValue>> {
        self.record(id);
        self.inner.get_aggregator_v1_state_value(id)
    }
}

impl<R: ExecutorView + ResourceGroupView> TDelayedFieldView for HotnessReadRecorder<'_, R> {
    type Identifier = DelayedFieldID;
    type ResourceGroupTag = StructTag;
    type ResourceKey = StateKey;

    fn get_delayed_field_value(
        &self,
        id: &DelayedFieldID,
    ) -> Result<DelayedFieldValue, PanicOr<DelayedFieldsSpeculativeError>> {
        self.inner.get_delayed_field_value(id)
    }

    fn delayed_field_try_add_delta_outcome(
        &self,
        id: &DelayedFieldID,
        base_delta: &SignedU128,
        delta: &SignedU128,
        max_value: u128,
    ) -> Result<bool, PanicOr<DelayedFieldsSpeculativeError>> {
        self.inner
            .delayed_field_try_add_delta_outcome(id, base_delta, delta, max_value)
    }

    fn generate_delayed_field_id(&self, width: u32) -> DelayedFieldID {
        self.inner.generate_delayed_field_id(width)
    }

    fn validate_delayed_field_id(&self, id: &DelayedFieldID) -> Result<(), PanicError> {
        self.inner.validate_delayed_field_id(id)
    }

    fn get_reads_needing_exchange(
        &self,
        delayed_write_set_ids: &HashSet<DelayedFieldID>,
        skip: &HashSet<StateKey>,
    ) -> Result<
        BTreeMap<StateKey, (StateValueMetadata, u64, TriompheArc<MoveTypeLayout>)>,
        PanicError,
    > {
        self.inner
            .get_reads_needing_exchange(delayed_write_set_ids, skip)
    }

    fn get_group_reads_needing_exchange(
        &self,
        delayed_write_set_ids: &HashSet<DelayedFieldID>,
        skip: &HashSet<StateKey>,
    ) -> PartialVMResult<BTreeMap<StateKey, (StateValueMetadata, u64)>> {
        self.inner
            .get_group_reads_needing_exchange(delayed_write_set_ids, skip)
    }
}

impl<R: ExecutorView + ResourceGroupView> StateStorageView for HotnessReadRecorder<'_, R> {
    type Key = StateKey;

    fn id(&self) -> StateViewId {
        self.inner.id()
    }

    fn read_state_value(&self, state_key: &StateKey) -> Result<(), StateViewError> {
        self.record(state_key);
        self.inner.read_state_value(state_key)
    }

    fn get_usage(&self) -> Result<StateStorageUsage, StateViewError> {
        self.inner.get_usage()
    }
}

impl<R: ExecutorView + ResourceGroupView> TResourceGroupView for HotnessReadRecorder<'_, R> {
    type GroupKey = StateKey;
    type Layout = MoveTypeLayout;
    type ResourceTag = StructTag;

    fn is_resource_groups_split_in_change_set_capable(&self) -> bool {
        self.inner.is_resource_groups_split_in_change_set_capable()
    }

    fn resource_group_size(&self, group_key: &StateKey) -> PartialVMResult<ResourceGroupSize> {
        self.record(group_key);
        self.inner.resource_group_size(group_key)
    }

    fn get_resource_from_group(
        &self,
        group_key: &StateKey,
        resource_tag: &StructTag,
        maybe_layout: Option<&MoveTypeLayout>,
    ) -> PartialVMResult<Option<Bytes>> {
        self.record(group_key);
        self.inner
            .get_resource_from_group(group_key, resource_tag, maybe_layout)
    }

    fn resource_size_in_group(
        &self,
        group_key: &StateKey,
        resource_tag: &StructTag,
    ) -> PartialVMResult<usize> {
        self.record(group_key);
        self.inner.resource_size_in_group(group_key, resource_tag)
    }

    fn resource_exists_in_group(
        &self,
        group_key: &StateKey,
        resource_tag: &StructTag,
    ) -> PartialVMResult<bool> {
        self.record(group_key);
        self.inner.resource_exists_in_group(group_key, resource_tag)
    }

    fn release_group_cache(&self) -> Option<HashMap<StateKey, BTreeMap<StructTag, Bytes>>> {
        self.inner.release_group_cache()
    }
}
