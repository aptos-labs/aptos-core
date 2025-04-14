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
        default_group_map, deserialize_to_delayed_field_id, raw_metadata, serialize_from_delayed_field_u128, GroupSizeOrMetadata, MockOutput, MockTransaction, ValueType, RESERVED_TAG, STORAGE_AGGREGATOR_VALUE
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
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use std::{collections::{HashMap, BTreeMap}, fmt::Debug, hash::Hash, result::Result, sync::atomic::Ordering};

#[derive(Clone)]
enum BaselineValue {
    GenericWrite(ValueType),
    Aggregator(u128),
    // Expected value and expected version of the delayed field.
    DelayedField(u128, u32),
    // If true, then baseline value (when non-empty), includes txn_idx serialized after
    // STORAGE_AGGREGATOR_VALUE.
    Empty(bool),
}

// TODO: Test delayed field deletions. 
// TODO: instead of the same Error on aggregator overflow / underflow, add a way to tell
// from the error and test it.
// enum AggregatorError {
//    Overflow,
//    Underflow,
// }

impl BaselineValue {
    // Compare to the read results during block execution. When Some is returned, there is a delayed field
    // follow-up checks that must be performed. In this case, the triple is (id, expected_value, version).
    pub(crate) fn assert_read_result<K: Clone + Debug + Eq + Hash>(
        &self, 
        baseline_key: &K, 
        bytes_read: &Option<Vec<u8>>, 
        delayed_field_reads: &mut impl Iterator<Item = (DelayedFieldID, u128, K)>,
        delayed_field_key_to_id_map: &mut HashMap<K, DelayedFieldID>) {
        match (self, bytes_read) {
            (BaselineValue::DelayedField(expected_value, expected_version), Some(bytes)) => {
                verify_delayed_field(
                    bytes,
                    baseline_key,
                    *expected_version,
                    *expected_value,
                    delayed_field_reads,
                    delayed_field_key_to_id_map
                );
            },
            (BaselineValue::DelayedField(_, _), None) => unreachable!("Deletes on delayed fields not yet tested"),
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
            (BaselineValue::Empty(with_txn_idx), Some(bytes)) => {
                assert_eq!(
                    if *with_txn_idx {
                        serialize_from_delayed_field_u128(STORAGE_AGGREGATOR_VALUE, u32::MAX) 
                    } else {
                        serialize(&STORAGE_AGGREGATOR_VALUE).into()
                    }, 
                    *bytes);
            },
            (BaselineValue::Empty(_), None) => (),
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

// Update the GroupReadInfo struct to always set baseline_value (not maybe_baseline_value) and dispatch based on has_delayed_field. Simplify the comparison logic.
#[derive(Debug)]
struct GroupReadInfo<K: Clone> {
    group_key: K,
    baseline_bytes: Option<Bytes>,
    // Set when delayed field is contained.
    maybe_delayed_field: Option<(u128, u32)>,
}

impl<K: Clone + Debug + Eq + Hash> GroupReadInfo<K> {
    // Compute group read results from group_reads and group_world
    fn compute_from_group_reads(
        group_reads: &Result<Vec<(K, u32, bool)>, ()>,
        delayed_field_key_to_id_map: &mut HashMap<K, DelayedFieldID>,
        group_world: &mut HashMap<K, (BTreeMap<u32, Bytes>, u128)>,
    ) -> Vec<GroupReadInfo<K>> {
        group_reads
            .as_ref()
            .unwrap()
            .iter()
            .map(|(group_key, resource_tag, has_delayed_field)| {
                if *has_delayed_field {
                    // Currently delayed fields are tested only with RESERVED_TAG.
                    assert_eq!(*resource_tag, RESERVED_TAG);
                }
                
                let group = group_world
                    .entry(group_key.clone())
                    .or_insert_with(|| (default_group_map(), STORAGE_AGGREGATOR_VALUE));

                let baseline_bytes = group
                    .0
                    .get(resource_tag)
                    .cloned();

                let maybe_delayed_field = has_delayed_field.then(|| {
                    let (id, version) = deserialize_to_delayed_field_id(baseline_bytes.as_ref().unwrap()).unwrap();
                    
                    if version != u32::MAX {
                        // Baseline doesn't have ID exchanged for storage reads. Otherwise,
                        // add ID to key map and assert consistency if already present
                        assert!(
                            delayed_field_key_to_id_map
                                .insert(group_key.clone(), id)
                                .map_or(true, |existing_id| existing_id == id),
                            "Inconsistent delayed field ID mapping"
                        );
                    }

                    (group.1, version)
                });

                GroupReadInfo {
                    group_key: group_key.clone(),
                    baseline_bytes,
                    maybe_delayed_field,
                }
            })
            .collect()
    }
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
pub(crate) struct BaselineOutput<K: Clone + Debug + Eq + Hash> {
    status: BaselineStatus,
    read_values: Vec<Result<Vec<(K, BaselineValue)>, ()>>,
    resolved_deltas: Vec<Result<HashMap<K, u128>, ()>>,
    group_reads: Vec<Result<Vec<(K, u32, bool)>, ()>>,
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
                    delayed_fields_or_aggregator_v1,
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
                    let base_value = BaselineValue::Empty(*delayed_fields_or_aggregator_v1);
                    match incarnation_behaviors[last_incarnation]
                        .deltas
                        .iter()
                        .map(|(k, delta)| {
                            let base = match current_world
                                .entry(k.clone())
                                .or_insert(base_value.clone())
                            {
                                // TODO(BlockSTMv2): add implicit reads (exchange) to deltas
                                // for delayed fields.
                                BaselineValue::DelayedField(value, _) => *value,
                                // Get base value from the latest write.
                                BaselineValue::GenericWrite(w_value) => w_value
                                    .as_u128()
                                    .expect("Delta to a non-existent aggregator")
                                    .expect("Must deserialize the aggregator base value"),
                                // Get base value from latest resolved aggregator value.
                                BaselineValue::Aggregator(value) => *value,
                                // Storage always gets resolved to a default constant.
                                BaselineValue::Empty(_) => STORAGE_AGGREGATOR_VALUE,
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
                                .map(|(k, has_deltas)| {
                                    let baseline_value = current_world
                                        .entry(k.clone())
                                        .or_insert(base_value.clone());
                                    
                                    let value = if *delayed_fields_or_aggregator_v1 && *has_deltas {
                                        match baseline_value {
                                            BaselineValue::DelayedField(_, _) => {
                                                baseline_value.clone()
                                            }
                                            BaselineValue::Empty(_) => {
                                                BaselineValue::DelayedField(STORAGE_AGGREGATOR_VALUE, u32::MAX)
                                            }
                                            BaselineValue::GenericWrite(_) => unreachable!("Delayed field testing should not have generic writes"),
                                            BaselineValue::Aggregator(_) => unreachable!("Delayed field testing should not have aggregators"),
                                        }
                                    } else {
                                        baseline_value.clone()
                                    };
                                    (k.clone(), value)
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
                                    // TODO(BlockSTMv2).
                                    // During delayed field testing, resolved deltas will be tested against 
                                    // the replaced writes in the output.
                                    if !*delayed_fields_or_aggregator_v1 {
                                        // In this case transaction did not fail due to delta application
                                        // errors, and thus we should update written_ and resolved_ worlds.
                                        current_world.insert(k.clone(), BaselineValue::Aggregator(v));
                                    }
                                    (k, v)
                                })
                                .collect()));
                            // TODO(BlockSTMv2): handle exchanged delayed field outputs (match up counts too).

                            for (k, v, has_delta) in incarnation_behaviors[last_incarnation].writes.iter() {
                                // Here we don't know IDs but we know values, so use the GenericWrite to store the 
                                // expected value, and compare that against the actual read on delayed field that was
                                // performed during committed execution.
                                current_world.insert(k.clone(), 
                                if *delayed_fields_or_aggregator_v1 && *has_delta {
                                    BaselineValue::DelayedField(
                                        match current_world.get(k) {
                                            Some(BaselineValue::DelayedField(value, _)) => *value,
                                            Some(BaselineValue::GenericWrite(_)) => unreachable!("Delayed field testing should not have generic writes"),
                                            Some(BaselineValue::Aggregator(_)) => unreachable!("Delayed field testing should not have aggregators"),
                                            None | Some(BaselineValue::Empty(_)) => STORAGE_AGGREGATOR_VALUE,
                                        }, 
                                        txn_idx as u32)
                                } else {
                                    BaselineValue::GenericWrite(v.clone())
                                });
                            }
                            for module_write in
                                incarnation_behaviors[last_incarnation].module_writes.iter()
                            {
                                module_world
                                    .insert(module_write.module_id().clone(), txn_idx as TxnIndex);
                            }

                            // For groups, we map so that has_deltas will imply that delayed field testing 
                            // is in progress (since AggregatorV1 may not reside in a group).
                            group_reads.push(Ok(incarnation_behaviors[last_incarnation]
                                .group_reads
                                .iter()
                                .map(|(k, tag, has_delayed_field)| (k.clone(), *tag, *has_delayed_field && *delayed_fields_or_aggregator_v1))
                                .collect()));

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
                let mut delayed_field_key_to_id_map = HashMap::new();
                
                // Compute group read results with all necessary information using the new method
                let group_read_infos = GroupReadInfo::compute_from_group_reads(group_reads, &mut delayed_field_key_to_id_map, &mut group_world);
                
                // Test group read results.
                let read_len = reads.as_ref().unwrap().len();
                
                assert!(!output.skipped, "Error at txn {}: {:?}", idx, output.maybe_error_msg);

                // Test normal reads, then group reads.
                let mut delayed_field_reads = output.delayed_field_reads.clone().into_iter();
                izip!(
                    reads
                        .as_ref()
                        .expect("Aggregator failures not yet tested")
                        .iter(),
                    output.read_results.iter().take(read_len)
                )
                .for_each(|((baseline_key, baseline_read), result_read)| {
                    baseline_read.assert_read_result(baseline_key, result_read, &mut delayed_field_reads, &mut delayed_field_key_to_id_map);
                });

                assert_eq!(
                    group_read_infos.len(),
                    output.read_results.len() - read_len
                );
                
                // Use the precomputed group_read_infos instead of zipping with group_reads again
                izip!(
                    group_read_infos.into_iter(),
                    output.read_results.iter().skip(read_len)
                )
                .for_each(|(group_info, result_group_read)| {
                    let result_bytes = result_group_read.clone().map(Into::<Bytes>::into);
                    
                    // Size check for all cases
                    if let (Some(result), Some(baseline)) = (result_group_read.as_ref(), group_info.baseline_bytes.as_ref()) {
                        assert_eq!(result.len(), baseline.len(), "Length mismatch for value");
                    }
                    
                    match group_info.maybe_delayed_field {
                        Some((expected_value, expected_version)) => {
                            // Extract bytes from the result and verify delayed field invariants.
                            let result_bytes = result_bytes.expect("Must have a result for verification");
                            // Verify delayed field with all required parameters
                            verify_delayed_field(
                                result_bytes.as_ref(),
                                &group_info.group_key,
                                expected_version,
                                expected_value,
                                &mut delayed_field_reads,
                                &mut delayed_field_key_to_id_map,
                            );
                        }
                        None => {
                            // Case 2: This is a regular value - just compare bytes directly
                            assert_eq!(result_bytes, group_info.baseline_bytes, 
                                     "Result bytes don't match baseline value for regular field");
                        }
                    }
                });

                // Ensure all delayed field reads have been processed
                assert_none!(delayed_field_reads.next());

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
                    let group_map = group_world
                        .entry(group_key.clone())
                        .or_insert_with(|| (default_group_map(), STORAGE_AGGREGATOR_VALUE));

                    match size_or_metadata {
                        GroupSizeOrMetadata::Size(size) => {
                            let baseline_size =
                                group_size_as_sum(group_map.0.iter().map(|(t, v)| (t, v.len())))
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

                // Update group world.
                for (group_key, v, group_size, updates) in output.group_writes.iter() {
                    group_metadata.insert(group_key.clone(), v.as_state_value_metadata());

                    let group_map = &mut group_world
                        .entry(group_key.clone())
                        .or_insert_with(|| (default_group_map(), STORAGE_AGGREGATOR_VALUE)).0;
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

// Simplify verify_delayed_field by removing Option wrappers
fn verify_delayed_field<K: Clone + Debug + PartialEq + Eq + Hash>(
    bytes: &[u8],
    baseline_key: &K,
    expected_version: u32,
    expected_value: u128,
    delayed_field_reads: &mut impl Iterator<Item = (DelayedFieldID, u128, K)>,
    delayed_field_key_to_id_map: &mut HashMap<K, DelayedFieldID>,
) {
    // Deserialize the ID and version from bytes
    let (id, version) = deserialize_to_delayed_field_id(bytes)
        .expect("Must deserialize delayed field tuple");
    
    // Verify the version matches
    assert_eq!(expected_version, version, "Version mismatch for delayed field");

    // Get the corresponding delayed field read
    let (delayed_id, value, key) = delayed_field_reads
        .next()
        .expect("Must have a delayed field read");
    
    // Verify the ID, key, and value match
    assert_eq!(id, delayed_id, "Delayed field ID mismatch");
    assert_eq!(*baseline_key, key, "Delayed field key mismatch");
    assert_eq!(expected_value, value, "Value mismatch for delayed field");

    // Add ID to key map and assert consistency if already present
    assert!(
        delayed_field_key_to_id_map
            .insert(baseline_key.clone(), id)
            .map_or(true, |existing_id| existing_id == id),
        "Inconsistent delayed field ID mapping"
    );
}
