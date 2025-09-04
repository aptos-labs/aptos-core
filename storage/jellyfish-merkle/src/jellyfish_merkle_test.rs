// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::*;
use crate::{
    node_type::NodeType,
    test_helper::{
        ValueBlob, arb_existent_kvs_and_nonexistent_keys, arb_kv_pair_with_distinct_last_nibble,
        arb_tree_with_index, gen_value, test_get_leaf_count, test_get_range_proof,
        test_get_with_proof, test_get_with_proof_with_distinct_last_nibble,
    },
};
use aptos_crypto::{HashValue, hash::SPARSE_MERKLE_PLACEHOLDER_HASH};
use aptos_types::nibble::Nibble;
use mock_tree_store::MockTreeStore;
use proptest::{collection::hash_set, prelude::*};
use rand::{Rng, SeedableRng, rngs::StdRng};

fn update_nibble(original_key: &HashValue, n: usize, nibble: u8) -> HashValue {
    assert!(nibble < 16);
    let mut key = original_key.to_vec();
    key[n / 2] = if n % 2 == 0 {
        key[n / 2] & 0x0F | (nibble << 4)
    } else {
        key[n / 2] & 0xF0 | nibble
    };
    HashValue::from_slice(&key).unwrap()
}

fn gen_leaf(k: HashValue, v: &(HashValue, ValueBlob), version: Version) -> Node<ValueBlob> {
    LeafNode::new(k, v.0, (v.1.clone(), version)).into()
}

#[test]
fn test_insert_to_empty_tree() {
    let db = MockTreeStore::default();
    let tree = JellyfishMerkleTree::new(&db);

    // Tree is initially empty. Root is a null node. We'll insert a key-value pair which creates a
    // leaf node.
    let key = HashValue::random();
    let state_key = ValueBlob::from(vec![1u8, 2u8, 3u8, 4u8]);
    let value_hash = HashValue::random();

    // batch version
    let (_new_root_hash, batch) = tree
        .put_value_set_test(
            vec![(key, Some(&(value_hash, state_key)))],
            0, /* version */
        )
        .unwrap();
    assert!(
        batch
            .stale_node_index_batch
            .iter()
            .flatten()
            .next()
            .is_none()
    );

    db.write_tree_update_batch(batch).unwrap();
    assert_eq!(tree.get(key, 0).unwrap().unwrap(), value_hash);

    let (empty_root_hash, batch) = tree
        .put_value_set_test(vec![(key, None)], 1 /* version */)
        .unwrap();
    db.write_tree_update_batch(batch).unwrap();
    assert_eq!(tree.get(key, 1).unwrap(), None);
    assert_eq!(empty_root_hash, *SPARSE_MERKLE_PLACEHOLDER_HASH);
}

