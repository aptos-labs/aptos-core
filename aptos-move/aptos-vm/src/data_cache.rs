// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0
//! Scratchpad for on chain values during the execution.

use crate::{
    aptos_vm_impl::gas_config,
    move_vm_ext::{get_max_binary_format_version, AptosMoveResolver, AsExecutorView},
    storage_adapter::ExecutorViewBase,
};
#[allow(unused_imports)]
use anyhow::Error;
use aptos_aggregator::resolver::{AggregatorReadMode, TAggregatorView};
use aptos_state_view::{StateView, StateViewId};
use aptos_table_natives::{TableHandle, TableResolver};
use aptos_types::{
    access_path::AccessPath,
    aggregator::AggregatorID,
    on_chain_config::{ConfigStorage, Features, OnChainConfig},
    state_store::{
        state_key::StateKey,
        state_storage_usage::StateStorageUsage,
        state_value::{StateValue, StateValueMetadataKind},
    },
};
use aptos_vm_types::resolver::{ExecutorView, StateStorageView, StateValueMetadataResolver};
use bytes::Bytes;
use claims::assert_none;
use move_binary_format::{errors::*, CompiledModule};
use move_core_types::{
    account_address::AccountAddress,
    language_storage::{ModuleId, StructTag},
    metadata::Metadata,
    resolver::{resource_size, ModuleResolver, ResourceResolver},
    vm_status::StatusCode,
};
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap},
    ops::Deref,
};

pub(crate) fn get_resource_group_from_metadata(
    struct_tag: &StructTag,
    metadata: &[Metadata],
) -> Option<StructTag> {
    let metadata = aptos_framework::get_metadata(metadata)?;
    metadata
        .struct_attributes
        .get(struct_tag.name.as_ident_str().as_str())?
        .iter()
        .find_map(|attr| attr.get_resource_group_member())
}

// Allows to keep a single `StorageAdapter` for both borrowed or owned views.
// For example, views are typically borrowed during block execution, but are
// owned in tests or in indexer.
// We also do not use `std::borrow::CoW` because otherwise `E` (which is the
// executor view) has to implement `Clone`.
enum ExecutorViewKind<'e, E: 'e> {
    Borrowed(&'e E),
    Owned(E),
}

impl<E> Deref for ExecutorViewKind<'_, E> {
    type Target = E;

    fn deref(&self) -> &Self::Target {
        match *self {
            ExecutorViewKind::Borrowed(e) => e,
            ExecutorViewKind::Owned(ref e) => e,
        }
    }
}

/// Adapter to convert a `ExecutorView` into a `AptosMoveResolver`.
pub struct StorageAdapter<'e, E> {
    // Underlying storage backend, borrowed or owned.
    executor_view: ExecutorViewKind<'e, E>,
    // When set, and if the resource group was not cached, the serialized resource
    // group size in bytes is added to the size of the resource from the group
    // (returned for gas purposes).
    accurate_byte_count: bool,
    // When set, accurate_byte_count must also be set, but the resource group size
    // is computed as the sum of sizes of all resources in the resource group, plus
    // the serialized sizes of the tags. This avoids dependency on group serialization.
    group_byte_count_as_sum: bool,
    max_binary_format_version: u32,
    resource_group_cache: RefCell<HashMap<StateKey, BTreeMap<StructTag, Bytes>>>,
}

impl<'e, E: ExecutorView> StorageAdapter<'e, E> {
    fn init(mut self, features: &Features, gas_feature_version: u64) -> Self {
        if gas_feature_version >= 9 {
            if gas_feature_version >= 12 {
                self.group_byte_count_as_sum = true;
            }
            self.accurate_byte_count = true;
        }
        self.max_binary_format_version =
            get_max_binary_format_version(features, gas_feature_version);

        self
    }

    pub(crate) fn from_borrowed_with_cached_config(
        executor_view: &'e E,
        gas_feature_version: u64,
        features: &Features,
    ) -> Self {
        let executor_view = ExecutorViewKind::Borrowed(executor_view);
        Self::new_with_cached_config(executor_view, gas_feature_version, features)
    }

