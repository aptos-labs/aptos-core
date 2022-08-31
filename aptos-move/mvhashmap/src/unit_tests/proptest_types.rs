// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::{MVHashMap, MVHashMapError, MVHashMapOutput};
use aptos_aggregator::{
    delta_change_set::{delta_add, delta_sub, DeltaOp},
    transaction::AggregatorValue,
};
use aptos_types::write_set::TransactionWrite;
use proptest::{collection::vec, prelude::*, sample::Index, strategy::Strategy};
use std::{
    collections::{BTreeMap, HashMap},
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
    Update(DeltaOp),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ExpectedOutput<V: Debug + Clone + PartialEq> {
    NotInMap,
    Deleted,
    Value(V),
    Resolved(u128),
    Unresolved(DeltaOp),
    Failure,
}

struct Value<V>(Option<V>);

impl<V: Into<Vec<u8>> + Clone> TransactionWrite for Value<V> {
    fn extract_raw_bytes(&self) -> Option<Vec<u8>> {
        if self.0.is_none() {
            None
        } else {
            let mut bytes = match self.0.clone().map(|v| v.into()) {
                Some(v) => v,
                None => vec![],
            };

            bytes.resize(16, 0);
            Some(bytes)
        }
    }
}

enum Data<V> {
    Write(Value<V>),
    Delta(DeltaOp),
}
struct Baseline<K, V>(HashMap<K, BTreeMap<usize, Data<V>>>);

impl<K, V> Baseline<K, V>
where
    K: Hash + Eq + Clone,
    V: Clone + Into<Vec<u8>> + Debug + PartialEq,
{
    pub fn new(txns: &[(K, Operator<V>)]) -> Self {
        let mut baseline: HashMap<K, BTreeMap<usize, Data<V>>> = HashMap::new();
        for (idx, (k, op)) in txns.iter().enumerate() {
            let value_to_update = match op {
                Operator::Insert(v) => Data::Write(Value(Some(v.clone()))),
                Operator::Remove => Data::Write(Value(None)),
                Operator::Update(d) => Data::Delta(*d),
                Operator::Read => continue,
            };

            baseline
                .entry(k.clone())
                .or_insert_with(BTreeMap::new)
                .insert(idx, value_to_update);
        }
        Self(baseline)
    }

    pub fn get(&self, key: &K, version: usize) -> ExpectedOutput<V> {
        match self.0.get(key).map(|tree| tree.range(..version)) {
            None => ExpectedOutput::NotInMap,
            Some(mut iter) => {
                let mut acc: Option<DeltaOp> = None;
                while let Some((_, data)) = iter.next_back() {
                    match data {
                        Data::Write(v) => match acc {
                            Some(d) => {
                                let maybe_value =
                                    AggregatorValue::from_write(v).map(|value| value.into());
                                if maybe_value.is_none() {
                                    // v must be a deletion.
                                    assert!(matches!(v, Value(None)));
                                    return ExpectedOutput::Deleted;
                                }

                                match d.apply_to(maybe_value.unwrap()) {
                                    Err(_) => return ExpectedOutput::Failure,
                                    Ok(i) => return ExpectedOutput::Resolved(i),
                                }
                            }
                            None => match v {
                                Value(Some(w)) => return ExpectedOutput::Value(w.clone()),
                                Value(None) => return ExpectedOutput::Deleted,
                            },
                        },
                        Data::Delta(d) => match acc.as_mut() {
                            Some(a) => {
                                if a.merge_with(*d).is_err() {
                                    return ExpectedOutput::Failure;
                                }
                            }
                            None => acc = Some(*d),
                        },
                    }
                }

                match acc {
                    Some(d) => ExpectedOutput::Unresolved(d),
                    None => ExpectedOutput::NotInMap,
                }
            }
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

fn run_and_assert<K, V>(
    universe: Vec<K>,
    transaction_gens: Vec<(Index, Operator<V>)>,
) -> Result<(), TestCaseError>
where
    K: PartialOrd + Send + Clone + Hash + Eq + Sync,
    V: Send + Into<Vec<u8>> + Debug + Clone + PartialEq + Sync,
{
    let transactions: Vec<(K, Operator<V>)> = transaction_gens
        .into_iter()
        .map(|(idx, op)| (idx.get(&universe).clone(), op))
        .collect::<Vec<_>>();

    let baseline = Baseline::new(transactions.as_slice());
    let map = MVHashMap::<K, Value<V>>::new();

    // make ESTIMATE placeholders for all versions to be updated.
    // allows to test that correct values appear at the end of concurrent execution.
    let versions_to_write = transactions
        .iter()
        .enumerate()
        .filter_map(|(idx, (key, op))| match op {
            Operator::Read => None,
            Operator::Insert(_) | Operator::Remove | Operator::Update(_) => {
                Some((key.clone(), idx))
            }
        })
        .collect::<Vec<_>>();
    for (key, idx) in versions_to_write {
        map.add_write(&key, (idx, 0), Value(None));
        map.mark_estimate(&key, idx);
    }

    let curent_idx = AtomicUsize::new(0);

    // Spawn a few threads in parallel to commit each operator.
    rayon::scope(|s| {
        for _ in 0..universe.len() {
            s.spawn(|_| loop {
                // Each thread will eagerly fetch an Operator to execute.
                let idx = curent_idx.fetch_add(1, Ordering::Relaxed);
                if idx >= transactions.len() {
                    // Abort when all transactions are processed.
                    break;
                }
                let key = &transactions[idx].0;
                match &transactions[idx].1 {
                    Operator::Read => {
                        use MVHashMapError::*;
                        use MVHashMapOutput::*;

                        let baseline = baseline.get(key, idx);
                        let mut retry_attempts = 0;
                        loop {
                            match map.read(key, idx) {
                                Ok(Version(_, v)) => {
                                    match &*v {
                                        Value(Some(w)) => {
                                            assert_eq!(
                                                baseline,
                                                ExpectedOutput::Value(w.clone()),
                                                "{:?}",
                                                idx
                                            );
                                        }
                                        Value(None) => {
                                            assert_eq!(
                                                baseline,
                                                ExpectedOutput::Deleted,
                                                "{:?}",
                                                idx
                                            );
                                        }
                                    }
                                    break;
                                }
                                Ok(Resolved(v)) => {
                                    assert_eq!(baseline, ExpectedOutput::Resolved(v), "{:?}", idx);
                                    break;
                                }
                                Err(NotFound) => {
                                    assert_eq!(baseline, ExpectedOutput::NotInMap, "{:?}", idx);
                                    break;
                                }
                                Err(DeltaApplicationFailure) => {
                                    assert_eq!(baseline, ExpectedOutput::Failure, "{:?}", idx);
                                    break;
                                }
                                Err(Unresolved(d)) => {
                                    assert_eq!(
                                        baseline,
                                        ExpectedOutput::Unresolved(d),
                                        "{:?}",
                                        idx
                                    );
                                    break;
                                }
                                Err(Dependency(_i)) => (),
                            }
                            retry_attempts += 1;
                            if retry_attempts > DEFAULT_TIMEOUT {
                                panic!("Failed to get value for {:?}", idx);
                            }
                            std::thread::sleep(std::time::Duration::from_millis(100));
                        }
                    }
                    Operator::Remove => {
                        map.add_write(key, (idx, 1), Value(None));
                    }
                    Operator::Insert(v) => {
                        map.add_write(key, (idx, 1), Value(Some(v.clone())));
                    }
                    Operator::Update(delta) => map.add_delta(key, idx, *delta),
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
        run_and_assert(universe, transactions)?;
    }

    #[test]
    fn single_key_large_transactions(
        universe in vec(any::<[u8; 32]>(), 1),
        transactions in vec((any::<Index>(), operator_strategy::<[u8; 32]>()), 2000),
    ) {
        run_and_assert(universe, transactions)?;
    }

    #[test]
    fn multi_key_proptest(
        universe in vec(any::<[u8; 32]>(), 10),
        transactions in vec((any::<Index>(), operator_strategy::<[u8; 32]>()), 100),
    ) {
        run_and_assert(universe, transactions)?;
    }
}
