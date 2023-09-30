// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::resolver::{TResourceGroupView, TResourceView};
use anyhow::Error;
use aptos_state_view::TStateView;
use aptos_types::{
    access_path::AccessPath, on_chain_config::ConfigStorage, state_store::state_key::StateKey,
};
use bytes::Bytes;
use move_core_types::{language_storage::StructTag, value::MoveTypeLayout};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap},
    fmt::Debug,
    hash::Hash,
};

/// Corresponding to different gas features, methods for counting the 'size' of a
/// resource group. None leads to 0, while AsBlob provides the group size as the
/// size of the serialized blob of the BTreeMap corresponding to the group.
/// For AsSum, the size is summed for each resource contained in the group (of
/// the resource blob, and its corresponding tag, when serialized)
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum GroupSizeKind {
    None,
    AsBlob,
    AsSum,
}

impl GroupSizeKind {
    pub fn from_gas_feature_version(gas_feature_version: u64) -> Self {
        if gas_feature_version >= 9 {
            if gas_feature_version >= 12 {
                GroupSizeKind::AsSum
            } else {
                // Keep old caching behavior for replay.
                GroupSizeKind::AsBlob
            }
        } else {
            GroupSizeKind::None
        }
    }
}

pub enum UnifiedResourceView<'a, K, L> {
    StateView(&'a dyn TStateView<Key = K>),
    ResourceView(&'a dyn TResourceView<Key = K, Layout = L>),
}

impl<'a, K, L> UnifiedResourceView<'a, K, L> {
    fn get_bytes(&self, state_key: &K) -> anyhow::Result<Option<Bytes>> {
        match self {
            UnifiedResourceView::StateView(s) => s.get_state_value_bytes(state_key),
            UnifiedResourceView::ResourceView(r) => r.get_resource_bytes(state_key, None),
        }
    }
}

impl<'a, L> ConfigStorage for UnifiedResourceView<'a, StateKey, L> {
    fn fetch_config(&self, access_path: AccessPath) -> Option<Bytes> {
        self.get_bytes(&StateKey::access_path(access_path)).ok()?
    }
}

pub struct TResourceGroupAdapter<'r, K, T, L>
where
    K: Clone + Eq + Hash,
    T: Debug + DeserializeOwned + Ord + Serialize,
{
    resource_resolver: UnifiedResourceView<'r, K, L>,
    group_size_kind: GroupSizeKind,
    // Caches group size alongside the BTreeMap corresponding to the group for key K.
    group_cache: RefCell<HashMap<K, (BTreeMap<T, Bytes>, u64)>>,
}

impl<'r, K, T, L> TResourceGroupAdapter<'r, K, T, L>
where
    K: Clone + Eq + Hash,
    T: Debug + DeserializeOwned + Ord + Serialize,
{
    pub fn from_resource_view(
        resource_view: &'r dyn TResourceView<Key = K, Layout = L>,
        group_size_kind: GroupSizeKind,
    ) -> Self {
        Self {
            resource_resolver: UnifiedResourceView::ResourceView(resource_view),
            group_size_kind,
            group_cache: RefCell::new(HashMap::new()),
        }
    }

    pub fn from_state_view(
        state_view: &'r dyn TStateView<Key = K>,
        group_size_kind: GroupSizeKind,
    ) -> Self {
        Self {
            resource_resolver: UnifiedResourceView::StateView(state_view),
            group_size_kind,
            group_cache: RefCell::new(HashMap::new()),
        }
    }

    pub fn group_size_kind(&self) -> GroupSizeKind {
        self.group_size_kind.clone()
    }

    fn release_group_cache(&self) -> HashMap<K, BTreeMap<T, Bytes>> {
        self.group_cache
            .borrow_mut()
            .drain()
            .map(|(k, v)| (k, v.0))
            .collect()
    }

    // Ensures that the resource group at state_key is cached in self.group_cache. Ok(true)
    // means the resource was already cached, while Ok(false) means it just got cached.
    fn ensure_cached(&self, state_key: &K) -> anyhow::Result<bool> {
        if self.group_cache.borrow().contains_key(state_key) {
            return Ok(true);
        }

        let group_data_from_resolver = self.resource_resolver.get_bytes(state_key)?;
        let (group_data, blob_len): (BTreeMap<T, Bytes>, u64) = group_data_from_resolver
            .map_or_else(
                || Ok::<_, Error>((BTreeMap::new(), 0)),
                |group_data_blob| {
                    let group_data = bcs::from_bytes(&group_data_blob)
                        .map_err(|_| anyhow::Error::msg("Resource group deserialization error"))?;
                    Ok((group_data, group_data_blob.len() as u64))
                },
            )?;

        let group_size = match self.group_size_kind {
            GroupSizeKind::None => 0,
            GroupSizeKind::AsBlob => blob_len,
            GroupSizeKind::AsSum => group_data
                .iter()
                .try_fold(0, |len, (tag, res)| {
                    let delta = bcs::serialized_size(tag)? + res.len();
                    Ok(len + delta as u64)
                })
                .map_err(|_: Error| {
                    anyhow::Error::msg("Resource group member tag serialization error")
                })?,
        };

        self.group_cache
            .borrow_mut()
            .insert(state_key.clone(), (group_data, group_size));
        Ok(false)
    }
}

pub type ResourceGroupAdapter<'a> = TResourceGroupAdapter<'a, StateKey, StructTag, MoveTypeLayout>;

impl<K, T, L> TResourceGroupView for TResourceGroupAdapter<'_, K, T, L>
where
    K: Clone + Eq + Hash,
    T: Debug + DeserializeOwned + Ord + Serialize,
{
    type Key = K;
    type Layout = L;
    type Tag = T;

    fn resource_group_size(&self, state_key: &Self::Key) -> anyhow::Result<u64> {
        self.ensure_cached(state_key)?;
        Ok(self
            .group_cache
            .borrow()
            .get(state_key)
            .expect("Must be cached")
            .1)
    }

    fn get_resource_from_group(
        &self,
        state_key: &Self::Key,
        resource_tag: &Self::Tag,
        _maybe_layout: Option<&Self::Layout>,
    ) -> anyhow::Result<Option<Bytes>> {
        self.ensure_cached(state_key)?;
        Ok(self
            .group_cache
            .borrow()
            .get(state_key)
            .expect("Must be cached")
            .0 // btreemap
            .get(resource_tag)
            .cloned())
    }

    fn release_naive_group_cache(&self) -> Option<HashMap<K, BTreeMap<T, Bytes>>> {
        Some(self.release_group_cache())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::utils::{mock_tag_0, mock_tag_1, mock_tag_2};
    use aptos_state_view::TStateView;
    use aptos_types::state_store::{
        state_storage_usage::StateStorageUsage, state_value::StateValue,
    };
    use claims::{assert_gt, assert_lt, assert_none, assert_ok_eq, assert_some, assert_some_eq};
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
            let mut group = HashMap::new();

            let key_0 = StateKey::raw(vec![0]);
            let key_1 = StateKey::raw(vec![1]);

            // for testing purposes, o.w. state view should never contain an empty map.
            group.insert(key_0, MockGroup::new(BTreeMap::new()));
            group.insert(
                key_1,
                MockGroup::new(BTreeMap::from([
                    (mock_tag_0(), vec![0; 1000]),
                    (mock_tag_1(), vec![1; 500]),
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

    #[test]
    fn group_size_kind_from_gas_version() {
        for i in 0..9 {
            assert_eq!(
                GroupSizeKind::from_gas_feature_version(i),
                GroupSizeKind::None
            );
        }

        for i in 9..12 {
            assert_eq!(
                GroupSizeKind::from_gas_feature_version(i),
                GroupSizeKind::AsBlob
            );
        }

        for i in 12..20 {
            assert_eq!(
                GroupSizeKind::from_gas_feature_version(i),
                GroupSizeKind::AsSum
            );
        }
    }

    #[test]
    fn ensure_cached() {
        let state_view = MockStateView::new();
        let s = ResourceGroupAdapter::from_state_view(&state_view, GroupSizeKind::None);

        let key_1 = StateKey::raw(vec![1]);
        let tag_0 = mock_tag_0();

        assert_ok_eq!(s.ensure_cached(&key_1), false);
        let _ = s.get_resource_from_group(&key_1, &tag_0, None);
        assert_ok_eq!(s.ensure_cached(&key_1), true);
    }

    #[test]
    fn test_get_resource_by_tag() {
        let state_view = MockStateView::new();
        let s = ResourceGroupAdapter::from_state_view(&state_view, GroupSizeKind::None);

        let key_0 = StateKey::raw(vec![0]);
        let key_1 = StateKey::raw(vec![1]);
        let key_2 = StateKey::raw(vec![2]);
        let tag_0 = mock_tag_0();
        let tag_1 = mock_tag_1();
        let tag_2 = mock_tag_2();

        // key_0 / tag_0 does not exist.
        assert_none!(s.get_resource_from_group(&key_0, &tag_0, None).unwrap());

        assert_some_eq!(
            s.get_resource_from_group(&key_1, &tag_0, None).unwrap(),
            vec![0; 1000]
        );

        // key_2 / tag_1 does not exist.
        assert_none!(s.get_resource_from_group(&key_2, &tag_1, None).unwrap());

        let key_1_blob = &state_view.group.get(&key_1).unwrap().blob;

        // Release the cache to test contents.
        let cache = s.release_group_cache();
        assert_eq!(cache.len(), 3);
        assert!(cache.get(&key_0).expect("Must be Some(..)").is_empty());
        assert!(cache.get(&key_2).expect("Must be Some(..)").is_empty());
        let cache_key_1_contents = cache.get(&key_1).unwrap();
        assert_eq!(bcs::to_bytes(&cache_key_1_contents).unwrap(), *key_1_blob);

        assert_some_eq!(
            s.get_resource_from_group(&key_1, &tag_1, None).unwrap(),
            vec![1; 500]
        );

        assert_none!(s.get_resource_from_group(&key_1, &tag_2, None).unwrap());

        // Test releasing the cache via trait method.
        let cache = s.release_naive_group_cache().unwrap();
        assert_eq!(cache.len(), 1);
        let cache_key_1_contents = cache.get(&key_1).unwrap();
        assert_eq!(bcs::to_bytes(&cache_key_1_contents).unwrap(), *key_1_blob);
    }

    #[test]
    fn size_as_blob_len() {
        let state_view = MockStateView::new();
        let s = ResourceGroupAdapter::from_state_view(&state_view, GroupSizeKind::AsBlob);

        let key_0 = StateKey::raw(vec![0]);
        let key_1 = StateKey::raw(vec![1]);
        let key_2 = StateKey::raw(vec![2]);

        let key_0_blob_len = state_view.group.get(&key_0).unwrap().blob.len() as u64;
        let key_1_blob_len = state_view.group.get(&key_1).unwrap().blob.len() as u64;

        assert_ok_eq!(s.resource_group_size(&key_1), key_1_blob_len);
        // Release the cache to test contents.
        let cache = s.release_group_cache();
        assert_eq!(cache.len(), 1);
        assert_some!(cache.get(&key_1));

        assert_ok_eq!(s.resource_group_size(&key_0), key_0_blob_len);
        assert_ok_eq!(s.resource_group_size(&key_1), key_1_blob_len);
        assert_ok_eq!(s.resource_group_size(&key_2), 0);

        // Test releasing the cache via trait method.
        let cache = s.release_naive_group_cache().unwrap();
        assert_eq!(cache.len(), 3);
        assert_some!(cache.get(&key_0));
        assert_some!(cache.get(&key_1));
        assert_some!(cache.get(&key_2));
    }

    #[test]
    fn size_as_sum() {
        let state_view = MockStateView::new();
        let s = ResourceGroupAdapter::from_state_view(&state_view, GroupSizeKind::AsSum);

        let key_0 = StateKey::raw(vec![0]);
        let key_1 = StateKey::raw(vec![1]);
        let key_2 = StateKey::raw(vec![2]);

        let key_0_size_as_sum = state_view.group.get(&key_0).unwrap().size_as_sum as u64;
        let key_1_size_as_sum = state_view.group.get(&key_1).unwrap().size_as_sum as u64;

        assert_ok_eq!(s.resource_group_size(&key_1), key_1_size_as_sum);
        // Release the cache to test contents.
        let cache = s.release_group_cache();
        assert_eq!(cache.len(), 1);
        assert_some!(cache.get(&key_1));

        assert_ok_eq!(s.resource_group_size(&key_0), key_0_size_as_sum);
        assert_ok_eq!(s.resource_group_size(&key_1), key_1_size_as_sum);
        assert_ok_eq!(s.resource_group_size(&key_2), 0);

        // Test releasing the cache via trait method.
        let cache = s.release_naive_group_cache().unwrap();
        assert_eq!(cache.len(), 3);
        assert_some!(cache.get(&key_0));
        assert_some!(cache.get(&key_1));
        assert_some!(cache.get(&key_2));

        // Sanity check the size numbers, at the time of writing the test 1582 and 1587.
        let key_1_blob_size = state_view.group.get(&key_1).unwrap().blob.len() as u64;
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

    #[test]
    fn size_as_none() {
        let state_view = MockStateView::new();
        let s = ResourceGroupAdapter::from_state_view(&state_view, GroupSizeKind::None);

        let key_0 = StateKey::raw(vec![0]);
        let key_1 = StateKey::raw(vec![1]);
        let key_2 = StateKey::raw(vec![2]);

        assert_ok_eq!(s.resource_group_size(&key_1), 0);
        // Release the cache to test contents.
        let cache = s.release_group_cache();
        assert_eq!(cache.len(), 1);
        assert_some!(cache.get(&key_1));

        assert_ok_eq!(s.resource_group_size(&key_0), 0);
        assert_ok_eq!(s.resource_group_size(&key_1), 0);
        assert_ok_eq!(s.resource_group_size(&key_2), 0);

        // Test releasing the cache via trait method.
        let cache = s.release_naive_group_cache().unwrap();
        assert_eq!(cache.len(), 3);
        assert_some!(cache.get(&key_0));
        assert_some!(cache.get(&key_1));
        assert_some!(cache.get(&key_2));
    }

    #[test]
    fn exists_resource_in_group() {
        let state_view = MockStateView::new();
        let s = ResourceGroupAdapter::from_state_view(&state_view, GroupSizeKind::None);

        let key_0 = StateKey::raw(vec![0]);
        let key_1 = StateKey::raw(vec![1]);
        let key_2 = StateKey::raw(vec![2]);
        let tag_0 = mock_tag_0();
        let tag_1 = mock_tag_1();
        let tag_2 = mock_tag_2();

        assert_ok_eq!(s.resource_exists_in_group(&key_1, &tag_0), true);
        assert_ok_eq!(s.resource_exists_in_group(&key_1, &tag_1), true);
        assert_ok_eq!(s.resource_exists_in_group(&key_2, &tag_2), false);
        // Release the cache to test contents.
        let cache = s.release_group_cache();
        assert_eq!(cache.len(), 2);
        assert_some!(cache.get(&key_1));
        assert_some!(cache.get(&key_2));

        assert_ok_eq!(s.resource_exists_in_group(&key_0, &tag_1), false);
        assert_ok_eq!(s.resource_exists_in_group(&key_1, &tag_2), false);

        // Test releasing the cache via trait method.
        let cache = s.release_naive_group_cache().unwrap();
        assert_eq!(cache.len(), 2);
        assert_some!(cache.get(&key_0));
        assert_some!(cache.get(&key_1));
    }

    #[test]
    fn resource_size_in_group() {
        let state_view = MockStateView::new();
        let s = ResourceGroupAdapter::from_state_view(&state_view, GroupSizeKind::None);

        let key_0 = StateKey::raw(vec![0]);
        let key_1 = StateKey::raw(vec![1]);
        let key_2 = StateKey::raw(vec![2]);
        let tag_0 = mock_tag_0();
        let tag_1 = mock_tag_1();
        let tag_2 = mock_tag_2();

        let key_1_tag_0_len = s
            .get_resource_from_group(&key_1, &tag_0, None)
            .unwrap()
            .unwrap()
            .len() as u64;
        let key_1_tag_1_len = s
            .get_resource_from_group(&key_1, &tag_1, None)
            .unwrap()
            .unwrap()
            .len() as u64;

        assert_ok_eq!(s.resource_size_in_group(&key_1, &tag_0), key_1_tag_0_len);
        assert_ok_eq!(s.resource_size_in_group(&key_1, &tag_1), key_1_tag_1_len);
        assert_ok_eq!(s.resource_size_in_group(&key_2, &tag_2), 0);
        // Release the cache to test contents.
        let cache = s.release_group_cache();
        assert_eq!(cache.len(), 2);
        assert_some!(cache.get(&key_1));
        assert_some!(cache.get(&key_2));

        assert_ok_eq!(s.resource_size_in_group(&key_0, &tag_1), 0);
        assert_ok_eq!(s.resource_size_in_group(&key_1, &tag_2), 0);

        // Test releasing the cache via trait method.
        let cache = s.release_naive_group_cache().unwrap();
        assert_eq!(cache.len(), 2);
        assert_some!(cache.get(&key_0));
        assert_some!(cache.get(&key_1));
    }
}
