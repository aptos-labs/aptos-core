// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    types::{TxnIndex, UnsyncGroupError, ValueWithLayout},
    BlockStateStats,
};
use anyhow::anyhow;
use aptos_aggregator::types::DelayedFieldValue;
use aptos_types::{
    error::{code_invariant_error, PanicError},
    executable::ModulePath,
    vm::modules::AptosModuleExtension,
    write_set::TransactionWrite,
};
use aptos_vm_types::{resolver::ResourceGroupSize, resource_group_adapter::group_size_as_sum};
use move_binary_format::{file_format::CompiledScript, CompiledModule};
use move_core_types::{language_storage::ModuleId, value::MoveTypeLayout};
use move_vm_runtime::{Module, Script};
use move_vm_types::code::{ModuleCache, ModuleCode, UnsyncModuleCache, UnsyncScriptCache};
use serde::Serialize;
use std::{
    cell::RefCell,
    collections::HashMap,
    fmt::Debug,
    hash::Hash,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

/// UnsyncMap is designed to mimic the functionality of MVHashMap for sequential execution.
/// In this case only the latest recorded version is relevant, simplifying the implementation.
pub struct UnsyncMap<
    K: ModulePath,
    T: Hash + Clone + Debug + Eq + Serialize,
    V: TransactionWrite,
    I: Copy,
> {
    // Only use Arc to provide unified interfaces with the MVHashMap.
    resource_map: RefCell<HashMap<K, ValueWithLayout<V>>>,
    group_cache: RefCell<HashMap<K, RefCell<(HashMap<T, ValueWithLayout<V>>, ResourceGroupSize)>>>,
    delayed_field_map: RefCell<HashMap<I, DelayedFieldValue>>,

    // Code caches for modules and scripts.
    module_cache:
        UnsyncModuleCache<ModuleId, CompiledModule, Module, AptosModuleExtension, Option<TxnIndex>>,
    script_cache: UnsyncScriptCache<[u8; 32], CompiledScript, Script>,

    total_base_resource_size: AtomicU64,
    total_base_delayed_field_size: AtomicU64,
}

impl<
        K: ModulePath + Hash + Clone + Eq,
        T: Hash + Clone + Debug + Eq + Serialize,
        V: TransactionWrite,
        I: Hash + Clone + Copy + Eq,
    > Default for UnsyncMap<K, T, V, I>
{
    fn default() -> Self {
        Self {
            resource_map: RefCell::new(HashMap::new()),
            module_cache: UnsyncModuleCache::empty(),
            script_cache: UnsyncScriptCache::empty(),
            group_cache: RefCell::new(HashMap::new()),
            delayed_field_map: RefCell::new(HashMap::new()),
            total_base_resource_size: AtomicU64::new(0),
            total_base_delayed_field_size: AtomicU64::new(0),
        }
    }
}

impl<
        K: ModulePath + Hash + Clone + Eq + Debug,
        T: Hash + Clone + Debug + Eq + Serialize,
        V: TransactionWrite,
        I: Hash + Clone + Copy + Eq,
    > UnsyncMap<K, T, V, I>
{
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the module cache for this [UnsyncMap].
    pub fn module_cache(
        &self,
    ) -> &UnsyncModuleCache<ModuleId, CompiledModule, Module, AptosModuleExtension, Option<TxnIndex>>
    {
        &self.module_cache
    }

    /// Returns the script cache for this [UnsyncMap].
    pub fn script_cache(&self) -> &UnsyncScriptCache<[u8; 32], CompiledScript, Script> {
        &self.script_cache
    }

    /// Returns all modules stored inside [UnsyncMap].
    pub fn into_modules_iter(
        self,
    ) -> impl Iterator<
        Item = (
            ModuleId,
            Arc<ModuleCode<CompiledModule, Module, AptosModuleExtension>>,
        ),
    > {
        self.module_cache.into_modules_iter()
    }

    pub fn stats(&self) -> BlockStateStats {
        BlockStateStats {
            num_resources: self.resource_map.borrow().len(),
            num_resource_groups: self.group_cache.borrow().len(),
            num_delayed_fields: self.delayed_field_map.borrow().len(),
            num_modules: self.module_cache.num_modules(),
            base_resources_size: self.total_base_resource_size.load(Ordering::Relaxed),
            base_delayed_fields_size: self.total_base_delayed_field_size.load(Ordering::Relaxed),
        }
    }

    pub fn set_group_base_values(
        &self,
        group_key: K,
        base_values: impl IntoIterator<Item = (T, V)>,
    ) -> anyhow::Result<()> {
        let base_map: HashMap<T, ValueWithLayout<V>> = base_values
            .into_iter()
            .map(|(t, v)| (t, ValueWithLayout::RawFromStorage(Arc::new(v))))
            .collect();
        let base_size = group_size_as_sum(
            base_map
                .iter()
                .flat_map(|(t, v)| v.bytes_len().map(|s| (t, s))),
        )
        .map_err(|e| {
            anyhow!(
                "Tag serialization error in resource group at {:?}: {:?}",
                group_key.clone(),
                e
            )
        })?;
        assert!(
            self.group_cache
                .borrow_mut()
                .insert(group_key, RefCell::new((base_map, base_size)))
                .is_none(),
            "UnsyncMap group cache must be empty to provide base values"
        );
        Ok(())
    }

    pub fn update_tagged_base_value_with_layout(
        &self,
        group_key: K,
        tag: T,
        value: V,
        layout: Option<Arc<MoveTypeLayout>>,
    ) {
        self.group_cache
            .borrow_mut()
            .get_mut(&group_key)
            .expect("Unable to fetch the entry for the group key in group_cache")
            .borrow_mut()
            .0
            .insert(tag, ValueWithLayout::Exchanged(Arc::new(value), layout));
    }

    pub fn get_group_size(&self, group_key: &K) -> Option<ResourceGroupSize> {
        self.group_cache
            .borrow()
            .get(group_key)
            .map(|entry| entry.borrow().1)
    }

    pub fn fetch_group_tagged_data(
        &self,
        group_key: &K,
        value_tag: &T,
    ) -> Result<ValueWithLayout<V>, UnsyncGroupError> {
        self.group_cache.borrow().get(group_key).map_or(
            Err(UnsyncGroupError::Uninitialized),
            |group_map| {
                group_map
                    .borrow()
                    .0
                    .get(value_tag)
                    .cloned()
                    .ok_or(UnsyncGroupError::TagNotFound)
            },
        )
    }

    /// Contains the latest group ops for the given group key.
    pub fn finalize_group(
        &self,
        group_key: &K,
    ) -> (
        impl Iterator<Item = (T, ValueWithLayout<V>)>,
        ResourceGroupSize,
    ) {
        let binding = self.group_cache.borrow();
        let group = binding
            .get(group_key)
            .expect("Resource group must be cached")
            .borrow();

        (group.0.clone().into_iter(), group.1)
    }

    pub fn insert_group_ops(
        &self,
        group_key: &K,
        group_ops: impl IntoIterator<Item = (T, (V, Option<Arc<MoveTypeLayout>>))>,
        group_size: ResourceGroupSize,
    ) -> Result<(), PanicError> {
        for (value_tag, (group_op, maybe_layout)) in group_ops.into_iter() {
            self.insert_group_op(group_key, value_tag, group_op, maybe_layout)?;
        }
        self.group_cache
            .borrow_mut()
            .get_mut(group_key)
            .expect("Resource group must be cached")
            .borrow_mut()
            .1 = group_size;
        Ok(())
    }

    fn insert_group_op(
        &self,
        group_key: &K,
        value_tag: T,
        v: V,
        maybe_layout: Option<Arc<MoveTypeLayout>>,
    ) -> Result<(), PanicError> {
        use aptos_types::write_set::WriteOpKind::*;
        use std::collections::hash_map::Entry::*;
        match (
            self.group_cache
                .borrow_mut()
                .get_mut(group_key)
                .expect("Resource group must be cached")
                .borrow_mut()
                .0
                .entry(value_tag.clone()),
            v.write_op_kind(),
        ) {
            (Occupied(entry), Deletion) => {
                entry.remove();
            },
            (Occupied(mut entry), Modification) => {
                entry.insert(ValueWithLayout::Exchanged(Arc::new(v), maybe_layout));
            },
            (Vacant(entry), Creation) => {
                entry.insert(ValueWithLayout::Exchanged(Arc::new(v), maybe_layout));
            },
            (l, r) => {
                return Err(code_invariant_error(format!(
                    "WriteOp kind {:?} not consistent with previous value at tag {:?}. Existing: {:?}, new: {:?}",
                    v.write_op_kind(),
                    value_tag,
		    l,
		    r,
                )));
            },
        }

        Ok(())
    }

    pub fn fetch_data(&self, key: &K) -> Option<ValueWithLayout<V>> {
        self.resource_map.borrow().get(key).cloned()
    }

    pub fn fetch_exchanged_data(
        &self,
        key: &K,
    ) -> Result<(Arc<V>, Arc<MoveTypeLayout>), PanicError> {
        let data = self.fetch_data(key);
        if let Some(ValueWithLayout::Exchanged(value, Some(layout))) = data {
            Ok((value, layout))
        } else {
            Err(code_invariant_error(format!(
                "Read value needing exchange {:?} does not exist or not in Exchanged format",
                data
            )))
        }
    }

    pub fn fetch_group_data(&self, key: &K) -> Option<Vec<(Arc<T>, ValueWithLayout<V>)>> {
        self.group_cache.borrow().get(key).map(|group_map| {
            group_map
                .borrow()
                .0
                .iter()
                .map(|(tag, value)| (Arc::new(tag.clone()), value.clone()))
                .collect()
        })
    }

    pub fn fetch_delayed_field(&self, id: &I) -> Option<DelayedFieldValue> {
        self.delayed_field_map.borrow().get(id).cloned()
    }

    pub fn write(&self, key: K, value: Arc<V>, layout: Option<Arc<MoveTypeLayout>>) {
        self.resource_map
            .borrow_mut()
            .insert(key, ValueWithLayout::Exchanged(value, layout));
    }

    pub fn set_base_value(&self, key: K, value: ValueWithLayout<V>) {
        let cur_size = value.bytes_len();
        if self.resource_map.borrow_mut().insert(key, value).is_none() {
            if let Some(cur_size) = cur_size {
                self.total_base_resource_size
                    .fetch_add(cur_size as u64, Ordering::Relaxed);
            }
        }
    }

    pub fn write_delayed_field(&self, id: I, value: DelayedFieldValue) {
        self.delayed_field_map.borrow_mut().insert(id, value);
    }

    pub fn set_base_delayed_field(&self, id: I, value: DelayedFieldValue) {
        self.total_base_delayed_field_size.fetch_add(
            value.get_approximate_memory_size() as u64,
            Ordering::Relaxed,
        );
        self.delayed_field_map.borrow_mut().insert(id, value);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::types::test::{KeyType, TestValue};
    use claims::{assert_err, assert_err_eq, assert_none, assert_ok, assert_ok_eq, assert_some_eq};

    fn finalize_group_as_hashmap(
        map: &UnsyncMap<KeyType<Vec<u8>>, usize, TestValue, ()>,
        key: &KeyType<Vec<u8>>,
    ) -> HashMap<usize, ValueWithLayout<TestValue>> {
        map.finalize_group(key).0.collect()
    }

    // TODO[agg_v2](test) Add tests with non trivial layout
    #[test]
    fn group_commit_idx() {
        let ap = KeyType(b"/foo/f".to_vec());
        let map = UnsyncMap::<KeyType<Vec<u8>>, usize, TestValue, ()>::new();

        map.set_group_base_values(
            ap.clone(),
            // base tag 1, 2, 3
            (1..4).map(|i| (i, TestValue::with_kind(i, true))),
        )
        .unwrap();
        assert_ok!(map.insert_group_op(&ap, 2, TestValue::with_kind(202, false), None));
        assert_ok!(map.insert_group_op(&ap, 3, TestValue::with_kind(203, false), None));
        let committed = finalize_group_as_hashmap(&map, &ap);

        // // The value at tag 1 is from base, while 2 and 3 are from txn 3.
        // // (Arc compares with value equality)
        assert_eq!(committed.len(), 3);
        assert_some_eq!(
            committed.get(&1),
            &ValueWithLayout::RawFromStorage(Arc::new(TestValue::with_kind(1, true)))
        );
        assert_some_eq!(
            committed.get(&2),
            &ValueWithLayout::Exchanged(Arc::new(TestValue::with_kind(202, false)), None)
        );
        assert_some_eq!(
            committed.get(&3),
            &ValueWithLayout::Exchanged(Arc::new(TestValue::with_kind(203, false)), None)
        );

        assert_ok!(map.insert_group_op(&ap, 3, TestValue::with_kind(303, false), None));
        assert_ok!(map.insert_group_op(&ap, 4, TestValue::with_kind(304, true), None));
        let committed = finalize_group_as_hashmap(&map, &ap);
        assert_eq!(committed.len(), 4);
        assert_some_eq!(
            committed.get(&1),
            &ValueWithLayout::RawFromStorage(Arc::new(TestValue::with_kind(1, true)))
        );
        assert_some_eq!(
            committed.get(&2),
            &ValueWithLayout::Exchanged(Arc::new(TestValue::with_kind(202, false)), None)
        );
        assert_some_eq!(
            committed.get(&3),
            &ValueWithLayout::Exchanged(Arc::new(TestValue::with_kind(303, false)), None)
        );
        assert_some_eq!(
            committed.get(&4),
            &ValueWithLayout::Exchanged(Arc::new(TestValue::with_kind(304, true)), None)
        );

        assert_ok!(map.insert_group_op(&ap, 0, TestValue::with_kind(100, true), None));
        assert_ok!(map.insert_group_op(&ap, 1, TestValue::deletion(), None));
        assert_err!(map.insert_group_op(&ap, 1, TestValue::deletion(), None));
        let committed = finalize_group_as_hashmap(&map, &ap);
        assert_eq!(committed.len(), 4);
        assert_some_eq!(
            committed.get(&0),
            &ValueWithLayout::Exchanged(Arc::new(TestValue::with_kind(100, true)), None)
        );
        assert_none!(committed.get(&1));
        assert_some_eq!(
            committed.get(&2),
            &ValueWithLayout::Exchanged(Arc::new(TestValue::with_kind(202, false)), None)
        );
        assert_some_eq!(
            committed.get(&3),
            &ValueWithLayout::Exchanged(Arc::new(TestValue::with_kind(303, false)), None)
        );
        assert_some_eq!(
            committed.get(&4),
            &ValueWithLayout::Exchanged(Arc::new(TestValue::with_kind(304, true)), None)
        );

        assert_ok!(map.insert_group_op(&ap, 0, TestValue::deletion(), None));
        assert_ok!(map.insert_group_op(&ap, 1, TestValue::with_kind(400, true), None));
        assert_ok!(map.insert_group_op(&ap, 2, TestValue::deletion(), None));
        assert_ok!(map.insert_group_op(&ap, 3, TestValue::deletion(), None));
        assert_ok!(map.insert_group_op(&ap, 4, TestValue::deletion(), None));
        let committed = finalize_group_as_hashmap(&map, &ap);
        assert_eq!(committed.len(), 1);
        assert_some_eq!(
            committed.get(&1),
            &ValueWithLayout::Exchanged(Arc::new(TestValue::with_kind(400, true)), None)
        );
    }

    #[should_panic]
    #[test]
    fn set_base_twice() {
        let ap = KeyType(b"/foo/f".to_vec());
        let map = UnsyncMap::<KeyType<Vec<u8>>, usize, TestValue, ()>::new();

        assert_ok!(map.set_group_base_values(
            ap.clone(),
            (1..4).map(|i| (i, TestValue::with_kind(i, true))),
        ));
        assert_ok!(map.set_group_base_values(
            ap.clone(),
            (1..4).map(|i| (i, TestValue::with_kind(i, true))),
        ));
    }

    #[should_panic]
    #[test]
    fn group_op_without_base() {
        let ap = KeyType(b"/foo/f".to_vec());
        let map = UnsyncMap::<KeyType<Vec<u8>>, usize, TestValue, ()>::new();

        assert_ok!(map.insert_group_op(&ap, 3, TestValue::with_kind(10, true), None));
    }

    #[should_panic]
    #[test]
    fn group_no_path_exists() {
        let ap = KeyType(b"/foo/b".to_vec());
        let map = UnsyncMap::<KeyType<Vec<u8>>, usize, TestValue, ()>::new();

        let _ = map.finalize_group(&ap).0.collect::<Vec<_>>();
    }

    #[test]
    fn group_size() {
        let ap = KeyType(b"/foo/f".to_vec());
        let map = UnsyncMap::<KeyType<Vec<u8>>, usize, TestValue, ()>::new();

        assert_none!(map.get_group_size(&ap));

        map.set_group_base_values(
            ap.clone(),
            // base tag 1, 2, 3, 4
            (1..5).map(|i| (i, TestValue::creation_with_len(1))),
        )
        .unwrap();

        let tag: usize = 5;
        let one_entry_len = TestValue::creation_with_len(1).bytes().unwrap().len();
        let two_entry_len = TestValue::creation_with_len(2).bytes().unwrap().len();
        let three_entry_len = TestValue::creation_with_len(3).bytes().unwrap().len();
        let four_entry_len = TestValue::creation_with_len(4).bytes().unwrap().len();

        let base_size = group_size_as_sum(vec![(&tag, one_entry_len); 4].into_iter()).unwrap();
        assert_some_eq!(map.get_group_size(&ap), base_size);

        let exp_size = group_size_as_sum(vec![(&tag, two_entry_len); 2].into_iter().chain(vec![
            (
                &tag,
                one_entry_len
            );
            3
        ]))
        .unwrap();
        assert_err!(map.insert_group_ops(
            &ap,
            vec![(0, (TestValue::modification_with_len(2), None))],
            exp_size,
        ));
        assert_err!(map.insert_group_ops(
            &ap,
            vec![(1, (TestValue::creation_with_len(2), None))],
            exp_size,
        ));
        assert_ok!(map.insert_group_ops(
            &ap,
            vec![
                (0, (TestValue::creation_with_len(2), None)),
                (1, (TestValue::modification_with_len(2), None))
            ],
            exp_size
        ));
        assert_some_eq!(map.get_group_size(&ap), exp_size);

        let exp_size = group_size_as_sum(
            vec![(&tag, one_entry_len); 2]
                .into_iter()
                .chain(vec![(&tag, two_entry_len); 2])
                .chain(vec![(&tag, three_entry_len); 2]),
        )
        .unwrap();
        assert_ok!(map.insert_group_ops(
            &ap,
            vec![
                (4, (TestValue::modification_with_len(3), None)),
                (5, (TestValue::creation_with_len(3), None)),
            ],
            exp_size
        ));
        assert_some_eq!(map.get_group_size(&ap), exp_size);

        let exp_size = group_size_as_sum(
            vec![(&tag, one_entry_len); 2]
                .into_iter()
                .chain(vec![(&tag, three_entry_len); 2])
                .chain(vec![(&tag, four_entry_len); 2]),
        )
        .unwrap();
        assert_ok!(map.insert_group_ops(
            &ap,
            vec![
                (0, (TestValue::modification_with_len(4), None)),
                (1, (TestValue::modification_with_len(4), None))
            ],
            exp_size
        ));
        assert_some_eq!(map.get_group_size(&ap), exp_size);
    }

    #[test]
    fn group_value() {
        let ap = KeyType(b"/foo/f".to_vec());
        let map = UnsyncMap::<KeyType<Vec<u8>>, usize, TestValue, ()>::new();

        // Uninitialized before group is set, TagNotFound afterwards
        assert_err_eq!(
            map.fetch_group_tagged_data(&ap, &1),
            UnsyncGroupError::Uninitialized
        );

        map.set_group_base_values(
            ap.clone(),
            // base tag 1, 2, 3, 4
            (1..5).map(|i| (i, TestValue::creation_with_len(i))),
        )
        .unwrap();

        for i in 1..5 {
            assert_ok_eq!(
                map.fetch_group_tagged_data(&ap, &i),
                ValueWithLayout::RawFromStorage(Arc::new(TestValue::creation_with_len(i)),)
            );
        }
        assert_err_eq!(
            map.fetch_group_tagged_data(&ap, &0),
            UnsyncGroupError::TagNotFound
        );
        assert_err_eq!(
            map.fetch_group_tagged_data(&ap, &6),
            UnsyncGroupError::TagNotFound
        );

        assert_ok!(map.insert_group_op(&ap, 1, TestValue::deletion(), None));
        assert_ok!(map.insert_group_op(&ap, 3, TestValue::modification_with_len(8), None));
        assert_ok!(map.insert_group_op(&ap, 6, TestValue::creation_with_len(9), None));

        assert_err_eq!(
            map.fetch_group_tagged_data(&ap, &1),
            UnsyncGroupError::TagNotFound,
        );
        assert_ok_eq!(
            map.fetch_group_tagged_data(&ap, &3),
            ValueWithLayout::Exchanged(Arc::new(TestValue::modification_with_len(8)), None,)
        );
        assert_ok_eq!(
            map.fetch_group_tagged_data(&ap, &6),
            ValueWithLayout::Exchanged(Arc::new(TestValue::creation_with_len(9)), None,)
        );

        // others unaffected.
        assert_err_eq!(
            map.fetch_group_tagged_data(&ap, &0),
            UnsyncGroupError::TagNotFound,
        );
        assert_ok_eq!(
            map.fetch_group_tagged_data(&ap, &2),
            ValueWithLayout::RawFromStorage(Arc::new(TestValue::creation_with_len(2)),)
        );
        assert_ok_eq!(
            map.fetch_group_tagged_data(&ap, &4),
            ValueWithLayout::RawFromStorage(Arc::new(TestValue::creation_with_len(4)),)
        );
    }
}