#[test]
fn test_insert_at_leaf_with_internal_created() {
    let db = MockTreeStore::default();
    let tree = JellyfishMerkleTree::new(&db);

    let key1 = HashValue::new([0x00u8; HashValue::LENGTH]);
    let value1 = gen_value();

    let (root0_hash, batch) = tree
        .put_value_set_test(vec![(key1, Some(&value1))], 0 /* version */)
        .unwrap();

    assert!(
        batch
            .stale_node_index_batch
            .iter()
            .flatten()
            .next()
            .is_none()
    );
    db.write_tree_update_batch(batch).unwrap();
    assert_eq!(tree.get(key1, 0).unwrap().unwrap(), value1.0);

    // Insert at the previous leaf node. Should generate an internal node at the root.
    // Change the 2st nibble to 15.
    let key2 = update_nibble(&key1, 1, 15);
    let value2 = gen_value();

    let (_root1_hash, batch) = tree
        .put_value_set_test(vec![(key2, Some(&value2))], 1 /* version */)
        .unwrap();
    assert_eq!(batch.num_stale_node(), 2);
    db.write_tree_update_batch(batch).unwrap();

    assert_eq!(tree.get(key1, 0).unwrap().unwrap(), value1.0);
    assert!(tree.get(key2, 0).unwrap().is_none());
    assert_eq!(tree.get(key2, 1).unwrap().unwrap(), value2.0);

    // get # of nodes
    assert_eq!(db.num_nodes(), 6 /* 2 + 4 */);

    let nibble_path = NibblePath::new_odd(vec![key2.nibble(0) << 4]);
    let internal_node_key = NodeKey::new(1, nibble_path.clone());

    let leaf1 = gen_leaf(key1, &value1, 0);
    let leaf2 = gen_leaf(key2, &value2, 1);
    let mut children = BTreeMap::new();
    children.insert(
        Nibble::from(0),
        Child::new(leaf1.hash(), 1 /* version */, NodeType::Leaf),
    );
    children.insert(
        Nibble::from(15),
        Child::new(leaf2.hash(), 1 /* version */, NodeType::Leaf),
    );
    let internal = Node::new_internal(Children::from_sorted(children));
    assert_eq!(db.get_node(&NodeKey::new(0, nibble_path)).unwrap(), leaf1);
    assert_eq!(
        db.get_node(&internal_node_key.gen_child_node_key(1 /* version */, Nibble::from(0)))
            .unwrap(),
        leaf1
    );
    assert_eq!(
        db.get_node(&internal_node_key.gen_child_node_key(1 /* version */, Nibble::from(15)))
            .unwrap(),
        leaf2
    );
    assert_eq!(db.get_node(&internal_node_key).unwrap(), internal);

    // Deletion
    let (root2_hash, batch) = tree
        .put_value_set_test(vec![(key2, None)], 2 /* version */)
        .unwrap();
    assert_eq!(batch.num_stale_node(), 4);
    db.write_tree_update_batch(batch).unwrap();

    assert_eq!(tree.get(key1, 0).unwrap().unwrap(), value1.0);
    assert!(tree.get(key2, 0).unwrap().is_none());
    assert_eq!(tree.get(key2, 1).unwrap().unwrap(), value2.0);
    assert!(tree.get(key2, 2).unwrap().is_none());
    assert_eq!(root0_hash, root2_hash);
    // get # of nodes
    assert_eq!(db.num_nodes(), 8 /* 2 + 4 + 2 */);
}

#[test]
fn test_insert_at_leaf_with_multiple_internals_created() {
    let db = MockTreeStore::default();
    let tree = JellyfishMerkleTree::new(&db);

    // 1. Insert the first leaf into empty tree
    let key1 = HashValue::new([0x00u8; HashValue::LENGTH]);
    let value1 = gen_value();

    let (_root0_hash, batch) = tree
        .put_value_set_test(vec![(key1, Some(&value1))], 0 /* version */)
        .unwrap();
    db.write_tree_update_batch(batch).unwrap();
    assert_eq!(tree.get(key1, 0).unwrap().unwrap(), value1.0);

    // 2. Insert at the previous leaf node. Should generate a branch node.
    // Change the 2nd nibble to 1.
    let key2 = update_nibble(&key1, 1 /* nibble_index */, 1 /* nibble */);
    let value2 = gen_value();

    let (_root1_hash, batch) = tree
        .put_value_set_test(vec![(key2, Some(&value2))], 1 /* version */)
        .unwrap();
    db.write_tree_update_batch(batch).unwrap();
    assert_eq!(tree.get(key1, 0).unwrap().unwrap(), value1.0);
    assert!(tree.get(key2, 0).unwrap().is_none());
    assert_eq!(tree.get(key2, 1).unwrap().unwrap(), value2.0);

    assert_eq!(db.num_nodes(), 6);

    let nibble_path = NibblePath::new_odd(vec![key2.nibble(0) << 4]);
    let internal_node_key = NodeKey::new(1, nibble_path.clone());

    let leaf1 = gen_leaf(key1, &value1, 0);
    let leaf2 = gen_leaf(key2, &value2, 1);
    let internal = {
        let mut children = BTreeMap::new();
        children.insert(
            Nibble::from(0),
            Child::new(leaf1.hash(), 1 /* version */, NodeType::Leaf),
        );
        children.insert(
            Nibble::from(1),
            Child::new(leaf2.hash(), 1 /* version */, NodeType::Leaf),
        );
        Node::new_internal(Children::from_sorted(children))
    };

    let root_internal = {
        let mut children = BTreeMap::new();
        children.insert(
            Nibble::from(0),
            Child::new(
                internal.hash(),
                1, /* version */
                NodeType::Internal { leaf_count: 2 },
            ),
        );
        Node::new_internal(Children::from_sorted(children))
    };

    assert_eq!(db.get_node(&NodeKey::new(0, nibble_path)).unwrap(), leaf1);
    assert_eq!(
        db.get_node(&internal_node_key.gen_child_node_key(1 /* version */, Nibble::from(0)))
            .unwrap(),
        leaf1,
    );
    assert_eq!(
        db.get_node(&internal_node_key.gen_child_node_key(1 /* version */, Nibble::from(1)))
            .unwrap(),
        leaf2,
    );
    assert_eq!(db.get_node(&internal_node_key).unwrap(), internal);
    assert_eq!(
        db.get_node(&NodeKey::new_empty_path(1)).unwrap(),
        root_internal,
    );

    // 3. Update leaf2 with new value
    let value2_update = gen_value();
    let (_root2_hash, batch) = tree
        .put_value_set_test(vec![(key2, Some(&value2_update))], 2 /* version */)
        .unwrap();
    db.write_tree_update_batch(batch).unwrap();
    assert!(tree.get(key2, 0).unwrap().is_none());
    assert_eq!(tree.get(key2, 1).unwrap().unwrap(), value2.0);
    assert_eq!(tree.get(key2, 2).unwrap().unwrap(), value2_update.0);

    // Get # of nodes.
    assert_eq!(db.num_nodes(), 9 /* 2 + 4 + 3 */);

    // Purge retired nodes.
    db.purge_stale_nodes(1).unwrap();
    assert_eq!(db.num_nodes(), 7);
    db.purge_stale_nodes(2).unwrap();
    assert_eq!(db.num_nodes(), 4);
    assert_eq!(tree.get(key1, 2).unwrap().unwrap(), value1.0);
    assert_eq!(tree.get(key2, 2).unwrap().unwrap(), value2_update.0);

    // 4. Delete leaf2
    let (_root2_hash, batch) = tree
        .put_value_set_test(vec![(key2, None)], 3 /* version */)
        .unwrap();
    db.write_tree_update_batch(batch).unwrap();
    // Get # of nodes.
    assert_eq!(db.num_nodes(), 6 /* 4 + 2 */);
    db.purge_stale_nodes(3).unwrap();
    assert_eq!(db.num_nodes(), 2);
    assert_eq!(tree.get(key1, 3).unwrap().unwrap(), value1.0);
}

