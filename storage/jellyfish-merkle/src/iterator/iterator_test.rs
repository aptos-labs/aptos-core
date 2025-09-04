// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    iterator::JellyfishMerkleIterator,
    mock_tree_store::MockTreeStore,
    test_helper::{gen_value, plus_one},
    JellyfishMerkleTree,
};
use velor_crypto::HashValue;
use velor_storage_interface::Result;
use velor_types::transaction::Version;
use rand::{rngs::StdRng, SeedableRng};
use std::{collections::BTreeMap, sync::Arc};

#[test]
fn test_iterator_same_version() {
    for i in (1..100).step_by(11) {
        test_n_leaves_same_version(i);
    }
}

#[test]
fn test_iterator_multiple_versions() {
    test_n_leaves_multiple_versions(50);
}

#[test]
fn test_long_path() {
    test_n_consecutive_addresses(50);
}

fn test_n_leaves_same_version(n: usize) {
    let db = Arc::new(MockTreeStore::default());
    let tree = JellyfishMerkleTree::new(&*db);

    let mut rng = StdRng::from_seed([1; 32]);
    let values: Vec<_> = (0..n).map(|_i| gen_value()).collect();

    let mut btree = BTreeMap::new();
    for (index, _) in values.iter().enumerate() {
        let key = HashValue::random_with_rng(&mut rng);
        assert_eq!(btree.insert(key, Some(&values[index])), None);
    }

    let (_root_hash, batch) = tree
        .put_value_set_test(btree.clone().into_iter().collect(), 0 /* version */)
        .unwrap();
    db.write_tree_update_batch(batch).unwrap();
    let btree: BTreeMap<_, _> = btree
        .clone()
        .into_iter()
        .filter_map(|(x, y)| y.map(|y| (x, y.clone())))
        .collect();

    run_tests(db, &btree, 0 /* version */);
}

fn test_n_leaves_multiple_versions(n: usize) {
    let db = Arc::new(MockTreeStore::default());
    let tree = JellyfishMerkleTree::new(&*db);

    let mut rng = StdRng::from_seed([1; 32]);

    let mut btree = BTreeMap::new();
    for i in 0..n {
        let key = HashValue::random_with_rng(&mut rng);
        let value = gen_value();
        let (_root_hash, batch) = tree
            .put_value_set_test(vec![(key, Some(&value))], i as Version)
            .unwrap();
        assert_eq!(btree.insert(key, value), None);
        db.write_tree_update_batch(batch).unwrap();
        run_tests(Arc::clone(&db), &btree, i as Version);
    }
}

fn test_n_consecutive_addresses(n: usize) {
    let db = Arc::new(MockTreeStore::default());
    let tree = JellyfishMerkleTree::new(&*db);
    let values: Vec<_> = (0..n).map(|_i| gen_value()).collect();

    let btree: BTreeMap<_, _> = (0..n)
        .map(|i| (HashValue::from_u64(i as u64), Some(&values[i])))
        .collect();

    let (_root_hash, batch) = tree
        .put_value_set_test(btree.clone().into_iter().collect(), 0 /* version */)
        .unwrap();
    db.write_tree_update_batch(batch).unwrap();
    let btree: BTreeMap<_, _> = btree
        .clone()
        .into_iter()
        .filter_map(|(x, y)| y.map(|y| (x, y.clone())))
        .collect();

    run_tests(db, &btree, 0 /* version */);
}

fn run_tests<K>(
    db: Arc<MockTreeStore<K>>,
    btree: &BTreeMap<HashValue, (HashValue, K)>,
    version: Version,
) where
    K: crate::TestKey,
{
    {
        let iter =
            JellyfishMerkleIterator::new(Arc::clone(&db), version, HashValue::zero()).unwrap();
        assert_eq!(
            iter.collect::<Result<Vec<_>>>()
                .unwrap()
                .into_iter()
                .map(|x| (x.0, x.1 .0))
                .collect::<Vec<_>>(),
            btree
                .clone()
                .into_iter()
                .map(|x| (x.0, x.1 .1))
                .collect::<Vec<_>>(),
        );
    }

    for i in 0..btree.len() {
        {
            let iter = JellyfishMerkleIterator::new_by_index(Arc::clone(&db), version, i).unwrap();
            assert_eq!(
                iter.collect::<Result<Vec<_>>>()
                    .unwrap()
                    .into_iter()
                    .map(|x| (x.0, x.1 .0))
                    .collect::<Vec<_>>(),
                btree
                    .clone()
                    .into_iter()
                    .skip(i)
                    .map(|x| (x.0, x.1 .1))
                    .collect::<Vec<_>>(),
            );
        }

        let ith_key = *btree.keys().nth(i).unwrap();

        {
            let iter = JellyfishMerkleIterator::new(Arc::clone(&db), version, ith_key).unwrap();
            assert_eq!(
                iter.collect::<Result<Vec<_>>>()
                    .unwrap()
                    .into_iter()
                    .map(|x| (x.0, x.1 .0))
                    .collect::<Vec<_>>(),
                btree
                    .clone()
                    .into_iter()
                    .skip(i)
                    .map(|x| (x.0, x.1 .1))
                    .collect::<Vec<_>>(),
            );
        }

        {
            let ith_key_plus_one = plus_one(ith_key);
            let iter =
                JellyfishMerkleIterator::new(Arc::clone(&db), version, ith_key_plus_one).unwrap();
            assert_eq!(
                iter.collect::<Result<Vec<_>>>()
                    .unwrap()
                    .into_iter()
                    .map(|x| (x.0, x.1 .0))
                    .collect::<Vec<_>>(),
                btree
                    .clone()
                    .into_iter()
                    .skip(i + 1)
                    .map(|x| (x.0, x.1 .1))
                    .collect::<Vec<_>>(),
            );
        }
    }

    {
        let iter =
            JellyfishMerkleIterator::new_by_index(Arc::clone(&db), version, btree.len()).unwrap();
        assert_eq!(
            iter.collect::<Result<Vec<_>>>()
                .unwrap()
                .into_iter()
                .map(|x| (x.0, x.1 .0))
                .collect::<Vec<_>>(),
            vec![]
        );
    }

    {
        let iter = JellyfishMerkleIterator::new(
            Arc::clone(&db),
            version,
            HashValue::new([0xFF; HashValue::LENGTH]),
        )
        .unwrap();
        assert_eq!(
            iter.collect::<Result<Vec<_>>>()
                .unwrap()
                .into_iter()
                .map(|x| (x.0, x.1 .0))
                .collect::<Vec<_>>(),
            vec![]
        );
    }
}