    fn new_with_cached_config(
        executor_view: ExecutorViewKind<'e, E>,
        gas_feature_version: u64,
        features: &Features,
    ) -> Self {
        let s = Self {
            executor_view,
            accurate_byte_count: false,
            group_byte_count_as_sum: false,
            max_binary_format_version: 0,
            resource_group_cache: RefCell::new(HashMap::new()),
        };
        s.init(features, gas_feature_version)
    }

    pub(crate) fn from_owned(executor_view: E) -> Self {
        let executor_view = ExecutorViewKind::Owned(executor_view);
        Self::new(executor_view)
    }

    pub(crate) fn from_borrowed(executor_view: &'e E) -> Self {
        let executor_view = ExecutorViewKind::Borrowed(executor_view);
        Self::new(executor_view)
    }

    fn new(executor_view: ExecutorViewKind<'e, E>) -> Self {
        let s = Self {
            executor_view,
            accurate_byte_count: false,
            group_byte_count_as_sum: false,
            max_binary_format_version: 0,
            resource_group_cache: RefCell::new(HashMap::new()),
        };
        let (_, gas_feature_version) = gas_config(&s);
        let features = Features::fetch_config(&s).unwrap_or_default();
        s.init(&features, gas_feature_version)
    }

    fn get_any_resource(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
        metadata: &[Metadata],
    ) -> Result<(Option<Bytes>, usize), VMError> {
        let resource_group = get_resource_group_from_metadata(struct_tag, metadata);
        if let Some(resource_group) = resource_group {
            let key = StateKey::access_path(AccessPath::resource_group_access_path(
                *address,
                resource_group.clone(),
            ));

            if let Some(group_data) = self.resource_group_cache.borrow_mut().get_mut(&key) {
                let buf = group_data.get(struct_tag).cloned();
                let buf_size = resource_size(&buf);
                return Ok((buf, buf_size));
            }

            let (buf, maybe_group_size) = self
                .get_resource_from_group(&key, struct_tag, self.accurate_byte_count)
                .map_err(|e| {
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message(format!("{}", e))
                        .finish(Location::Undefined)
                })?;

            let buf_size = resource_size(&buf);
            Ok((buf, buf_size + maybe_group_size.unwrap_or(0)))
        } else {
            let access_path = AccessPath::resource_access_path(*address, struct_tag.clone())
                .map_err(|_| {
                    PartialVMError::new(StatusCode::TOO_MANY_TYPE_NODES).finish(Location::Undefined)
                })?;

            let buf = self
                .executor_view
                .get_resource_bytes(&StateKey::access_path(access_path), None)
                .map_err(|_| {
                    PartialVMError::new(StatusCode::STORAGE_ERROR).finish(Location::Undefined)
                })?;
            let buf_size = resource_size(&buf);
            Ok((buf, buf_size))
        }
    }

    fn get_resource_from_group(
        &self,
        key: &StateKey,
        resource_tag: &StructTag,
        return_group_size: bool,
    ) -> anyhow::Result<(Option<Bytes>, Option<usize>)> {
        // Resolve directly from state store (ExecutorView interface).
        let group_data = self.executor_view.get_resource_bytes(key, None)?;
        if let Some(group_data_blob) = group_data {
            let group_data: BTreeMap<StructTag, Bytes> = bcs::from_bytes(&group_data_blob)
                .map_err(|_| anyhow::Error::msg("Resource group deserialization error"))?;

            let maybe_group_size = if return_group_size {
                Some(
                    if self.group_byte_count_as_sum {
                        // Computing the size based on the sizes of the elements in group_data.
                        group_data
                            .iter()
                            .try_fold(0, |len, (tag, res)| {
                                let delta = bcs::serialized_size(tag)? + res.len();
                                Ok(len + delta)
                            })
                            .map_err(|_: Error| {
                                anyhow::Error::msg("Resource group member tag serialization error")
                            })?
                    } else if self.accurate_byte_count {
                        // Computing the size based on the serialized length of group_data.
                        group_data_blob.len()
                    } else {
                        0
                    },
                )
            } else {
                None
            };

            let res = group_data.get(resource_tag).cloned();

            assert_none!(self
                .resource_group_cache
                .borrow_mut()
                .insert(key.clone(), group_data));

            Ok((res, maybe_group_size))
        } else {
            Ok((None, None))
        }
    }
}