#[test]
fn test_batch_insertion() {
    // ```text
    //                             internal(root)
    //                            /        \
    //                       internal       2        <- nibble 0
    //                      /   |   \
    //              internal    3    4               <- nibble 1
    //                 |
    //              internal                         <- nibble 2
    //              /      \
    //        internal      6                        <- nibble 3
    //           |
    //        internal                               <- nibble 4
    //        /      \
    //       1        5                              <- nibble 5
    //
    // Total: 12 nodes
    // ```
    let key1 = HashValue::new([0x00u8; HashValue::LENGTH]);
    let value1 = gen_value();

    let key2 = update_nibble(&key1, 0, 2);
    let value2 = gen_value();
    let value2_update = gen_value();

    let key3 = update_nibble(&key1, 1, 3);
    let value3 = gen_value();

    let key4 = update_nibble(&key1, 1, 4);
    let value4 = gen_value();

    let key5 = update_nibble(&key1, 5, 5);
    let value5 = gen_value();

    let key6 = update_nibble(&key1, 3, 6);
    let value6 = gen_value();

    let batches = vec![
        vec![(key1, Some(&value1))],
        vec![(key2, Some(&value2))],
        vec![(key3, Some(&value3))],
        vec![(key4, Some(&value4))],
        vec![(key5, Some(&value5))],
        vec![(key6, Some(&value6))],
        vec![(key2, Some(&value2_update))],
    ];
    let one_batch = batches.iter().flatten().cloned().collect::<Vec<_>>();

    let mut to_verify = one_batch.clone();
    // key2 was updated so we remove it.
    to_verify.remove(1);
    let verify_fn = |tree: &JellyfishMerkleTree<MockTreeStore<ValueBlob>, ValueBlob>,
                     version: Version| {
        to_verify
            .iter()
            .for_each(|(k, v)| assert_eq!(tree.get(*k, version).unwrap().unwrap(), v.unwrap().0))
    };

    // Insert as one batch and update one by one.
    {
        let db = MockTreeStore::default();
        let tree = JellyfishMerkleTree::new(&db);

        let (_root, batch) = tree.put_value_set_test(one_batch, 0 /* version */).unwrap();
        db.write_tree_update_batch(batch).unwrap();
        verify_fn(&tree, 0);

        // get # of nodes
        assert_eq!(db.num_nodes(), 12);
    }

    // Insert in multiple batches.
    {
        let db = MockTreeStore::default();
        let tree = JellyfishMerkleTree::new(&db);

        for (idx, kvs) in batches.into_iter().enumerate() {
            let (_roots, batch) = tree.put_value_set_test(kvs, idx as Version).unwrap();
            db.write_tree_update_batch(batch).unwrap();
        }
        verify_fn(&tree, 6);

        // get # of nodes
        assert_eq!(db.num_nodes(), 32 /* 2 + 3 + 5 + 4 + 8 + 5 + 3 */);

        // Purge retired nodes('p' means purged and 'a' means added).
        // The initial state of the tree at version 0
        // ```test
        //     internal
        //    /
        //   1
        // ```
        db.purge_stale_nodes(1).unwrap();
        // ```text
        //     internal(p)         internal(a)
        //    /             ->    /        \
        //   1(p)                1(a)       2(a)
        //
        // add 3, prune 2
        // ```
        assert_eq!(db.num_nodes(), 30);
        db.purge_stale_nodes(2).unwrap();
        // ```text
        //     internal(p)             internal(a)
        //    /        \              /        \
        //   1(p)       2(p) ->  internal(a)    2(a)
        //                       /       \
        //                      1(a)      3(a)
        // add 5, prune 3
        // ```
        assert_eq!(db.num_nodes(), 27);
        db.purge_stale_nodes(3).unwrap();
        // ```text
        //         internal(p)                internal(a)
        //        /        \                 /        \
        //   internal(p)    2(p)   ->  internal(a)     2(a)
        //   /       \                /   |   \
        //  1         3              1    3    4(a)
        // add 4, prune 3
        // ```
        assert_eq!(db.num_nodes(), 24);
        db.purge_stale_nodes(4).unwrap();
        // ```text
        //            internal(p)                         internal(a)
        //           /        \                          /        \
        //     internal(p)     2(p)                 internal(a)    2(a)
        //    /   |   \                            /   |   \
        //   1(p) 3    4           ->      internal(a) 3    4
        //                                     |
        //                                 internal(a)
        //                                     |
        //                                 internal(a)
        //                                     |
        //                                 internal(a)
        //                                 /      \
        //                                1(a)     5(a)
        // add 9, prune 4
        // ```
        assert_eq!(db.num_nodes(), 20);
        db.purge_stale_nodes(5).unwrap();
        // ```text
        //                  internal(p)                             internal(a)
        //                 /        \                              /        \
        //            internal(p)    2(p)                     internal(a)    2(a)
        //           /   |   \                               /   |   \
        //   internal(p) 3    4                      internal(a) 3    4
        //       |                                      |
        //   internal(p)                 ->          internal(a)
        //       |                                   /      \
        //   internal                          internal      6(a)
        //       |                                |
        //   internal                          internal
        //   /      \                          /      \
        //  1        5                        1        5
        // add 6, prune 5
        // ```
        assert_eq!(db.num_nodes(), 15);
        db.purge_stale_nodes(6).unwrap();
        // ```text
        //                         internal(p)                               internal(a)
        //                        /        \                                /        \
        //                   internal(p)    2(p)                       internal(a)    2(a)
        //                  /   |   \                                 /   |   \
        //          internal    3    4                        internal    3    4
        //             |                                         |
        //          internal                      ->          internal
        //          /      \                                  /      \
        //    internal      6                           internal      6
        //       |                                         |
        //    internal                                  internal
        //    /      \                                  /      \
        //   1        5                                1        5
        // add 3, prune 3
        // ```
        assert_eq!(db.num_nodes(), 12);
        verify_fn(&tree, 6);
    }
}

