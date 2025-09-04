// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::{
    types::{test::KeyType, Incarnation, MVDataError, MVDataOutput, TxnIndex, ValueWithLayout},
    MVHashMap,
};
use crate::{types::ShiftedTxnIndex, unit_tests::proptest_types::MockValue};
use claims::{assert_matches, assert_ok};
use concurrent_queue::ConcurrentQueue;
use proptest::{
    collection::vec, prelude::*, sample::Index, strategy::ValueTree, test_runner::TestRunner,
};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fmt::Debug,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    thread::sleep,
    time::Duration,
};
use test_case::test_case;

#[derive(Debug, Clone)]
enum Operator<V: Debug + Clone> {
    Insert(V),
    // InsertAndRemove transforms into two operations, with the second
    // operation taking place after the first operation is completed.
    InsertAndRemove(V),
    Read,
}

fn operator_strategy<V: Arbitrary + Clone>() -> impl Strategy<Value = Operator<V>> {
    prop_oneof![
        1 => any::<V>().prop_map(Operator::Insert),
        1 => any::<V>().prop_map(Operator::InsertAndRemove),
        1 => Just(Operator::Read),
    ]
}

/// This test works as follows:
/// 1. We generate a sequence of transactions based on the above Operator and
/// generate the expected baseline.
/// 2. The worker threads pick transactions from the queue and execute them.
/// 3. For the reads, the final result is recorded, and reads are rescheduled
/// when a write_v2 or remove_v2 invalidate the previous dependency record.
/// 4. In the end we simply ensure that the final result matches the expected
/// baseline, and is recorded once as a dependency in the corresponding entry.
#[test_case(30, 2, 1, 30, 5)]
#[test_case(50, 4, 3, 20, 5)]
#[test_case(100, 8, 3, 50, 0)]
#[test_case(1000, 16, 10, 1, 1)]
#[test_case(20, 6, 1, 100, 1)]
fn test_dependencies(
    num_txns: usize,
    num_workers: usize,
    universe_size: usize,
    num_random_generations: usize,
    sleep_millis: u64,
) {
    if num_workers > num_cpus::get() {
        // Ideally, we would want:
        // https://users.rust-lang.org/t/how-to-programatically-ignore-a-unit-test/64096/5
        return;
    }

    let mut runner = TestRunner::default();

    for _ in 0..num_random_generations {
        // Generate universe & transactions.
        let universe = vec(any::<[u8; 32]>(), universe_size)
            .new_tree(&mut runner)
            .expect("creating universe should succeed")
            .current();
        let transactions = vec((any::<Index>(), operator_strategy::<[u8; 32]>()), num_txns)
            .new_tree(&mut runner)
            .expect("creating transactions should succeed")
            .current()
            .into_iter()
            .map(|(idx, op)| (*idx.get(&universe), op))
            .collect::<Vec<_>>();

        let mut baseline = universe
            .iter()
            .map(|key| (*key, BTreeMap::new()))
            .collect::<HashMap<_, _>>();
        for (idx, (key, op)) in transactions.iter().enumerate() {
            if let Operator::Insert(v) = op {
                baseline
                    .entry(*key)
                    .or_default()
                    .insert(idx as TxnIndex, MockValue::new(Some(*v)));
            }
        }

        let map = MVHashMap::<KeyType<[u8; 32]>, usize, MockValue<[u8; 32]>, ()>::new();

        // Each read may get invalidated and be rescheduled, but since each original
        // txn performs at most one read, total number of rescheduled reads at any given
        // time is bounded by num_txns.
        let rescheduled_reads = ConcurrentQueue::<(TxnIndex, Incarnation)>::bounded(num_txns);
        // When a read occurs, if it does not read the correct value, the corresponding
        // correct_read flag is set to false (initialized to true since not all txns
        // contain a read), o.w. to true (if correct value is read). With each invalidations
        // causing a rescheduling, eventually all correct_read flags should be set to true.
        // The flag is stored in the least significant bit, while prefix is incarnation * 2,
        // so that (using fetch_max)the flag from the latest incarnation is recorded in the end.
        let correct_read = (0..num_txns)
            .map(|_| AtomicUsize::new(1))
            .collect::<Vec<_>>();

        let current_idx: AtomicUsize = AtomicUsize::new(0);
        rayon::scope(|s| {
            for _ in 0..num_workers {
                s.spawn(|_| loop {
                    let process_deps = |invalidated_deps: BTreeSet<(TxnIndex, Incarnation)>| {
                        for (txn_idx, incarnation) in invalidated_deps {
                            assert_ok!(rescheduled_reads.push((txn_idx, incarnation + 1)));
                        }
                    };

                    let maybe_perform_read = if current_idx.load(Ordering::Relaxed) < num_txns {
                        let idx = current_idx.fetch_add(1, Ordering::Relaxed);
                        if idx < num_txns {
                            let key = KeyType(transactions[idx].0);
                            match &transactions[idx].1 {
                                Operator::Read => Some((transactions[idx].0, idx as TxnIndex, 0)),
                                Operator::Insert(v) => {
                                    process_deps(map.data().write_v2::<false>(
                                        key,
                                        idx as TxnIndex,
                                        0,
                                        Arc::new(MockValue::new(Some(*v))),
                                        None,
                                    ));
                                    None
                                },
                                Operator::InsertAndRemove(v) => {
                                    process_deps(map.data().write_v2::<false>(
                                        key.clone(),
                                        idx as TxnIndex,
                                        0,
                                        Arc::new(MockValue::new(Some(*v))),
                                        None,
                                    ));
                                    sleep(Duration::from_millis(sleep_millis));
                                    process_deps(
                                        map.data()
                                            .remove_v2::<_, false>(&key, idx as TxnIndex)
                                            .unwrap(),
                                    );
                                    None
                                },
                            }
                        } else {
                            None
                        }
                    } else {
                        let ret = rescheduled_reads.pop().ok().map(|(txn_idx, incarnation)| {
                            assert_matches!(&transactions[txn_idx as usize].1, Operator::Read);
                            (transactions[txn_idx as usize].0, txn_idx, incarnation)
                        });
                        if ret.is_none() {
                            break;
                        }
                        ret
                    };

                    if let Some((key, txn_idx, incarnation)) = maybe_perform_read {
                        let speculative_read_value = map.data().fetch_data_and_record_dependency(
                            &KeyType(key),
                            txn_idx,
                            incarnation,
                        );
                        let correct = match speculative_read_value {
                            Ok(MVDataOutput::Versioned(_version, value)) => {
                                let correct = baseline
                                    .get(&key)
                                    .expect("key should exist in baseline")
                                    .range(..txn_idx)
                                    .next_back()
                                    .map_or_else(
                                        || {
                                            // Comparison ignores version since push invalidation
                                            // is based only on values.
                                            value
                                                == ValueWithLayout::Exchanged(
                                                    Arc::new(MockValue::new(None)),
                                                    None,
                                                )
                                        },
                                        |(_expected_txn_idx, expected_output)| {
                                            // Comparison ignores expected_txn_idx since push
                                            // validation is based only on values.
                                            value
                                                == ValueWithLayout::Exchanged(
                                                    Arc::new(expected_output.clone()),
                                                    None,
                                                )
                                        },
                                    );
                                correct
                            },
                            Err(MVDataError::Uninitialized) => {
                                map.data().set_base_value(
                                    KeyType(key),
                                    ValueWithLayout::Exchanged(
                                        Arc::new(MockValue::new(None)),
                                        None,
                                    ),
                                );
                                assert_ok!(rescheduled_reads.push((txn_idx, incarnation + 1)));
                                false
                            },
                            _ => unreachable!("Should be versioned or uninitialized"),
                        };

                        correct_read[txn_idx as usize].fetch_max(
                            incarnation as usize * 2 + correct as usize,
                            Ordering::Relaxed,
                        );
                    }
                });
            }
        });

        assert_eq!(rescheduled_reads.len(), 0);
        assert!(correct_read
            .iter()
            .all(|correct_flag| correct_flag.load(Ordering::Relaxed) & 1 == 1));

        let mut expected_deps: HashMap<
            KeyType<[u8; 32]>,
            BTreeMap<ShiftedTxnIndex, BTreeSet<(TxnIndex, Incarnation)>>,
        > = HashMap::new();
        for (idx, txn) in transactions.iter().enumerate() {
            if let Operator::Read = txn.1 {
                let expected_idx = baseline
                    .get(&txn.0)
                    .expect("key should exist in baseline")
                    .range(..idx as TxnIndex)
                    .next_back()
                    .map_or(ShiftedTxnIndex::zero_idx(), |(txn_idx, _)| {
                        ShiftedTxnIndex::new(*txn_idx)
                    });

                expected_deps
                    .entry(KeyType(txn.0))
                    .or_default()
                    .entry(expected_idx.clone())
                    .or_default()
                    .insert((
                        idx as TxnIndex,
                        correct_read[idx].load(Ordering::Relaxed) as u32 / 2,
                    ));
            }
        }

        for (key, expected_deps) in expected_deps {
            for (expected_idx, expected_deps) in expected_deps {
                let recorded_deps = map.data().get_dependencies(&key, expected_idx.clone());
                assert_eq!(recorded_deps, expected_deps);
            }
        }
    }
}