impl<'e, E: ExecutorView> AptosMoveResolver for StorageAdapter<'e, E> {
    fn release_resource_group_cache(&self) -> HashMap<StateKey, BTreeMap<StructTag, Bytes>> {
        self.resource_group_cache.take()
    }
}

impl<'e, E: ExecutorView> ResourceResolver for StorageAdapter<'e, E> {
    fn get_resource_with_metadata(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
        metadata: &[Metadata],
    ) -> anyhow::Result<(Option<Bytes>, usize)> {
        Ok(self.get_any_resource(address, struct_tag, metadata)?)
    }
}

impl<'e, E: ExecutorView> ModuleResolver for StorageAdapter<'e, E> {
    fn get_module_metadata(&self, module_id: &ModuleId) -> Vec<Metadata> {
        let module_bytes = match self.get_module(module_id) {
            Ok(Some(bytes)) => bytes,
            _ => return vec![],
        };
        let module = match CompiledModule::deserialize_with_max_version(
            &module_bytes,
            self.max_binary_format_version,
        ) {
            Ok(module) => module,
            _ => return vec![],
        };
        module.metadata
    }

    fn get_module(&self, module_id: &ModuleId) -> Result<Option<Bytes>, Error> {
        let access_path = AccessPath::from(module_id);
        Ok(self
            .executor_view
            .get_module_bytes(&StateKey::access_path(access_path))
            .map_err(|_| {
                PartialVMError::new(StatusCode::STORAGE_ERROR).finish(Location::Undefined)
            })?)
    }
}

impl<'e, E: ExecutorView> TableResolver for StorageAdapter<'e, E> {
    fn resolve_table_entry(
        &self,
        handle: &TableHandle,
        key: &[u8],
    ) -> Result<Option<Bytes>, Error> {
        self.executor_view
            .get_resource_bytes(&StateKey::table_item((*handle).into(), key.to_vec()), None)
    }
}

impl<'e, E: ExecutorView> TAggregatorView for StorageAdapter<'e, E> {
    type IdentifierV1 = StateKey;
    type IdentifierV2 = AggregatorID;

    fn get_aggregator_v1_state_value(
        &self,
        id: &Self::IdentifierV1,
        mode: AggregatorReadMode,
    ) -> anyhow::Result<Option<StateValue>> {
        self.executor_view.get_aggregator_v1_state_value(id, mode)
    }
}

impl<'e, E: ExecutorView> ConfigStorage for StorageAdapter<'e, E> {
    fn fetch_config(&self, access_path: AccessPath) -> Option<Bytes> {
        self.executor_view
            .get_resource_bytes(&StateKey::access_path(access_path), None)
            .ok()?
    }
}

/// Converts `StateView` into `AptosMoveResolver`.
pub trait AsMoveResolver<S> {
    fn as_move_resolver(&self) -> StorageAdapter<ExecutorViewBase<S>>;
}

impl<S: StateView> AsMoveResolver<S> for S {
    fn as_move_resolver(&self) -> StorageAdapter<ExecutorViewBase<S>> {
        StorageAdapter::from_owned(ExecutorViewBase::new(self))
    }
}

impl<'e, E: ExecutorView> StateStorageView for StorageAdapter<'e, E> {
    fn id(&self) -> StateViewId {
        self.executor_view.id()
    }

    fn get_usage(&self) -> anyhow::Result<StateStorageUsage> {
        self.executor_view.get_usage()
    }
}

impl<'e, E: ExecutorView> StateValueMetadataResolver for StorageAdapter<'e, E> {
    fn get_module_state_value_metadata(
        &self,
        state_key: &StateKey,
    ) -> anyhow::Result<Option<StateValueMetadataKind>> {
        self.executor_view
            .get_module_state_value_metadata(state_key)
    }

    fn get_resource_state_value_metadata(
        &self,
        state_key: &StateKey,
    ) -> anyhow::Result<Option<StateValueMetadataKind>> {
        self.executor_view
            .get_resource_state_value_metadata(state_key)
    }

    fn get_resource_group_state_value_metadata(
        &self,
        _state_key: &StateKey,
    ) -> anyhow::Result<Option<StateValueMetadataKind>> {
        // TODO: forward to self.executor_view.
        unimplemented!("Resource group metadata handling not yet implemented");
    }
}