#[test]
fn test_deletion() {
    // ```text
    //                             internal(root)
    //                            /        \
    //                       internal       2        <- nibble 0
    //                      /   |   \
    //              internal    3    4               <- nibble 1
    //                 |
    //              internal                         <- nibble 2
    //              /      \
    //        internal      6                        <- nibble 3
    //           |
    //        internal                               <- nibble 4
    //        /      \
    //       1        5                              <- nibble 5
    //
    // Total: 12 nodes
    // ```
    let key1 = HashValue::new([0x00u8; HashValue::LENGTH]);
    let value1 = gen_value();

    let key2 = update_nibble(&key1, 0, 2);
    let value2 = gen_value();

    let key3 = update_nibble(&key1, 1, 3);
    let value3 = gen_value();

    let key4 = update_nibble(&key1, 1, 4);
    let value4 = gen_value();

    let key5 = update_nibble(&key1, 5, 5);
    let value5 = gen_value();

    let key6 = update_nibble(&key1, 3, 6);
    let value6 = gen_value();

    let batches = vec![
        vec![(key1, Some(&value1))],
        vec![(key2, Some(&value2))],
        vec![(key3, Some(&value3))],
        vec![(key4, Some(&value4))],
        vec![(key5, Some(&value5))],
        vec![(key6, Some(&value6))],
    ];
    let db = MockTreeStore::default();
    let tree = JellyfishMerkleTree::new(&db);
    let mut idx = batches.len() as u64;

    for (idx, kvs) in batches.into_iter().enumerate() {
        let (_roots, batch) = tree.put_value_set_test(kvs, idx as Version).unwrap();
        db.write_tree_update_batch(batch).unwrap();
    }
    db.purge_stale_nodes(6).unwrap();
    assert_eq!(db.num_nodes(), 12);

    // Delete key3
    let (_roots, batch) = tree
        .put_value_set_test(vec![(key3, None)], idx as Version)
        .unwrap();
    db.write_tree_update_batch(batch).unwrap();
    assert_eq!(db.num_nodes(), 15 /* 12 + 3 */);
    db.purge_stale_nodes(idx).unwrap();
    assert_eq!(db.num_nodes(), 11);

    idx += 1;
    // Delete key1
    let (_roots, batch) = tree
        .put_value_set_test(vec![(key1, None)], idx as Version)
        .unwrap();
    db.write_tree_update_batch(batch).unwrap();
    assert_eq!(db.num_nodes(), 17 /* 11 + 6 */);
    db.purge_stale_nodes(idx).unwrap();
    assert_eq!(db.num_nodes(), 8);

    idx += 1;
    // Delete key5, key6 and key4
    let (_roots, batch) = tree
        .put_value_set_test(
            vec![(key4, None), (key5, None), (key6, None)],
            idx as Version,
        )
        .unwrap();
    db.write_tree_update_batch(batch).unwrap();
    assert_eq!(db.num_nodes(), 10 /* 8 + 2 */);
    db.purge_stale_nodes(idx).unwrap();
    assert_eq!(db.num_nodes(), 2);

    idx += 1;
    // Delete key2
    let (root, batch) = tree
        .put_value_set_test(vec![(key2, None)], idx as Version)
        .unwrap();
    db.write_tree_update_batch(batch).unwrap();
    assert_eq!(db.num_nodes(), 3 /* 2 + 1 */);
    db.purge_stale_nodes(idx).unwrap();
    assert_eq!(db.num_nodes(), 1);
    assert_eq!(root, *SPARSE_MERKLE_PLACEHOLDER_HASH);
}

