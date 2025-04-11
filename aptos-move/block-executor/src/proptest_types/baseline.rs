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
    errors::{BlockExecutionError, BlockExecutionResult},
    proptest_types::types::{
        raw_metadata, GroupSizeOrMetadata, MockOutput, MockTransaction, ValueType, RESERVED_TAG,
        STORAGE_AGGREGATOR_VALUE,
    },
};
use aptos_aggregator::delta_change_set::serialize;
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::{
    contract_event::TransactionEvent, state_store::state_value::StateValueMetadata,
    transaction::BlockOutput, write_set::TransactionWrite,
};
use aptos_vm_types::resource_group_adapter::group_size_as_sum;
use bytes::Bytes;
use claims::{assert_gt, assert_matches, assert_none, assert_some, assert_some_eq};
use itertools::izip;
use move_core_types::language_storage::ModuleId;
use std::{collections::HashMap, fmt::Debug, hash::Hash, result::Result, sync::atomic::Ordering};

// TODO: extend to derived values, and code.
#[derive(Clone)]
enum BaselineValue {
    GenericWrite(ValueType),
    Aggregator(u128),
    Empty,
}

// TODO: instead of the same Error on aggregator overflow / underflow, add a way to tell
// from the error and test it.
// enum AggregatorError {
//    Overflow,
//    Underflow,
// }

