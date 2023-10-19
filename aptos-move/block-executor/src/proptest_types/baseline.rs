// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

/// This file implements the baseline evaluation, performed sequentially, the output
/// of which is used to test the results of the block executor. The baseline must be
/// evaluated after the block executor has completed, as the transaction type used
/// for testing tracks the incarnation number, which is used to emulate dynamic behavior.
/// Dynamic behavior means that when a transaction is re-executed, it might read
/// different values and end up with a completely different behavior (be it read set,
/// write set, or executed code). In the tests, behavior changes based on the incarnation
/// number, and hence it is crucial for the baseline to know the final incarnation number
/// of each transaction of the tested block executor execution.
use crate::{
    errors::{Error as BlockExecutorError, Result as BlockExecutorResult},
    proptest_types::types::{MockOutput, MockTransaction, STORAGE_AGGREGATOR_VALUE},
};
use aptos_aggregator::{delta_change_set::serialize, transaction::AggregatorValue};
use aptos_types::{contract_event::ReadWriteEvent, write_set::TransactionWrite};
use claims::{assert_matches, assert_none, assert_some_eq};
use itertools::izip;
use std::{collections::HashMap, fmt::Debug, hash::Hash, result::Result, sync::atomic::Ordering};

// TODO: extend to derived values, and code.
#[derive(Clone)]
enum BaselineValue<V> {
    GenericWrite(V),
    Aggregator(u128),
    Empty,
}

// TODO: instead of the same Error on aggregator overflow / underflow, add a way to tell
// from the error and test it.
// enum AggregatorError {
//    Overflow,
//    Underflow,
// }

impl<V: Debug + Clone + PartialEq + Eq + TransactionWrite> BaselineValue<V> {
    // Compare to the read results during block execution.
    pub(crate) fn assert_read_result(&self, bytes_read: &Option<Vec<u8>>) {
        match (self, bytes_read) {
            (BaselineValue::GenericWrite(v), Some(bytes)) => {
                assert_some_eq!(v.extract_raw_bytes(), *bytes);
            },
            (BaselineValue::GenericWrite(v), None) => {
                assert_none!(v.extract_raw_bytes());
            },
            (BaselineValue::Aggregator(aggr_value), Some(bytes)) => {
                assert_eq!(serialize(aggr_value), *bytes);
            },
            (BaselineValue::Aggregator(_), None) => unreachable!(
                "Deleted or non-existent value from storage can't match aggregator value"
            ),
            (BaselineValue::Empty, Some(bytes)) => {
                assert_eq!(serialize(&STORAGE_AGGREGATOR_VALUE), *bytes);
            },
            (BaselineValue::Empty, None) => (),
        }
    }
}

// The status of the baseline execution.
#[derive(Debug)]
enum BaselineStatus {
    Success,
    Aborted,
    SkipRest,
    GasLimitExceeded,
}

/// Sequential baseline of execution result for dummy transaction, containing a vector
/// of BaselineValues for the reads of the (latest incarnation of the dummy) transaction.
/// The size of the vector should be equal to the size of the block if the block execution
/// was successful. Otherwise, it is the index of a transaction where the block execution
/// stopped, e.g. due to gas limit, abort, or reconfiguration (skip rest status). It also
/// contains resolved values for each of the deltas produced by the dummy transaction.
///
/// For both read_values and resolved_deltas the keys are not included because they are
/// in the same order as the reads and deltas in the Transaction::Write.
pub(crate) struct BaselineOutput<V> {
    status: BaselineStatus,
    read_values: Vec<Result<Vec<BaselineValue<V>>, ()>>,
    resolved_deltas: Vec<Result<Vec<u128>, ()>>,
}