#[test]
fn test_non_existence() {
    let db = MockTreeStore::default();
    let tree = JellyfishMerkleTree::new(&db);
    // ```text
    //                     internal(root)
    //                    /        \
    //                internal      2
    //                   |
    //                internal
    //                /      \
    //               1        3
    // Total: 7 nodes
    // ```
    let key1 = HashValue::new([0x00u8; HashValue::LENGTH]);
    let value1 = gen_value();

    let key2 = update_nibble(&key1, 0, 15);
    let value2 = gen_value();

    let key3 = update_nibble(&key1, 2, 3);
    let value3 = gen_value();

    let (root, batch) = tree
        .put_value_set_test(
            vec![
                (key1, Some(&value1)),
                (key2, Some(&value2)),
                (key3, Some(&value3)),
            ],
            0, /* version */
        )
        .unwrap();
    db.write_tree_update_batch(batch).unwrap();
    assert_eq!(tree.get(key1, 0).unwrap().unwrap(), value1.0);
    assert_eq!(tree.get(key2, 0).unwrap().unwrap(), value2.0);
    assert_eq!(tree.get(key3, 0).unwrap().unwrap(), value3.0);
    // get # of nodes
    assert_eq!(db.num_nodes(), 6);

    // test non-existing nodes.
    // 1. Non-existing node at root node
    {
        let non_existing_key = update_nibble(&key1, 0, 1);
        let (value, proof) = tree.get_with_proof(non_existing_key, 0).unwrap();
        assert_eq!(value, None);
        assert!(proof.verify_by_hash(root, non_existing_key, None).is_ok());
    }
    // 2. Non-existing node at non-root internal node
    {
        let non_existing_key = update_nibble(&key1, 1, 15);
        let (value, proof) = tree.get_with_proof(non_existing_key, 0).unwrap();
        assert_eq!(value, None);
        assert!(proof.verify_by_hash(root, non_existing_key, None).is_ok());
    }
    // 3. Non-existing node at leaf node
    {
        let non_existing_key = update_nibble(&key1, 2, 4);
        let (value, proof) = tree.get_with_proof(non_existing_key, 0).unwrap();
        assert_eq!(value, None);
        assert!(proof.verify_by_hash(root, non_existing_key, None).is_ok());
    }
}

