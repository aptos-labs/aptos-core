// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::state_restore::{
    StateSnapshotProgress, StateSnapshotRestore, StateValueBatch, StateValueWriter,
};
use anyhow::Result;
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_infallible::RwLock;
use aptos_jellyfish_merkle::mock_tree_store::MockTreeStore;
use aptos_jellyfish_merkle::node_type::{LeafNode, Node, NodeKey};
use aptos_jellyfish_merkle::test_helper::{init_mock_db, ValueBlob};
use aptos_jellyfish_merkle::{
    JellyfishMerkleTree, NodeBatch, TestKey, TestValue, TreeReader, TreeWriter,
};
use aptos_types::state_store::state_storage_usage::StateStorageUsage;
use aptos_types::transaction::Version;
use proptest::{collection::btree_map, prelude::*};
use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};
use storage_interface::StateSnapshotReceiver;

#[derive(Default)]
struct MockSnapshotStore<K: TestKey, V: TestValue> {
    tree_store: MockTreeStore<K>,
    kv_store: RwLock<BTreeMap<(K, Version), V>>,
    usage_store: RwLock<HashMap<Version, StateStorageUsage>>,
    progress_store: RwLock<HashMap<Version, StateSnapshotProgress>>,
}

impl<K, V> MockSnapshotStore<K, V>
where
    K: TestKey,
    V: TestValue,
{
    fn new(overwrite: bool) -> Self {
        Self {
            tree_store: MockTreeStore::new(overwrite),
            kv_store: RwLock::new(BTreeMap::default()),
            usage_store: RwLock::new(HashMap::new()),
            progress_store: RwLock::new(HashMap::new()),
        }
    }

    fn get_value_at_version(&self, k: &(K, Version)) -> Option<V> {
        self.kv_store.read().get(k).cloned()
    }

    fn get_stored_usage(&self, version: Version) -> StateStorageUsage {
        *self
            .usage_store
            .read()
            .get(&version)
            .expect("usage must be set before querying.")
    }

    fn calculate_usage(&self, version: Version) -> StateStorageUsage {
        let mut usage = StateStorageUsage::zero();
        for ((k, ver), v) in self.kv_store.read().iter() {
            if *ver == version {
                usage.add_item(k.key_size() + v.value_size());
            }
        }
        usage
    }
}

impl<K, V> StateValueWriter<K, V> for MockSnapshotStore<K, V>
where
    K: TestKey,
    V: TestValue,
{
    fn write_kv_batch(
        &self,
        version: Version,
        kv_batch: &StateValueBatch<K, Option<V>>,
        progress: StateSnapshotProgress,
    ) -> Result<()> {
        for (k, v) in kv_batch {
            if let Some(v) = v {
                self.kv_store.write().insert(k.clone(), v.clone());
            } else {
                self.kv_store.write().remove(k);
            }
        }
        self.progress_store.write().insert(version, progress);
        Ok(())
    }

    fn write_usage(&self, version: Version, usage: StateStorageUsage) -> Result<()> {
        self.usage_store.write().insert(version, usage);
        Ok(())
    }

    fn get_progress(&self, version: Version) -> Result<Option<StateSnapshotProgress>> {
        Ok(self.progress_store.read().get(&version).cloned())
    }
}

impl<K, V> TreeReader<K> for MockSnapshotStore<K, V>
where
    K: TestKey,
    V: TestValue,
{
    fn get_node_option(&self, node_key: &NodeKey) -> Result<Option<Node<K>>> {
        self.tree_store.get_node_option(node_key)
    }

    fn get_rightmost_leaf(&self, version: Version) -> Result<Option<(NodeKey, LeafNode<K>)>> {
        self.tree_store.get_rightmost_leaf(version)
    }
}

impl<K, V> TreeWriter<K> for MockSnapshotStore<K, V>
where
    K: TestKey,
    V: TestValue,
{
    fn write_node_batch(&self, node_batch: &NodeBatch<K>) -> Result<()> {
        self.tree_store.write_node_batch(node_batch)
    }
}

fn init_mock_store<V>(kvs: &BTreeMap<V, V>) -> (MockSnapshotStore<V, V>, Version)
where
    V: TestKey + TestValue,
{
    let mut kv_store = BTreeMap::new();
    kvs.iter().enumerate().for_each(|(i, (k, v))| {
        kv_store.insert((k.clone(), i as Version), v.clone());
    });

    let (tree_store, version) = init_mock_db(
        &kvs.iter()
            .map(|(k, v)| (CryptoHash::hash(k), (CryptoHash::hash(v), k.clone())))
            .collect(),
    );

    (
        MockSnapshotStore {
            tree_store,
            kv_store: RwLock::new(kv_store),
            usage_store: RwLock::new(HashMap::new()),
            progress_store: RwLock::new(HashMap::new()),
        },
        version,
    )
}