impl<V: Debug + Clone + PartialEq + Eq + TransactionWrite> BaselineOutput<V> {
    /// Must be invoked after parallel execution to have incarnation information set and
    /// work with dynamic read/writes.
    pub(crate) fn generate<K: Hash + Clone + Eq, E: Debug + Clone + ReadWriteEvent>(
        txns: &[MockTransaction<K, V, E>],
        maybe_block_gas_limit: Option<u64>,
    ) -> Self {
        let mut current_world = HashMap::<K, BaselineValue<_>>::new();
        let mut accumulated_gas = 0;

        let mut status = BaselineStatus::Success;
        let mut read_values = vec![];
        let mut resolved_deltas = vec![];
        for txn in txns.iter() {
            match txn {
                MockTransaction::Abort => {
                    status = BaselineStatus::Aborted;
                    break;
                },
                MockTransaction::SkipRest => {
                    // In executor, SkipRest skips from the next index. Test assumes it's an empty
                    // transaction, so create a successful empty reads and deltas.
                    read_values.push(Ok(vec![]));
                    resolved_deltas.push(Ok(vec![]));

                    status = BaselineStatus::SkipRest;
                    break;
                },
                MockTransaction::Write {
                    incarnation_counter,
                    incarnation_behaviors,
                } => {
                    // Determine the behavior of the latest incarnation of the transaction. The index
                    // is based on the value of the incarnation counter prior to the fetch_add during
                    // the last mock execution, and is >= 1 because there is at least one execution.
                    let last_incarnation = (incarnation_counter.load(Ordering::SeqCst) - 1)
                        % incarnation_behaviors.len();

                    match incarnation_behaviors[last_incarnation]
                        .deltas
                        .iter()
                        .map(|(k, delta)| {
                            let base = match current_world
                                .entry(k.clone())
                                .or_insert(BaselineValue::Empty)
                            {
                                // Get base value from the latest write.
                                BaselineValue::GenericWrite(w_value) => {
                                    AggregatorValue::from_write(w_value)
                                        .expect("Delta to a non-existent aggregator")
                                        .into()
                                },
                                // Get base value from latest resolved aggregator value.
                                BaselineValue::Aggregator(value) => *value,
                                // Storage always gets resolved to a default constant.
                                BaselineValue::Empty => STORAGE_AGGREGATOR_VALUE,
                            };

                            delta
                                .apply_to(base)
                                .map(|resolved_value| (k.clone(), resolved_value))
                        })
                        .collect::<Result<Vec<_>, _>>()
                    {
                        Ok(txn_resolved_deltas) => {
                            // Update the read_values and resolved_deltas. Performing reads here is
                            // correct because written_ and resolved_ worlds have not been updated.
                            read_values.push(Ok(incarnation_behaviors[last_incarnation]
                                .reads
                                .iter()
                                .map(|k| {
                                    current_world
                                        .entry(k.clone())
                                        .or_insert(BaselineValue::Empty)
                                        .clone()
                                })
                                .collect()));

                            resolved_deltas.push(Ok(txn_resolved_deltas
                                .into_iter()
                                .map(|(k, v)| {
                                    // In this case transaction did not fail due to delta application
                                    // errors, and thus we should update written_ and resolved_ worlds.
                                    current_world.insert(k, BaselineValue::Aggregator(v));
                                    v
                                })
                                .collect()));

                            // We ensure that the latest state is always reflected in exactly one of
                            // the hashmaps, by possibly removing an element from the other Hashmap.
                            for (k, v) in incarnation_behaviors[last_incarnation].writes.iter() {
                                current_world
                                    .insert(k.clone(), BaselineValue::GenericWrite(v.clone()));
                            }

                            // Apply gas.
                            accumulated_gas += incarnation_behaviors[last_incarnation].gas;
                            if let Some(block_gas_limit) = maybe_block_gas_limit {
                                if accumulated_gas >= block_gas_limit {
                                    status = BaselineStatus::GasLimitExceeded;
                                    break;
                                }
                            }
                        },
                        Err(_) => {
                            // Transaction does not take effect and we record delta application failure.
                            read_values.push(Err(()));
                            resolved_deltas.push(Err(()));
                        },
                    }
                },
            }
        }

        Self {
            status,
            read_values,
            resolved_deltas,
        }
    }

    // Used for testing, hence the function asserts the correctness conditions within
    // itself to be easily traceable in case of an error.
    pub(crate) fn assert_output<K: Debug, E: Debug>(
        &self,
        results: &BlockExecutorResult<Vec<MockOutput<K, V, E>>, usize>,
    ) {
        match results {
            Ok(results) => {
                let committed = self.read_values.len();
                assert_eq!(self.resolved_deltas.len(), committed);

                // Check read values & delta writes.
                izip!(
                    results.iter().take(committed),
                    self.read_values.iter(),
                    self.resolved_deltas.iter()
                )
                .for_each(|(output, reads, resolved_deltas)| {
                    reads
                        .as_ref()
                        .expect("Aggregator failures not yet tested")
                        .iter()
                        .zip(output.read_results.iter())
                        .for_each(|(baseline_read, result_read)| {
                            baseline_read.assert_read_result(result_read)
                        });

                    resolved_deltas
                        .as_ref()
                        .expect("Aggregator failures not yet tested")
                        .iter()
                        .zip(
                            output
                                .materialized_delta_writes
                                .get()
                                .expect("Delta writes must be set")
                                .iter(),
                        )
                        .for_each(|(baseline_delta_write, (_, result_delta_write))| {
                            assert_eq!(
                                *baseline_delta_write,
                                AggregatorValue::from_write(result_delta_write)
                                    .expect("Delta to a non-existent aggregator")
                                    .into()
                            );
                        });
                });

                results.iter().skip(committed).for_each(|output| {
                    // Ensure the transaction is skipped based on the output.
                    assert!(output.writes.is_empty());
                    assert!(output.deltas.is_empty());
                    assert!(output.read_results.is_empty());
                    assert_eq!(output.total_gas, 0);

                    // Implies that materialize_delta_writes was never called, as should
                    // be for skipped transactions.
                    assert_none!(output.materialized_delta_writes.get());
                });
            },
            Err(BlockExecutorError::UserError(idx)) => {
                assert_matches!(&self.status, BaselineStatus::Aborted);
                assert_eq!(*idx, self.read_values.len());
                assert_eq!(*idx, self.resolved_deltas.len());
            },
            Err(BlockExecutorError::ModulePathReadWrite) => unimplemented!("not tested here"),
        }
    }
}
