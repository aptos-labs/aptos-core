// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    mock_tree_store::MockTreeStore,
    node_type::{LeafNode, Node, NodeKey},
    restore::StateSnapshotRestore,
    test_helper::{init_mock_db, ValueBlob},
    JellyfishMerkleTree, NodeBatch, StateValueBatch, StateValueWriter, TestKey, TestValue,
    TreeReader, TreeWriter,
};
use anyhow::Result;
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_infallible::RwLock;
use aptos_types::transaction::Version;
use proptest::{collection::btree_map, prelude::*};
use std::{collections::BTreeMap, sync::Arc};
use storage_interface::StateSnapshotReceiver;

#[derive(Default)]
struct MockSnapshotStore<K: TestKey, V: TestValue> {
    tree_store: MockTreeStore<K>,
    kv_store: RwLock<BTreeMap<(K, Version), V>>,
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
        }
    }

    fn get_value_at_version(&self, k: &(K, Version)) -> Option<V> {
        self.kv_store.read().get(k).cloned()
    }
}

impl<K, V> StateValueWriter<K, V> for MockSnapshotStore<K, V>
where
    K: TestKey,
    V: TestValue,
{
    fn write_kv_batch(&self, kv_batch: &StateValueBatch<K, V>) -> Result<()> {
        for (k, v) in kv_batch {
            self.kv_store.write().insert(k.clone(), v.clone());
        }
        Ok(())
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

    fn get_rightmost_leaf(&self) -> Result<Option<(NodeKey, LeafNode<K>)>> {
        self.tree_store.get_rightmost_leaf()
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
        (all, batch1_size) in arb_btree_map(2)
            .prop_flat_map(|btree| {
                let len = btree.len();
                (Just(btree), 1..len)
            })
    ) {
        let (db, version) = init_mock_store(&all.clone().into_iter().map(|(_, kv)| kv).collect());
        let tree = JellyfishMerkleTree::new(&db);
        let expected_root_hash = tree.get_root_hash(version).unwrap();
        let batch1: Vec<_> = all.clone().into_iter().take(batch1_size).collect();

        let restore_db = Arc::new(MockSnapshotStore::default());
        {
            let mut restore =
                StateSnapshotRestore::new(&restore_db, &restore_db,  version, expected_root_hash ).unwrap();
            let proof = tree
                .get_range_proof(batch1.last().map(|(key, _value)| *key).unwrap(), version)
                .unwrap();
            restore.add_chunk(batch1.into_iter().map(|(_, kv)| kv).collect(), proof).unwrap();
            // Do not call `finish`.
        }

        {
            let rightmost_key = match restore_db.get_rightmost_leaf().unwrap() {
                None => {
                    // Sometimes the batch is too small so nothing is written to DB.
                    return Ok(());
                }
                Some((_, node)) => node.account_key(),
            };
            let remaining_accounts: Vec<_> = all
                .clone()
                .into_iter()
                .filter(|(k, _)| *k > rightmost_key)
                .collect();

            let mut restore =
                StateSnapshotRestore::new(&restore_db, &restore_db,  version, expected_root_hash ).unwrap();
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
        btree1 in arb_btree_map(1),
        btree2 in arb_btree_map(1),
        target_version in 0u64..2000,
    ) {
        let restore_db = Arc::new(MockSnapshotStore::new(true /* allow_overwrite */));
        restore_without_interruption(&btree1, target_version, &restore_db, true);
        // overwrite, an entirely different tree
        restore_without_interruption(&btree2, target_version, &restore_db, false);
    }
}

fn assert_success<V>(
    db: &MockSnapshotStore<V, V>,
    expected_root_hash: HashValue,
    btree: &BTreeMap<HashValue, (V, V)>,
    version: Version,
) where
    V: crate::TestKey + crate::TestValue,
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
}

fn restore_without_interruption<V>(
    btree: &BTreeMap<HashValue, (V, V)>,
    target_version: Version,
    target_db: &Arc<MockSnapshotStore<V, V>>,
    try_resume: bool,
) where
    V: crate::TestKey + crate::TestValue,
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
        StateSnapshotRestore::new(target_db, target_db, target_version, expected_root_hash).unwrap()
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