#[test]
fn test_missing_root() {
    let db = MockTreeStore::<ValueBlob>::default();
    let tree = JellyfishMerkleTree::new(&db);
    let err = tree.get_with_proof(HashValue::random(), 0).err().unwrap();
    if let AptosDbError::MissingRootError(version) = err {
        assert_eq!(version, 0);
    } else {
        panic!("Unexpected error: {:?}", err);
    }
}

fn many_keys_get_proof_and_verify_tree_root(seed: &[u8], num_keys: usize) {
    assert!(seed.len() < 32);
    let mut actual_seed = [0u8; 32];
    actual_seed[..seed.len()].copy_from_slice(seed);
    let mut rng: StdRng = StdRng::from_seed(actual_seed);

    let db = MockTreeStore::default();
    let tree = JellyfishMerkleTree::new(&db);

    let mut kvs = vec![];

    let values: Vec<_> = (0..num_keys).map(|_i| gen_value()).collect();

    for (index, _) in values.iter().enumerate() {
        let key = HashValue::random_with_rng(&mut rng);
        kvs.push((key, Some(&values[index])));
    }

    let (root, batch) = tree
        .put_value_set_test(kvs.clone(), 0 /* version */)
        .unwrap();
    db.write_tree_update_batch(batch).unwrap();

    for (k, v) in &kvs {
        let (value, proof) = tree.get_with_proof(*k, 0).unwrap();
        assert_eq!(value.as_ref().unwrap().0, v.unwrap().0);
        assert_eq!(value.as_ref().unwrap().1.0, v.unwrap().1);
        assert!(proof.verify_by_hash(root, *k, v.map(|x| x.0)).is_ok());
    }
}

fn many_keys_deletion(seed: &[u8], num_keys: usize) {
    assert!(seed.len() < 32);
    let mut actual_seed = [0u8; 32];
    actual_seed[..seed.len()].copy_from_slice(seed);
    let mut rng: StdRng = StdRng::from_seed(actual_seed);

    let db = MockTreeStore::default();
    let tree = JellyfishMerkleTree::new(&db);

    let mut first_batch = vec![];

    let values: Vec<_> = (0..2 * num_keys).map(|_i| gen_value()).collect();

    for (index, _) in values.iter().enumerate() {
        let key = HashValue::random_with_rng(&mut rng);
        first_batch.push((key, Some(&values[index])));
    }

    let mut second_batch = first_batch[..num_keys]
        .iter()
        .map(|(k, _)| (*k, None))
        .collect::<Vec<_>>();

    let values: Vec<_> = (0..num_keys).map(|_i| gen_value()).collect();
    for (index, _) in values.iter().enumerate() {
        let key = HashValue::random_with_rng(&mut rng);
        second_batch.push((key, Some(&values[index])));
    }

    let (_root, batch) = tree
        .put_value_set_test(first_batch.clone(), 0 /* version */)
        .unwrap();
    db.write_tree_update_batch(batch).unwrap();
    let (root, batch) = tree
        .put_value_set_test(second_batch.clone(), 1 /* version */)
        .unwrap();
    db.write_tree_update_batch(batch).unwrap();

    for (k, v) in first_batch[num_keys..]
        .iter()
        .chain(second_batch[num_keys..].iter())
    {
        let (value, proof) = tree.get_with_proof(*k, 1).unwrap();
        assert_eq!(value.as_ref().unwrap().0, v.unwrap().0);
        assert_eq!(value.as_ref().unwrap().1.0, v.unwrap().1);
        assert!(proof.verify_by_hash(root, *k, v.map(|x| x.0)).is_ok());
    }

    for (k, _v) in first_batch[0..num_keys].iter() {
        let (value, proof) = tree.get_with_proof(*k, 1).unwrap();
        assert!(value.is_none());
        assert!(proof.verify_by_hash(root, *k, None).is_ok());
    }
}