impl BaselineValue {
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
pub(crate) struct BaselineOutput<K> {
    status: BaselineStatus,
    read_values: Vec<Result<Vec<BaselineValue>, ()>>,
    resolved_deltas: Vec<Result<HashMap<K, u128>, ()>>,
    group_reads: Vec<Result<Vec<(K, u32)>, ()>>,
    module_reads: Vec<Result<Vec<Option<TxnIndex>>, ()>>,
}

impl<K: Debug + Hash + Clone + Eq> BaselineOutput<K> {
    /// Must be invoked after parallel execution to have incarnation information set and
    /// work with dynamic read/writes.
    pub(crate) fn generate<E: Debug + Clone + TransactionEvent>(
        txns: &[MockTransaction<K, E>],
        maybe_block_gas_limit: Option<u64>,
    ) -> Self {
        let mut current_world = HashMap::<K, BaselineValue>::new();
        let mut module_world = HashMap::<ModuleId, TxnIndex>::new();
        let mut accumulated_gas = 0;

        let mut status = BaselineStatus::Success;
        let mut read_values = vec![];
        let mut resolved_deltas = vec![];
        let mut group_reads = vec![];
        let mut module_reads = vec![];

        for (txn_idx, txn) in txns.iter().enumerate() {
            match txn {
                MockTransaction::Abort => {
                    status = BaselineStatus::Aborted;
                    break;
                },
                MockTransaction::SkipRest(gas) => {
                    // In executor, SkipRest skips from the next index. Test assumes it's an empty
                    // transaction, so create a successful empty reads and deltas.
                    read_values.push(Ok(vec![]));
                    resolved_deltas.push(Ok(HashMap::new()));

                    // gas in SkipRest is used for unit tests for now (can generalize when needed).
                    assert_eq!(*gas, 0);

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
                    let incarnation_counter = incarnation_counter.load(Ordering::SeqCst);
                    // Mock execute_transaction call always increments the incarnation counter.
                    assert_gt!(
                        incarnation_counter,
                        0,
                        "Mock execution of txn {txn_idx} never incremented incarnation"
                    );
                    let last_incarnation = (incarnation_counter - 1) % incarnation_behaviors.len();
                    match incarnation_behaviors[last_incarnation]
                        .deltas
                        .iter()
                        .map(|(k, delta)| {
                            let base = match current_world
                                .entry(k.clone())
                                .or_insert(BaselineValue::Empty)
                            {
                                // Get base value from the latest write.
                                BaselineValue::GenericWrite(w_value) => w_value
                                    .as_u128()
                                    .expect("Delta to a non-existent aggregator")
                                    .expect("Must deserialize the aggregator base value"),
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
                                        .entry(k.0.clone())
                                        .or_insert(BaselineValue::Empty)
                                        .clone()
                                })
                                .collect()));

                            module_reads.push(Ok(incarnation_behaviors[last_incarnation]
                                .module_reads
                                .iter()
                                .map(|module_id| module_world.get(module_id).cloned())
                                .collect()));

                            resolved_deltas.push(Ok(txn_resolved_deltas
                                .into_iter()
                                .map(|(k, v)| {
                                    // In this case transaction did not fail due to delta application
                                    // errors, and thus we should update written_ and resolved_ worlds.
                                    current_world.insert(k.clone(), BaselineValue::Aggregator(v));
                                    (k, v)
                                })
                                .collect()));

                            // We ensure that the latest state is always reflected in exactly one of
                            // the hashmaps, by possibly removing an element from the other Hashmap.
                            for (k, v, _) in incarnation_behaviors[last_incarnation].writes.iter() {
                                current_world
                                    .insert(k.clone(), BaselineValue::GenericWrite(v.clone()));
                            }
                            for module_write in
                                incarnation_behaviors[last_incarnation].module_writes.iter()
                            {
                                module_world
                                    .insert(module_write.module_id().clone(), txn_idx as TxnIndex);
                            }

                            group_reads.push(Ok(incarnation_behaviors[last_incarnation]
                                .group_reads
                                .clone()));

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
                            group_reads.push(Err(()));
                            module_reads.push(Err(()));
                        },
                    }
                },
                MockTransaction::InterruptRequested => unreachable!("Not tested with outputs"),
            }
        }

        Self {
            status,
            read_values,
            resolved_deltas,
            group_reads,
            module_reads,
        }
    }

    fn assert_success<E: Debug>(&self, block_output: &BlockOutput<MockOutput<K, E>>) {
        let base_map: HashMap<u32, Bytes> = HashMap::from([(RESERVED_TAG, vec![0].into())]);
        let mut group_world = HashMap::new();
        let mut group_metadata: HashMap<K, Option<StateValueMetadata>> = HashMap::new();

        let results = block_output.get_transaction_outputs_forced();
        let committed = self.read_values.len();
        assert_eq!(self.resolved_deltas.len(), committed);

        // Check read values & delta writes.
        izip!(
            (0..committed),
            results.iter().take(committed),
            self.read_values.iter(),
            self.resolved_deltas.iter(),
            self.group_reads.iter(),
            self.module_reads.iter(),
        )
        .for_each(
            |(idx, output, reads, resolved_deltas, group_reads, module_reads)| {
                // Compute group read results.
                let group_read_results: Vec<Option<Bytes>> = group_reads
                    .as_ref()
                    .unwrap()
                    .iter()
                    .map(|(group_key, resource_tag)| {
                        let group_map = group_world.entry(group_key).or_insert(base_map.clone());

                        group_map.get(resource_tag).cloned()
                    })
                    .collect();
                // Test group read results.
                let read_len = reads.as_ref().unwrap().len();

                assert_eq!(
                    group_read_results.len(),
                    output.read_results.len() - read_len
                );
                izip!(
                    output.read_results.iter().skip(read_len),
                    group_read_results.into_iter()
                )
                .for_each(|(result_group_read, baseline_group_read)| {
                    assert_eq!(
                        result_group_read.clone().map(Into::<Bytes>::into),
                        baseline_group_read
                    );
                });

                izip!(
                    output.module_read_results.iter(),
                    module_reads
                        .as_ref()
                        .expect("No delta failures")
                        .into_iter()
                )
                .for_each(|(module_read, baseline_module_read)| {
                    assert_eq!(
                        module_read
                            .as_ref()
                            .map(|m| m.creation_time_usecs())
                            .unwrap(),
                        baseline_module_read
                            .map(|i| i as u64)
                            .unwrap_or(u32::MAX as u64),
                        "for txn_idx = {}",
                        idx
                    );
                });

                for (group_key, size_or_metadata) in output.read_group_size_or_metadata.iter() {
                    let group_map = group_world.entry(group_key).or_insert(base_map.clone());

                    match size_or_metadata {
                        GroupSizeOrMetadata::Size(size) => {
                            let baseline_size =
                                group_size_as_sum(group_map.iter().map(|(t, v)| (t, v.len())))
                                    .unwrap()
                                    .get();

                            assert_eq!(
                                baseline_size, *size,
                                "ERR: idx = {} group_key {:?}, baseline size {} != output_size {}",
                                idx, group_key, baseline_size, size
                            );
                        },
                        GroupSizeOrMetadata::Metadata(metadata) => {
                            if !group_metadata.contains_key(group_key) {
                                assert_eq!(
                                    *metadata,
                                    Some(raw_metadata(5)) /* default metadata */
                                );
                            } else {
                                let baseline_metadata =
                                    group_metadata.get(group_key).cloned().flatten();
                                assert_eq!(*metadata, baseline_metadata);
                            }
                        },
                    }
                }

                // Test normal reads.
                izip!(
                    reads
                        .as_ref()
                        .expect("Aggregator failures not yet tested")
                        .iter(),
                    output.read_results.iter().take(read_len)
                )
                .for_each(|(baseline_read, result_read)| {
                    baseline_read.assert_read_result(result_read)
                });

                // Update group world.
                for (group_key, v, group_size, updates) in output.group_writes.iter() {
                    group_metadata.insert(group_key.clone(), v.as_state_value_metadata());

                    let group_map = group_world.entry(group_key).or_insert(base_map.clone());
                    for (tag, v) in updates {
                        if v.is_deletion() {
                            assert_some!(group_map.remove(tag));
                        } else {
                            let existed = group_map
                                .insert(*tag, v.extract_raw_bytes().unwrap())
                                .is_some();
                            assert_eq!(existed, v.is_modification());
                        }
                    }
                    let computed_size =
                        group_size_as_sum(group_map.iter().map(|(t, v)| (t, v.len()))).unwrap();
                    assert_eq!(computed_size, *group_size);
                }

                // Test recorded finalized group writes: it should contain the whole group, and
                // as such, correspond to the contents of the group_world.
                // TODO: figure out what can still be tested here, e.g. RESERVED_TAG
                // let groups_tested =
                //     (output.group_writes.len() + group_reads.as_ref().unwrap().len()) > 0;
                // for (group_key, _, finalized_updates) in output.recorded_groups.get().unwrap() {
                //     let baseline_group_map =
                //         group_world.entry(group_key).or_insert(base_map.clone());
                //     assert_eq!(finalized_updates.len(), baseline_group_map.len());
                //     if groups_tested {
                //         // RESERVED_TAG should always be contained.
                //         assert_ge!(finalized_updates.len(), 1);

                //         for (tag, v) in finalized_updates.iter() {
                //             assert_eq!(
                //                 v.bytes().unwrap(),
                //                 baseline_group_map.get(tag).unwrap(),
                //             );
                //         }
                //     }
                // }

                let baseline_deltas = resolved_deltas
                    .as_ref()
                    .expect("Aggregator failures not yet tested");
                output
                    .materialized_delta_writes
                    .get()
                    .expect("Delta writes must be set")
                    .iter()
                    .for_each(|(k, result_delta_write)| {
                        assert_eq!(
                            *baseline_deltas.get(k).expect("Baseline must contain delta"),
                            result_delta_write
                                .as_u128()
                                .expect("Baseline must contain delta")
                                .expect("Must deserialize aggregator write value")
                        );
                    });
            },
        );

        results.iter().skip(committed).for_each(|output| {
            // Ensure the transaction is skipped based on the output.
            assert!(output.skipped);

            // Implies that materialize_delta_writes was never called, as should
            // be for skipped transactions.
            assert_none!(output.materialized_delta_writes.get());
        });
    }

    // Used for testing, hence the function asserts the correctness conditions within
    // itself to be easily traceable in case of an error.
    pub(crate) fn assert_output<E: Debug>(
        &self,
        results: &BlockExecutionResult<BlockOutput<MockOutput<K, E>>, usize>,
    ) {
        match results {
            Ok(block_output) => {
                self.assert_success(block_output);
            },
            Err(BlockExecutionError::FatalVMError(idx)) => {
                assert_matches!(&self.status, BaselineStatus::Aborted);
                assert_eq!(*idx, self.read_values.len());
                assert_eq!(*idx, self.resolved_deltas.len());
            },
            Err(BlockExecutionError::FatalBlockExecutorError(e)) => {
                unimplemented!("not tested here FallbackToSequential({:?})", e);
            },
        }
    }

    pub(crate) fn assert_parallel_output<E: Debug>(
        &self,
        results: &Result<BlockOutput<MockOutput<K, E>>, ()>,
    ) {
        match results {
            Ok(block_output) => {
                self.assert_success(block_output);
            },
            Err(()) => {
                // Parallel execution currently returns an arbitrary error to fallback.
                // TODO: adjust the logic to be able to test better.
            },
        }
    }
}
