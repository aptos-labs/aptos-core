// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::{
    types::{test::KeyType, MVDataError, MVDataOutput, MVGroupError, TxnIndex},
    MVHashMap,
};
use aptos_types::{block_executor::value::SpeculativeValue, write_set::WriteOpKind};
use aptos_vm_types::resolver::ResourceGroupSize;
use bytes::Bytes;
use proptest::{collection::vec, prelude::*, sample::Index, strategy::Strategy};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt::Debug,
    hash::Hash,
    sync::atomic::{AtomicUsize, Ordering},
};

const DEFAULT_TIMEOUT: u64 = 30;

#[derive(Debug, Clone)]
enum Operator<V: Debug + Clone> {
    Insert(V),
    Remove,
    Read,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ExpectedOutput<V: Clone + Debug + Eq + PartialEq> {
    NotInMap,
    Deleted,
    Value(V),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MockValue<V: Eq + PartialEq> {
    maybe_value: Option<V>,
    maybe_bytes: Option<Bytes>,
}

impl<V: Into<Vec<u8>> + Clone + Eq + PartialEq> MockValue<V> {
    pub(crate) fn new(maybe_value: Option<V>) -> Self {
        let maybe_bytes = maybe_value.clone().map(|v| {
            let mut bytes = v.into();
            bytes.resize(16, 0);
            bytes.into()
        });
        Self {
            maybe_value,
            maybe_bytes,
        }
    }
}

impl<V: Into<Vec<u8>> + Clone + Debug + Eq + PartialEq + Send + Sync> SpeculativeValue
    for MockValue<V>
{
    fn eq_value(&self, other: &Self) -> bool {
        self == other
    }

    fn eq_metadata(&self, _other: &Self) -> bool {
        unimplemented!("Irrelevant for the tests")
    }

    fn bytes_len(&self) -> Option<usize> {
        self.maybe_value.as_ref().map(|v| v.clone().into().len())
    }

    fn is_deletion(&self) -> bool {
        // A missing value represents a deletion.
        self.maybe_value.is_none()
    }

    fn write_op_kind(&self) -> WriteOpKind {
        unimplemented!("Irrelevant for the tests")
    }
}

struct Baseline<K, V: Eq + PartialEq>(HashMap<K, BTreeMap<TxnIndex, MockValue<V>>>);

impl<K, V> Baseline<K, V>
where
    K: Hash + Eq + Clone + Debug,
    V: Clone + Into<Vec<u8>> + Debug + Eq + PartialEq,
{
    pub fn new(txns: &[(K, Operator<V>)]) -> Self {
        let mut baseline: HashMap<K, BTreeMap<TxnIndex, MockValue<V>>> = HashMap::new();
        for (idx, (k, op)) in txns.iter().enumerate() {
            let value_to_update = match op {
                Operator::Insert(v) => MockValue::new(Some(v.clone())),
                Operator::Remove => MockValue::new(None),
                Operator::Read => continue,
            };

            baseline
                .entry(k.clone())
                .or_default()
                .insert(idx as TxnIndex, value_to_update);
        }
        Self(baseline)
    }

    pub fn get(&self, key: &K, txn_idx: TxnIndex) -> ExpectedOutput<V> {
        match self.0.get(key).map(|tree| tree.range(..txn_idx)) {
            None => ExpectedOutput::NotInMap,
            Some(mut iter) => match iter.next_back() {
                None => ExpectedOutput::NotInMap,
                Some((_, v)) => match v.maybe_value.as_ref() {
                    Some(w) => ExpectedOutput::Value(w.clone()),
                    None => ExpectedOutput::Deleted,
                },
            },
        }
    }
}

fn operator_strategy<V: Arbitrary + Clone>() -> impl Strategy<Value = Operator<V>> {
    prop_oneof![
        2 => any::<V>().prop_map(Operator::Insert),
        1 => Just(Operator::Remove),
        2 => Just(Operator::Read),
    ]
}

// If test group is set, we prop-test the group_data multi-version hashmap, otherwise the
// data() multi-version hashmap.
fn run_and_assert<K, V>(
    universe: Vec<K>,
    transaction_gens: Vec<(Index, Operator<V>)>,
    test_group: bool,
) -> Result<(), TestCaseError>
where
    K: PartialOrd + Send + Clone + Hash + Eq + Sync + Debug,
    V: Send + Into<Vec<u8>> + Debug + Clone + Eq + PartialEq + Sync,
{
    let transactions: Vec<(K, Operator<V>)> = transaction_gens
        .into_iter()
        .map(|(idx, op)| (idx.get(&universe).clone(), op))
        .collect::<Vec<_>>();

    let baseline = Baseline::new(transactions.as_slice());
    let map = MVHashMap::<KeyType<K>, usize, MockValue<V>, ()>::new();

    // make ESTIMATE placeholders for all versions to be updated.
    // allows to test that correct values appear at the end of concurrent execution.
    let versions_to_write = transactions
        .iter()
        .enumerate()
        .filter_map(|(idx, (key, op))| match op {
            Operator::Read => None,
            Operator::Insert(_) | Operator::Remove => Some((key.clone(), idx)),
        })
        .collect::<Vec<_>>();
    for (key, idx) in versions_to_write {
        let key = KeyType(key);
        let value = MockValue::new(None);
        let idx = idx as TxnIndex;
        if test_group {
            map.group_data
                .set_raw_base_values(key.clone(), vec![])
                .unwrap();
            map.group_data()
                .write(
                    key.clone(),
                    idx,
                    0,
                    vec![(5, value)],
                    ResourceGroupSize::zero_combined(),
                    HashSet::new(),
                )
                .unwrap();
            let tags_5: Vec<usize> = vec![5];
            map.group_data()
                .mark_estimate(&key, idx, tags_5.iter().collect());
        } else {
            map.data().write(key.clone(), idx, 0, value).unwrap();
            map.data().mark_estimate(&key, idx);
        }
    }

    let current_idx = AtomicUsize::new(0);

    // Spawn a few threads in parallel to commit each operator.
    rayon::scope(|s| {
        for _ in 0..universe.len() {
            s.spawn(|_| loop {
                // Each thread will eagerly fetch an Operator to execute.
                let idx = current_idx.fetch_add(1, Ordering::Relaxed);
                if idx >= transactions.len() {
                    // Abort when all transactions are processed.
                    break;
                }
                let key = &transactions[idx].0;
                match &transactions[idx].1 {
                    Operator::Read => {
                        use MVDataError::*;
                        use MVDataOutput::*;

                        let baseline = baseline.get(key, idx as TxnIndex);
                        let assert_value = |v: MockValue<V>| match v.maybe_value.as_ref() {
                            Some(w) => {
                                assert_eq!(baseline, ExpectedOutput::Value(w.clone()), "{:?}", idx);
                            },
                            None => {
                                assert_eq!(baseline, ExpectedOutput::Deleted, "{:?}", idx);
                            },
                        };

                        let mut retry_attempts = 0;
                        loop {
                            if test_group {
                                match map.group_data.fetch_tagged_data_no_record(
                                    &KeyType(key.clone()),
                                    &5,
                                    idx as TxnIndex,
                                ) {
                                    Ok((_, v)) => {
                                        assert_value(v);
                                        break;
                                    },
                                    Err(MVGroupError::Uninitialized)
                                    | Err(MVGroupError::TagNotFound) => {
                                        assert_eq!(baseline, ExpectedOutput::NotInMap, "{:?}", idx);
                                        break;
                                    },
                                    Err(MVGroupError::Dependency(_i)) => (),
                                }
                            } else {
                                match map
                                    .data()
                                    .fetch_data_no_record(&KeyType(key.clone()), idx as TxnIndex)
                                {
                                    Ok(Versioned(_, v)) => {
                                        assert_value(v);
                                        break;
                                    },
                                    Err(Uninitialized) => {
                                        assert_eq!(baseline, ExpectedOutput::NotInMap, "{:?}", idx);
                                        break;
                                    },
                                    Err(Dependency(_i)) => (),
                                }
                            }
                            retry_attempts += 1;
                            if retry_attempts > DEFAULT_TIMEOUT {
                                panic!("Failed to get value for {:?}", idx);
                            }
                            std::thread::sleep(std::time::Duration::from_millis(100));
                        }
                    },
                    Operator::Remove => {
                        let key = KeyType(key.clone());
                        let value = MockValue::new(None);
                        if test_group {
                            map.group_data()
                                .write(
                                    key,
                                    idx as TxnIndex,
                                    1,
                                    vec![(5, value)],
                                    ResourceGroupSize::zero_combined(),
                                    HashSet::new(),
                                )
                                .unwrap();
                        } else {
                            map.data().write(key, idx as TxnIndex, 1, value).unwrap();
                        }
                    },
                    Operator::Insert(v) => {
                        let key = KeyType(key.clone());
                        let value = MockValue::new(Some(v.clone()));
                        if test_group {
                            map.group_data()
                                .write(
                                    key,
                                    idx as TxnIndex,
                                    1,
                                    vec![(5, value)],
                                    ResourceGroupSize::zero_combined(),
                                    HashSet::new(),
                                )
                                .unwrap();
                        } else {
                            map.data().write(key, idx as TxnIndex, 1, value).unwrap();
                        }
                    },
                }
            })
        }
    });

    Ok(())
}

proptest! {
    #[test]
    fn single_key_proptest(
        universe in vec(any::<[u8; 32]>(), 1),
        transactions in vec((any::<Index>(), operator_strategy::<[u8; 32]>()), 100),
    ) {
        run_and_assert(universe, transactions, false)?;
    }

    #[test]
    fn single_key_large_transactions(
        universe in vec(any::<[u8; 32]>(), 1),
        transactions in vec((any::<Index>(), operator_strategy::<[u8; 32]>()), 2000),
    ) {
        run_and_assert(universe, transactions, false)?;
    }

    #[test]
    fn multi_key_proptest(
        universe in vec(any::<[u8; 32]>(), 10),
        transactions in vec((any::<Index>(), operator_strategy::<[u8; 32]>()), 100),
    ) {
        run_and_assert(universe, transactions, false)?;
    }

    #[test]
    fn multi_key_proptest_group(
        universe in vec(any::<[u8; 32]>(), 3),
        transactions in vec((any::<Index>(), operator_strategy::<[u8; 32]>()), 200),
    ) {
        run_and_assert(universe, transactions, true)?;
    }
}