prop_compose! {
    fn arb_btree_map(min_quantity: usize)(tree in btree_map(any::<ValueBlob>(), any::<ValueBlob>(), min_quantity..1000)) -> BTreeMap<HashValue, (ValueBlob, ValueBlob)> {
        tree.into_iter().map(|(k, v)| (CryptoHash::hash(&k), (k, v))).collect()
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]

    #[test]
    fn test_restore_without_interruption(
        btree in arb_btree_map(1),
        target_version in 0u64..2000,
    ) {
        let restore_db = Arc::new(MockSnapshotStore::default());
        // For this test, restore everything without interruption.
        restore_without_interruption(&btree, target_version, &restore_db, true);
    }

    #[test]
    fn test_restore_with_interruption(
        (all, batch1_size, overlap_size) in arb_btree_map(2)
            .prop_flat_map(|btree| {
                let len = btree.len();
                (Just(btree), 1..len)
            })
            .prop_flat_map(|(btree, batch1_size)| {
                // n.b. overlap needs to be at least 1, because the last leaf is not frozen
                (Just(btree), Just(batch1_size), (1..=batch1_size))
            })
    ) {
        let (db, version) = init_mock_store(&all.clone().into_iter().map(|(_, kv)| kv).collect());
        let tree = JellyfishMerkleTree::new(&db);
        let expected_root_hash = tree.get_root_hash(version).unwrap();
        let batch1: Vec<_> = all.clone().into_iter().take(batch1_size).collect();

        let restore_db = Arc::new(MockSnapshotStore::default());
        {
            let mut restore =
                StateSnapshotRestore::new(&restore_db, &restore_db,  version, expected_root_hash, true /* async_commit */).unwrap();
            let proof = tree
                .get_range_proof(batch1.last().map(|(key, _value)| *key).unwrap(), version)
                .unwrap();
            restore.add_chunk(batch1.into_iter().map(|(_, kv)| kv).collect(), proof).unwrap();
            // Do not call `finish`.
        }

        {
            let remaining_accounts: Vec<_> = all.clone().into_iter().skip(batch1_size - overlap_size).collect();

            let mut restore =
                StateSnapshotRestore::new(&restore_db, &restore_db,  version, expected_root_hash, true /* async commit */ ).unwrap();
            let proof = tree
                .get_range_proof(
                    remaining_accounts.last().map(|(h, _)| *h).unwrap(),
                    version,
                )
                .unwrap();
            restore.add_chunk(remaining_accounts.into_iter().
                map(|(_, kv)| kv)
                              .collect()
                              , proof).unwrap();
            restore.finish().unwrap();
        }

        assert_success(&restore_db, expected_root_hash, &all, version);
    }

    #[test]
    fn test_overwrite(
        btree in arb_btree_map(1),
        target_version in 0u64..2000,
    ) {
        let restore_db = Arc::new(MockSnapshotStore::new(true /* allow_overwrite */));
        restore_without_interruption(&btree, target_version, &restore_db, true);
        // overwrite, an entirely different tree
        restore_without_interruption(&btree, target_version, &restore_db, false);
    }
}

fn assert_success<V>(
    db: &MockSnapshotStore<V, V>,
    expected_root_hash: HashValue,
    btree: &BTreeMap<HashValue, (V, V)>,
    version: Version,
) where
    V: TestKey + TestValue,
{
    let tree = JellyfishMerkleTree::new(db);
    for (key, value) in btree.values() {
        let (value_hash, value_index) = tree
            .get_with_proof(CryptoHash::hash(key), version)
            .unwrap()
            .0
            .unwrap();
        let value_in_db = db.get_value_at_version(&value_index).unwrap();
        assert_eq!(CryptoHash::hash(value), value_hash);
        assert_eq!(&value_in_db, value);
    }

    let actual_root_hash = tree.get_root_hash(version).unwrap();
    assert_eq!(actual_root_hash, expected_root_hash);
    let usage_calculated = db.calculate_usage(version);
    let usage_stored = db.get_stored_usage(version);
    assert_eq!(usage_calculated, usage_stored);
    assert_eq!(usage_stored.items(), tree.get_leaf_count(version).unwrap());
}

fn restore_without_interruption<V>(
    btree: &BTreeMap<HashValue, (V, V)>,
    target_version: Version,
    target_db: &Arc<MockSnapshotStore<V, V>>,
    try_resume: bool,
) where
    V: TestKey + TestValue,
{
    let (db, source_version) = init_mock_store(
        &btree
            .iter()
            .map(|(_, (k, v))| (k.clone(), v.clone()))
            .collect(),
    );
    let tree = JellyfishMerkleTree::new(&db);
    let expected_root_hash = tree.get_root_hash(source_version).unwrap();

    let mut restore = if try_resume {
        StateSnapshotRestore::new(
            target_db,
            target_db,
            target_version,
            expected_root_hash,
            true, /* async_commit */
        )
        .unwrap()
    } else {
        StateSnapshotRestore::new_overwrite(
            target_db,
            target_db,
            target_version,
            expected_root_hash,
        )
        .unwrap()
    };
    for (hashed_key, (k, v)) in btree {
        let proof = tree.get_range_proof(*hashed_key, source_version).unwrap();
        restore
            .add_chunk(vec![(k.clone(), v.clone())], proof)
            .unwrap();
    }
    Box::new(restore).finish().unwrap();

    assert_success(target_db, expected_root_hash, btree, target_version);
}