#[test]
fn test_1000_keys() {
    let seed: &[_] = &[1, 2, 3, 4];
    many_keys_get_proof_and_verify_tree_root(seed, 1000);
}

#[test]
fn test_2000_keys_deletion() {
    let seed: &[_] = &[1, 2, 3, 4];
    many_keys_deletion(seed, 2000);
}

fn many_versions_get_proof_and_verify_tree_root(seed: &[u8], num_versions: usize) {
    assert!(seed.len() < 32);
    let mut actual_seed = [0u8; 32];
    actual_seed[..seed.len()].copy_from_slice(seed);
    let mut rng: StdRng = StdRng::from_seed(actual_seed);

    let db = MockTreeStore::default();
    let tree = JellyfishMerkleTree::new(&db);

    let mut kvs = vec![];
    let mut roots = vec![];

    let values: Vec<_> = (0..num_versions).map(|_i| gen_value()).collect();

    let new_values: Vec<_> = (0..num_versions).map(|_i| gen_value()).collect();

    for i in 0..num_versions {
        let key = HashValue::random_with_rng(&mut rng);
        kvs.push((key, Some(&values[i]), Some(&new_values[i])));
    }

    for (idx, kvs) in kvs.iter().enumerate() {
        let (root, batch) = tree
            .put_value_set_test(vec![(kvs.0, kvs.1)], idx as Version)
            .unwrap();
        roots.push(root);
        db.write_tree_update_batch(batch).unwrap();
    }

    // Update value of all keys
    for (idx, kvs) in kvs.iter().enumerate() {
        let version = (num_versions + idx) as Version;
        let (root, batch) = tree
            .put_value_set_test(vec![(kvs.0, kvs.2)], version)
            .unwrap();
        roots.push(root);
        db.write_tree_update_batch(batch).unwrap();
    }

    for (i, (k, v, _)) in kvs.iter().enumerate() {
        let random_version = rng.gen_range(i, i + num_versions);
        let (value, proof) = tree.get_with_proof(*k, random_version as Version).unwrap();
        assert_eq!(value.as_ref().unwrap().0, v.unwrap().0);
        assert_eq!(value.as_ref().unwrap().1.0, v.unwrap().1);
        assert!(
            proof
                .verify_by_hash(roots[random_version], *k, v.map(|x| x.0))
                .is_ok()
        );
    }

    for (i, (k, _, v)) in kvs.iter().enumerate() {
        let random_version = rng.gen_range(i + num_versions, 2 * num_versions);
        let (value, proof) = tree.get_with_proof(*k, random_version as Version).unwrap();
        assert_eq!(value.as_ref().unwrap().0, v.unwrap().0);
        assert_eq!(value.as_ref().unwrap().1.0, v.unwrap().1);
        assert!(
            proof
                .verify_by_hash(roots[random_version], *k, v.map(|x| x.0))
                .is_ok()
        );
    }
}

#[test]
fn test_1000_versions() {
    let seed: &[_] = &[1, 2, 3, 4];
    many_versions_get_proof_and_verify_tree_root(seed, 1000);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]

    #[test]
    fn proptest_get_with_proof((existent_kvs, nonexistent_keys) in arb_existent_kvs_and_nonexistent_keys::<ValueBlob>(1000, 100)) {
        test_get_with_proof((existent_kvs, nonexistent_keys))
    }

    #[test]
    fn proptest_get_with_proof_with_distinct_last_nibble((kv1, kv2) in arb_kv_pair_with_distinct_last_nibble::<ValueBlob>()) {
        test_get_with_proof_with_distinct_last_nibble((kv1, kv2))
    }

    #[test]
    fn proptest_get_range_proof((btree, n) in arb_tree_with_index::<ValueBlob>(1000)) {
        test_get_range_proof((btree, n))
    }

    #[test]
    fn proptest_get_leaf_count(keys in hash_set(any::<HashValue>(), 3..2000)) {
        test_get_leaf_count(keys)
    }
}
