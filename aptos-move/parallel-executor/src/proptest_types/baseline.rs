// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    errors::{Error, Result},
    executor::{ReadResult, DELTA_FAILURE_AGGREGATOR_VALUE},
    proptest_types::types::{Output, Transaction},
};
use aptos_aggregator::{delta_change_set::DeltaOp, transaction::AggregatorValue};
use aptos_types::write_set::{TransactionWrite, WriteOp};
use claims::{assert_none, assert_some};
use std::{collections::HashMap, fmt::Debug, hash::Hash, sync::atomic::Ordering};

///////////////////////////////////////////////////////////////////////////
// Sequential Baseline implementation.
///////////////////////////////////////////////////////////////////////////

#[derive(Clone, Debug)]
pub struct BaselineState<V> {
    // If last WriteOp wrote val: V, this will be Some(val).
    maybe_v: Option<V>,
    // The correct aggregator value can be computed by applying the sequence
    // of suffix deltas to the base value, similar to how a stateview read
    // would behave. Computing the value incrementally may introduce a drift
    // for every delta application error and not be correct.
    delta_suffix: (Option<u128>, Vec<DeltaOp>),
}

impl<V: Debug + Clone + PartialEq + Eq + TransactionWrite> BaselineState<V> {
    fn from_v(v: V) -> Self {
        // If v corresponds to a deletion, maybe_base will be None.
        let maybe_base = AggregatorValue::from_write(&v).map(|base| base.into());
        Self {
            maybe_v: Some(v),
            delta_suffix: (maybe_base, Vec::new()),
        }
    }

    fn from_u128(value: u128) -> Self {
        Self {
            maybe_v: None,
            delta_suffix: (Some(value), Vec::new()),
        }
    }

    fn from_delta(delta: DeltaOp, storage_default: Option<u128>) -> Self {
        Self {
            maybe_v: None,
            delta_suffix: (storage_default, vec![delta]),
        }
    }

    fn apply_delta(&mut self, delta: DeltaOp) {
        // If maybe_v corresponds to deletion, delta is not applied.
        if self.delta_suffix.0.is_some()
            || (self.delta_suffix.0.is_none() && self.maybe_v.is_none())
        {
            self.maybe_v = None;
            self.delta_suffix.1.push(delta);
        }
    }

    pub fn aggregator_value(&self) -> u128 {
        let mut ret = self.delta_suffix.0.unwrap();
        for delta in self.delta_suffix.1.iter() {
            ret = match delta.apply_to(ret) {
                Ok(value) => value,
                Err(_) => return DELTA_FAILURE_AGGREGATOR_VALUE,
            };
        }
        ret
    }

    fn check_v(&self, other_v: V, other_resolved: bool) {
        // If other wasn't explicitly resolved, then baseline should also hold V.
        assert!(other_resolved || self.maybe_v.is_some());
        match &self.maybe_v {
            Some(v) => assert_eq!(*v, other_v),
            None => assert_eq!(
                AggregatorValue::from_write(&other_v).unwrap().into(),
                self.aggregator_value()
            ),
        }
    }

    fn merge_deltas(&self) -> Option<DeltaOp> {
        let mut d = *self.delta_suffix.1.last().unwrap();
        for prev_d in self.delta_suffix.1.iter().rev().skip(1) {
            if d.merge_onto(*prev_d).is_err() {
                return None;
            }
        }
        Some(d)
    }

    fn check_u128(&self, value: u128) {
        if value != DELTA_FAILURE_AGGREGATOR_VALUE || self.delta_suffix.0.is_some() {
            assert_eq!(self.aggregator_value(), value);
        } else {
            // Delta merging must have failed.
            assert_none!(self.merge_deltas());
        }
    }

    fn check_deltas(&self, delta: DeltaOp) {
        assert_none!(&self.maybe_v);
        assert_eq!(self.merge_deltas().unwrap(), delta);
    }
}

/// Sequential baseline of execution result for dummy transaction.
pub enum ExpectedOutput<V> {
    Aborted(usize),
    // Both 'SkipRest' and 'Success' contain BaselineState per executed transaction,
    // describing the baseline (expected) output. 'SkipRest' additionally contains
    // the index after which transaction execution is skipped. Finally, the last
    // parameter is the storage default value, for testing the resolver.
    SkipRest(usize, Vec<Vec<Option<BaselineState<V>>>>, Option<u128>),
    Success(Vec<Vec<Option<BaselineState<V>>>>, Option<u128>),
}

