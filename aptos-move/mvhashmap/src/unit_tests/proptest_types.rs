// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::{
    types::{test::KeyType, MVDataError, MVDataOutput, MVGroupError, TxnIndex},
    MVHashMap,
};
use crate::types::ValueWithLayout;
use aptos_aggregator::delta_change_set::{delta_add, delta_sub, DeltaOp};
use aptos_types::{
    state_store::state_value::StateValue,
    write_set::{TransactionWrite, WriteOpKind},
};
use aptos_vm_types::resolver::ResourceGroupSize;
use bytes::Bytes;
use claims::assert_none;
use proptest::{collection::vec, prelude::*, sample::Index, strategy::Strategy};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt::Debug,
    hash::Hash,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

const DEFAULT_TIMEOUT: u64 = 30;

#[derive(Debug, Clone)]
enum Operator<V: Debug + Clone> {
    Insert(V),
    Remove,
    Read,
    Update(DeltaOp),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ExpectedOutput<V: Clone + Debug + Eq + PartialEq> {
    NotInMap,
    Deleted,
    Value(V),
    Resolved(u128),
    Unresolved(DeltaOp),
    Failure,
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

impl<V: Into<Vec<u8>> + Clone + Debug + Eq + PartialEq> TransactionWrite for MockValue<V> {
    fn bytes(&self) -> Option<&Bytes> {
        self.maybe_bytes.as_ref()
    }

    fn write_op_kind(&self) -> WriteOpKind {
        unimplemented!("Irrelevant for the test")
    }

    fn from_state_value(_maybe_state_value: Option<StateValue>) -> Self {
        unimplemented!("Irrelevant for the test")
    }

    fn as_state_value(&self) -> Option<StateValue> {
        unimplemented!("Irrelevant for the test")
    }

    fn set_bytes(&mut self, bytes: Bytes) {
        self.maybe_bytes = Some(bytes);
    }
}

enum Data<V: Eq + PartialEq> {
    Write(MockValue<V>),
    Delta(DeltaOp),
}
struct Baseline<K, V: Eq + PartialEq>(HashMap<K, BTreeMap<TxnIndex, Data<V>>>);

impl<K, V> Baseline<K, V>
where
    K: Hash + Eq + Clone + Debug,
    V: Clone + Into<Vec<u8>> + Debug + Eq + PartialEq,
{
    pub fn new(txns: &[(K, Operator<V>)], ignore_updates: bool) -> Self {
        let mut baseline: HashMap<K, BTreeMap<TxnIndex, Data<V>>> = HashMap::new();
        for (idx, (k, op)) in txns.iter().enumerate() {
            let value_to_update = match op {
                Operator::Insert(v) => Data::Write(MockValue::new(Some(v.clone()))),
                Operator::Remove => Data::Write(MockValue::new(None)),
                Operator::Update(d) => {
                    if ignore_updates {
                        continue;
                    }
                    Data::Delta(*d)
                },
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
            Some(mut iter) => {
                let mut acc: Option<DeltaOp> = None;
                let mut failure = false;
                while let Some((_, data)) = iter.next_back() {
                    match data {
                        Data::Write(v) => match acc {
                            Some(d) => {
                                match v.as_u128().unwrap() {
                                    Some(value) => {
                                        assert!(!failure); // acc should be none.
                                        match d.apply_to(value) {
                                            Err(_) => return ExpectedOutput::Failure,
                                            Ok(i) => return ExpectedOutput::Resolved(i),
                                        }
                                    },
                                    None => {
                                        // v must be a deletion.
                                        assert_none!(v.bytes());
                                        return ExpectedOutput::Deleted;
                                    },
                                }
                            },
                            None => match v.maybe_value.as_ref() {
                                Some(w) => {
                                    return if failure {
                                        ExpectedOutput::Failure
                                    } else {
                                        ExpectedOutput::Value(w.clone())
                                    };
                                },
                                None => return ExpectedOutput::Deleted,
                            },
                        },
                        Data::Delta(d) => match acc.as_mut() {
                            Some(a) => {
                                if a.merge_with_previous_delta(*d).is_err() {
                                    failure = true;
                                }
                            },
                            None => acc = Some(*d),
                        },
                    }

                    if failure {
                        // for overriding the delta failure if entry is deleted.
                        acc = None;
                    }
                }

                if failure {
                    ExpectedOutput::Failure
                } else {
                    match acc {
                        Some(d) => ExpectedOutput::Unresolved(d),
                        None => ExpectedOutput::NotInMap,
                    }
                }
            },
        }
    }
}

fn operator_strategy<V: Arbitrary + Clone>() -> impl Strategy<Value = Operator<V>> {
    prop_oneof![
        2 => any::<V>().prop_map(Operator::Insert),
        4 => any::<u32>().prop_map(|v| {
            // TODO: Is there a proptest way of doing that?
            if v % 2 == 0 {
                Operator::Update(delta_sub(v as u128, u32::MAX as u128))
            } else {
                Operator::Update(delta_add(v as u128, u32::MAX as u128))
            }
        }),
        1 => Just(Operator::Remove),
        1 => Just(Operator::Read),
    ]
}

// If test group is set, we prop-test the group_data multi-version hashmap: we ignore the
// Update/Deltas (as only data() MVHashMap deals with AggregatorV1 and even that will get
// deprecated in favor of the dedicated aggregator MVHashMap for AggregatorV2).
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

    let baseline = Baseline::new(transactions.as_slice(), test_group);
    let map = MVHashMap::<KeyType<K>, usize, MockValue<V>, ()>::new();

    // make ESTIMATE placeholders for all versions to be updated.
    // allows to test that correct values appear at the end of concurrent execution.
    let versions_to_write = transactions
        .iter()
        .enumerate()
        .filter_map(|(idx, (key, op))| match op {
            Operator::Read => None,
            Operator::Insert(_) | Operator::Remove => Some((key.clone(), idx)),
            Operator::Update(_) => (!test_group).then_some((key.clone(), idx)),
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
                    vec![(5, (value, None))],
                    ResourceGroupSize::zero_combined(),
                    HashSet::new(),
                )
                .unwrap();
            let tags_5: Vec<usize> = vec![5];
            map.group_data()
                .mark_estimate(&key, idx, tags_5.iter().collect());
        } else {
            map.data().write(key.clone(), idx, 0, Arc::new(value), None);
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
                        let assert_value = |v: ValueWithLayout<MockValue<V>>| match v
                            .extract_value_no_layout()
                            .maybe_value
                            .as_ref()
                        {
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
                                    Ok(Resolved(v)) => {
                                        assert_eq!(
                                            baseline,
                                            ExpectedOutput::Resolved(v),
                                            "{:?}",
                                            idx
                                        );
                                        break;
                                    },
                                    Err(Uninitialized) => {
                                        assert_eq!(baseline, ExpectedOutput::NotInMap, "{:?}", idx);
                                        break;
                                    },
                                    Err(DeltaApplicationFailure) => {
                                        assert_eq!(baseline, ExpectedOutput::Failure, "{:?}", idx);
                                        break;
                                    },
                                    Err(Unresolved(d)) => {
                                        assert_eq!(
                                            baseline,
                                            ExpectedOutput::Unresolved(d),
                                            "{:?}",
                                            idx
                                        );
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
                                    vec![(5, (value, None))],
                                    ResourceGroupSize::zero_combined(),
                                    HashSet::new(),
                                )
                                .unwrap();
                        } else {
                            map.data()
                                .write(key, idx as TxnIndex, 1, Arc::new(value), None);
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
                                    vec![(5, (value, None))],
                                    ResourceGroupSize::zero_combined(),
                                    HashSet::new(),
                                )
                                .unwrap();
                        } else {
                            map.data()
                                .write(key, idx as TxnIndex, 1, Arc::new(value), None);
                        }
                    },
                    Operator::Update(delta) => {
                        if !test_group {
                            map.data()
                                .add_delta(KeyType(key.clone()), idx as TxnIndex, *delta)
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