// Allows to extract the view from `StorageAdapter`.
impl<'e, E: ExecutorView> AsExecutorView for StorageAdapter<'e, E> {
    fn as_executor_view(&self) -> &dyn ExecutorView {
        self.executor_view.deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_state_view::TStateView;
    use claims::{assert_gt, assert_lt, assert_some, assert_some_eq};
    use move_core_types::{identifier::Identifier, language_storage::TypeTag};
    use std::cmp::max;

    struct MockGroup {
        blob: Vec<u8>,
        size_as_sum: usize,
    }

    impl MockGroup {
        fn new(contents: BTreeMap<StructTag, Vec<u8>>) -> Self {
            let mut size_as_sum = 0;
            for (tag, v) in &contents {
                // Compute size indirectly, by first serializing.
                let serialized_tag = bcs::to_bytes(&tag).unwrap();
                size_as_sum += v.len() + serialized_tag.len();
            }
            let blob = bcs::to_bytes(&contents).unwrap();

            Self { blob, size_as_sum }
        }
    }

    struct MockStateView {
        group: HashMap<StateKey, MockGroup>,
    }

    impl MockStateView {
        fn new() -> Self {
            let key_0 = StateKey::raw(vec![0]);
            let key_1 = StateKey::raw(vec![1]);

            let mut group = HashMap::new();
            // for testing purposes, o.w. state view should never contain an empty map.
            group.insert(key_0, MockGroup::new(BTreeMap::new()));
            group.insert(
                key_1,
                MockGroup::new(BTreeMap::from([
                    (tag_0(), vec![0; 1000]),
                    (tag_1(), vec![1; 500]),
                ])),
            );

            Self { group }
        }
    }

    impl TStateView for MockStateView {
        type Key = StateKey;

        fn get_state_value(&self, state_key: &Self::Key) -> anyhow::Result<Option<StateValue>> {
            Ok(self
                .group
                .get(state_key)
                .map(|entry| StateValue::new_legacy(entry.blob.clone().into())))
        }

        fn get_usage(&self) -> anyhow::Result<StateStorageUsage> {
            unimplemented!();
        }
    }

    fn tag_0() -> StructTag {
        StructTag {
            address: AccountAddress::ONE,
            module: Identifier::new("a").unwrap(),
            name: Identifier::new("a").unwrap(),
            type_params: vec![TypeTag::U8],
        }
    }

    fn tag_1() -> StructTag {
        StructTag {
            address: AccountAddress::ONE,
            module: Identifier::new("abcde").unwrap(),
            name: Identifier::new("fgh").unwrap(),
            type_params: vec![TypeTag::U64],
        }
    }

    fn tag_2() -> StructTag {
        StructTag {
            address: AccountAddress::ONE,
            module: Identifier::new("abcdex").unwrap(),
            name: Identifier::new("fghx").unwrap(),
            type_params: vec![TypeTag::U128],
        }
    }

    #[test]
    fn test_version_flags() {
        let state_view = MockStateView::new();
        let mut s = state_view.as_move_resolver();

        assert!(!s.accurate_byte_count);
        assert!(!s.group_byte_count_as_sum);
        for i in 0..9 {
            s = s.init(&Features::default(), i);
            assert!(!s.accurate_byte_count);
            assert!(!s.group_byte_count_as_sum);
        }

        for i in 9..12 {
            s = s.init(&Features::default(), i);
            assert!(s.accurate_byte_count);
            assert!(!s.group_byte_count_as_sum);
        }

        for i in 12..20 {
            s = s.init(&Features::default(), i);
            assert!(s.accurate_byte_count);
            assert!(s.group_byte_count_as_sum);
        }
    }

    #[test]
    #[should_panic]
    fn test_already_cached() {
        let state_view = MockStateView::new();
        let s = state_view.as_move_resolver();

        let tag_0 = tag_0();
        let tag_1 = tag_1();
        let key_1 = StateKey::raw(vec![1]);

        let _ = s.get_resource_from_group(&key_1, &tag_0, false);
        // key_1 group is cached, and when cached, get_resource_from_group may not be called.
        let _ = s.get_resource_from_group(&key_1, &tag_1, false);
    }

    #[test]
    fn test_get_resource_by_tag() {
        let state_view = MockStateView::new();
        let s = state_view.as_move_resolver();

        let key_0 = StateKey::raw(vec![0]);
        let key_1 = StateKey::raw(vec![1]);
        let key_2 = StateKey::raw(vec![2]);
        let tag_0 = tag_0();
        let tag_1 = tag_1();
        let tag_2 = tag_2();

        let (maybe_bytes, maybe_size) = s.get_resource_from_group(&key_0, &tag_0, false).unwrap();
        // key_0 / tag_0 does not exist.
        assert_none!(maybe_size);
        assert_none!(maybe_bytes);

        let (maybe_bytes, maybe_size) = s.get_resource_from_group(&key_1, &tag_0, false).unwrap();
        assert_none!(maybe_size);
        let bytes = maybe_bytes.expect("key_1 / tag_0 must exist");
        assert_eq!(bytes, vec![0; 1000]);

        let (maybe_bytes, maybe_size) = s.get_resource_from_group(&key_2, &tag_1, false).unwrap();
        // key_2 / tag_1 does not exist.
        assert_none!(maybe_size);
        assert_none!(maybe_bytes);

        let key_1_blob = &state_view.group.get(&key_1).unwrap().blob;

        // Release the cache to test contents, and to avoid assert when querying key_1 again.
        let cache = s.release_resource_group_cache();
        assert_eq!(cache.len(), 2);
        assert_some!(cache.get(&key_0));
        let cache_key_1_contents = cache.get(&key_1).unwrap();
        assert_eq!(bcs::to_bytes(&cache_key_1_contents).unwrap(), *key_1_blob);

        let (maybe_bytes, maybe_size) = s.get_resource_from_group(&key_1, &tag_1, false).unwrap();
        assert_none!(maybe_size);
        let bytes = maybe_bytes.expect("key_1 / tag_1 must exist");
        assert_eq!(bytes, vec![1; 500]);

        // Release the cache to test contents, and to avoid assert when querying key_1 again.
        let cache = s.release_resource_group_cache();
        assert_eq!(cache.len(), 1);
        assert_some!(cache.get(&key_1));

        let (maybe_bytes, maybe_size) = s.get_resource_from_group(&key_1, &tag_2, false).unwrap();
        assert_none!(maybe_size);
        assert_none!(maybe_bytes);

        // still cached.
        let cache = s.release_resource_group_cache();
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_size_by_blob_len() {
        let state_view = MockStateView::new();
        let mut s = state_view.as_move_resolver();
        s = s.init(&Features::default(), 10);
        // Tested separately, but re-confirm for the sanity of this test.
        assert!(s.accurate_byte_count);
        assert!(!s.group_byte_count_as_sum);

        let key_1 = StateKey::raw(vec![1]);
        let tag_0 = tag_0();
        let tag_1 = tag_1();
        let tag_2 = tag_2();

        let key_1_blob = &state_view.group.get(&key_1).unwrap().blob;

        let (maybe_bytes, maybe_size) = s.get_resource_from_group(&key_1, &tag_0, true).unwrap();
        assert_some_eq!(maybe_size, key_1_blob.len());
        let bytes = maybe_bytes.expect("key_1 / tag_0 must exist");
        assert_eq!(bytes, vec![0; 1000]);

        // Release the cache to test contents, and to avoid assert when querying key_1 again.
        let cache = s.release_resource_group_cache();
        assert_eq!(cache.len(), 1);
        let cache_key_1_contents = cache.get(&key_1).unwrap();
        assert_eq!(bcs::to_bytes(&cache_key_1_contents).unwrap(), *key_1_blob);

        let (maybe_bytes, maybe_size) = s.get_resource_from_group(&key_1, &tag_1, true).unwrap();
        assert_some_eq!(maybe_size, key_1_blob.len());
        let bytes = maybe_bytes.expect("key_1 / tag_1 must exist");
        assert_eq!(bytes, vec![1; 500]);

        // Release the cache to test contents, and to avoid assert when querying key_1 again.
        let cache = s.release_resource_group_cache();
        assert_eq!(cache.len(), 1);
        assert_some!(cache.get(&key_1));

        let (maybe_bytes, maybe_size) = s.get_resource_from_group(&key_1, &tag_2, true).unwrap();
        // Should still return size, even if tag is not found!
        assert_some_eq!(maybe_size, key_1_blob.len());
        assert_none!(maybe_bytes);
    }

    #[test]
    fn test_size_as_sum() {
        let state_view = MockStateView::new();
        let mut s = state_view.as_move_resolver();
        s = s.init(&Features::default(), 20);
        // Tested separately, but re-confirm for the sanity of this test.
        assert!(s.accurate_byte_count);
        assert!(s.group_byte_count_as_sum);

        let key_1 = StateKey::raw(vec![1]);
        let tag_0 = tag_0();
        let tag_1 = tag_1();
        let tag_2 = tag_2();

        let key_1_size_as_sum = state_view.group.get(&key_1).unwrap().size_as_sum;

        let (maybe_bytes, maybe_size) = s.get_resource_from_group(&key_1, &tag_0, true).unwrap();
        assert_some_eq!(maybe_size, key_1_size_as_sum);
        let bytes = maybe_bytes.expect("key_1 / tag_0 must exist");
        assert_eq!(bytes, vec![0; 1000]);

        // Release the cache to test contents, and to avoid assert when querying key_1 again.
        let cache = s.release_resource_group_cache();
        assert_eq!(cache.len(), 1);
        assert_some!(cache.get(&key_1));

        let (maybe_bytes, maybe_size) = s.get_resource_from_group(&key_1, &tag_1, true).unwrap();
        assert_some_eq!(maybe_size, key_1_size_as_sum);
        let bytes = maybe_bytes.expect("key_1 / tag_1 must exist");
        assert_eq!(bytes, vec![1; 500]);

        // Release the cache to test contents, and to avoid assert when querying key_1 again.
        let cache = s.release_resource_group_cache();
        assert_eq!(cache.len(), 1);
        assert_some!(cache.get(&key_1));

        let (maybe_bytes, maybe_size) = s.get_resource_from_group(&key_1, &tag_2, true).unwrap();
        // Should still return size, even if tag is not found!
        assert_some_eq!(maybe_size, key_1_size_as_sum);
        assert_none!(maybe_bytes);

        // Sanity check the size numbers, at the time of writing the test 1582 and 1587.
        let key_1_blob_size = state_view.group.get(&key_1).unwrap().blob.len();
        assert_lt!(
            key_1_size_as_sum,
            key_1_blob_size,
            "size as sum must be less than BTreeMap blob size",
        );
        assert_gt!(
            key_1_size_as_sum,
            max(1000, key_1_blob_size - 100),
            "size as sum may not be too small"
        );
    }

    // TODO: bring back with proper implementation of resource exists in a group. Storage adapter
    // needs this because resource group change-set preparation will check existence.
    // #[test]
    // fn test_exists_resource_in_group() {
    //     let state_view = MockStateView::new();
    //     let mut s = state_view.as_move_resolver();
    //     s = s.init(&Features::default(), 10);
    //     // Tested separately, but re-confirm for the sanity of this test.
    //     assert!(s.accurate_byte_count);
    //     assert!(!s.group_byte_count_as_sum);

    //     let key_1 = StateKey::raw(vec![1]);
    //     let tag_0 = tag_0();
    //     let tag_1 = tag_1();
    //     let tag_2 = tag_2();

    //     // TODO: test when we implement the group methods.
    //     assert_ok_eq!(s.resource_exists_in_group(&key_1, &tag_0), true);

    //     // Release the cache to test contents, and to avoid assert when querying key_1 again.
    //     let cache = s.release_resource_group_cache();
    //     assert_eq!(cache.len(), 1);
    //     assert_some!(cache.get(&key_1));

    //     assert_ok_eq!(s.resource_exists_in_group(&key_1, &tag_1), true);
    //     // Release the cache to test contents, and to avoid assert when querying key_1 again.
    //     let cache = s.release_resource_group_cache();
    //     assert_eq!(cache.len(), 1);
    //     assert_some!(cache.get(&key_1));

    //     assert_ok_eq!(s.resource_exists_in_group(&key_1, &tag_2), false);
    // }
}