impl<V: Debug + Clone + PartialEq + Eq + TransactionWrite> ExpectedOutput<V> {
    /// Must be invoked after parallel execution to work with dynamic read/writes.
    pub fn generate_baseline<K: Hash + Clone + Eq>(
        txns: &[Transaction<K, V>],
        // Resolved deltas.
        resolver_output: Option<Vec<Vec<(K, WriteOp)>>>,
        // Storage default used to resolve (by resolver, or sequential).
        storage_default: Option<u128>,
    ) -> Self {
        let mut current_world: HashMap<K, BaselineState<V>> = HashMap::new();

        let resolved_delta_writes = resolver_output.map(|delta_writes| {
            delta_writes
                .into_iter()
                .map(|at_idx| at_idx.into_iter().collect::<HashMap<K, WriteOp>>())
                .collect::<Vec<HashMap<K, WriteOp>>>()
        });
        if resolved_delta_writes.is_some() {
            assert_some!(storage_default);
        }
        let mut result_vec = vec![];
        for (idx, txn) in txns.iter().enumerate() {
            match txn {
                Transaction::Abort => return Self::Aborted(idx),
                Transaction::Write {
                    incarnation,
                    writes_and_deltas,
                    reads,
                } => {
                    // Determine the read and write sets of the latest incarnation
                    // of the transaction. The index for choosing the read and
                    // write sets is based on the value of the incarnation counter
                    // prior to the fetch_add during the last execution.
                    let last_incarnation = if reads.len() > 1 || writes_and_deltas.len() > 1 {
                        let i = incarnation.load(Ordering::SeqCst);
                        assert!(i > 0, "must run after parallel execution");
                        i - 1
                    } else {
                        0
                    };

                    // Determine the read-, delta- and write-sets of the latest
                    // incarnation during parallel execution to use for the baseline.
                    let read_set = &reads[last_incarnation as usize % reads.len()];
                    let (write_set, delta_set) =
                        &writes_and_deltas[last_incarnation as usize % writes_and_deltas.len()];

                    let mut result = vec![];
                    for k in read_set.iter() {
                        result.push(current_world.get(k).cloned());
                    }

                    // We ensure that the latest state is always reflected in exactly one of
                    // the hashmaps, by possibly removing an element from the other Hashmap.
                    for (k, v) in write_set.iter() {
                        current_world.insert(k.clone(), BaselineState::from_v(v.clone()));
                    }

                    for (k, delta) in delta_set.iter() {
                        match resolved_delta_writes.as_ref() {
                            Some(delta_writes) => {
                                // Make sure resolver resolved all deltas.
                                assert_eq!(delta_writes[idx].len(), delta_set.len());
                                current_world.insert(
                                    k.clone(),
                                    BaselineState::from_u128(
                                        AggregatorValue::from_write(
                                            delta_writes[idx].get(k).unwrap(),
                                        )
                                        .unwrap()
                                        .into(),
                                    ),
                                );
                            }
                            None => {
                                current_world
                                    .entry(k.clone())
                                    .and_modify(|state| state.apply_delta(*delta))
                                    .or_insert_with(|| {
                                        BaselineState::from_delta(*delta, storage_default)
                                    });
                            }
                        }
                    }

                    result_vec.push(result)
                }
                Transaction::SkipRest => return Self::SkipRest(idx, result_vec, storage_default),
            }
        }
        Self::Success(result_vec, storage_default)
    }

    fn check_result(
        expected_results: &[Option<BaselineState<V>>],
        results: &[ReadResult<V>],
        execution_resolved: bool,
        storage_default: Option<u128>,
    ) {
        expected_results
            .iter()
            .zip(results.iter())
            .for_each(|(expected_result, result)| match result {
                ReadResult::Value(v) => {
                    expected_result
                        .as_ref()
                        .unwrap()
                        .check_v((**v).clone(), execution_resolved);
                }
                ReadResult::U128(v) => {
                    expected_result.as_ref().unwrap().check_u128(*v);
                }
                ReadResult::Unresolved(delta) => {
                    // If execution (sequential) resolved deltas, the read would never
                    // return Unresolved.
                    assert!(!execution_resolved);

                    if storage_default.is_some() {
                        // Since execution_resolved == false, we are testing Resolver.
                        let exec_state: BaselineState<V> =
                            BaselineState::from_delta(*delta, storage_default);
                        expected_result
                            .as_ref()
                            .unwrap()
                            .check_u128(exec_state.aggregator_value());
                    } else {
                        expected_result.as_ref().unwrap().check_deltas(*delta);
                    }
                }
                ReadResult::None => {
                    assert_none!(expected_result);
                }
            })
    }

    // Used for testing, hence the function asserts the correctness conditions within
    // itself to be easily traceable in case of an error.
    pub fn assert_output<K>(
        &self,
        results: &Result<Vec<Output<K, V>>, usize>,
        // e.g. in sequential execution, aggregator deltas are resolved.
        execution_resolved: bool,
    ) {
        match (self, results) {
            (Self::Aborted(i), Err(Error::UserError(idx))) => {
                assert_eq!(i, idx);
            }
            (Self::SkipRest(skip_at, expected_results, storage_default), Ok(results)) => {
                // Check_result asserts internally, so no need to return a bool.
                results
                    .iter()
                    .take(*skip_at)
                    .zip(expected_results.iter())
                    .for_each(|(Output(_, _, result), expected_result)| {
                        Self::check_result(
                            expected_result,
                            result,
                            execution_resolved,
                            *storage_default,
                        )
                    });

                results
                    .iter()
                    .skip(*skip_at)
                    .for_each(|Output(_, _, result)| assert!(result.is_empty()))
            }
            (Self::Success(expected_results, storage_default), Ok(results)) => results
                .iter()
                .zip(expected_results.iter())
                .for_each(|(Output(_, _, result), expected_result)| {
                    Self::check_result(
                        expected_result,
                        result,
                        execution_resolved,
                        *storage_default,
                    );
                }),
            _ => panic!("Incomparable execution outcomes"),
        }
    }
}
