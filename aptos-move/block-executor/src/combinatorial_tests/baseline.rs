// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    combinatorial_tests::{
        mock_executor::MockOutput,
        types::{
            default_group_map, raw_metadata, DeltaTestKind, GroupSizeOrMetadata, MockIncarnation,
            MockTransaction, ValueType, RESERVED_TAG, STORAGE_AGGREGATOR_VALUE,
        },
    },
    errors::{BlockExecutionError, BlockExecutionResult},
    types::delayed_field_mock_serialization::{
        deserialize_to_delayed_field_id, deserialize_to_delayed_field_u128,
        serialize_from_delayed_field_u128,
    },
};
use aptos_aggregator::delta_change_set::{serialize, DeltaOp};
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::{
    contract_event::TransactionEvent, executable::ModulePath,
    state_store::state_value::StateValueMetadata, transaction::BlockOutput,
    write_set::TransactionWrite,
};
use aptos_vm_types::{
    module_write_set::ModuleWrite, resolver::ResourceGroupSize,
    resource_group_adapter::group_size_as_sum,
};
use bytes::Bytes;
use claims::{assert_gt, assert_matches, assert_none, assert_some, assert_some_eq};
use move_core_types::language_storage::ModuleId;
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap},
    fmt::Debug,
    hash::Hash,
    result::Result,
    sync::atomic::Ordering,
};

/// This file implements the baseline evaluation, performed sequentially, the output
/// of which is used to test the results of the block executor. The baseline must be
/// evaluated after the block executor has completed, as the transaction type used
/// for testing tracks the incarnation number, which is used to emulate dynamic behavior.
/// Dynamic behavior means that when a transaction is re-executed, it might read
/// different values and end up with a completely different behavior (be it read set,
/// write set, or executed code). In the tests, behavior changes based on the incarnation
/// number, and hence it is crucial for the baseline to know the final incarnation number
/// of each transaction of the tested block executor execution.
///
/// The BaselineOutputBuilder is used to build the BaselineOutput. One difference between
/// resources and groups is that the resources are processed while building BaselineOutput,
/// while groups are processed while asserting the output. We may want to reconsider this
/// in the future, but for now it provides two different ways of testing similar invariants,
/// such as the handling of delayed fields and their IDs / values.
///
/// TODO: Not yet tested or supported cases in the testing framework:
/// - Delayed field deletions.
/// - Writes & delta for the same resource.
/// - Multiple delta applications, including failures.
/// - Empty groups and group deletions.
/// - Gas limit with sequential execution.

#[derive(Clone)]
enum BaselineValue {
    GenericWrite(ValueType),
    Aggregator(u128),
    // Expected value and expected version of the delayed field.
    DelayedField(u128, u32),
    // If true, then baseline value (when non-empty), includes txn_idx
    // serialized after STORAGE_AGGREGATOR_VALUE. This is used to test
    // the delayed fields, as unlike AggregatorV1, the delayed fields
    // exist within a larger resource, and the writer's index (for storage
    // version max u32) is stored for testing in the same mock resource.
    Empty(DeltaTestKind),
}

// The status of the baseline execution.
#[derive(Debug)]
enum BaselineStatus {
    Success,
    Aborted,
    SkipRest,
    GasLimitExceeded,
}

// TODO: Update the GroupReadInfo struct to always set baseline value
// and simplify the comparison logic.
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
        group_world: &mut HashMap<K, BTreeMap<u32, Bytes>>,
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
                    .or_insert_with(default_group_map);
                let baseline_bytes = group.get(resource_tag).cloned();
                let maybe_delayed_field = has_delayed_field.then(|| {
                    deserialize_to_delayed_field_u128(baseline_bytes.as_ref().unwrap()).unwrap()
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
    group_deltas: Vec<Result<Vec<(K, DeltaOp)>, ()>>,
    module_reads: Vec<Result<Vec<Option<TxnIndex>>, ()>>,
    delayed_field_key_to_id_map: RefCell<HashMap<K, DelayedFieldID>>,
}

/// Builder for BaselineOutput to simplify construction
pub(crate) struct BaselineOutputBuilder<K: Clone + Debug + Eq + Hash> {
    status: BaselineStatus,
    read_values: Vec<Result<Vec<(K, BaselineValue)>, ()>>,
    resolved_deltas: Vec<Result<HashMap<K, u128>, ()>>,
    group_reads: Vec<Result<Vec<(K, u32, bool)>, ()>>,
    group_deltas: Vec<Result<Vec<(K, DeltaOp)>, ()>>,
    module_reads: Vec<Result<Vec<Option<TxnIndex>>, ()>>,
    current_world: HashMap<K, BaselineValue>,
    module_world: HashMap<ModuleId, TxnIndex>,
    txn_read_write_resolved_deltas: HashMap<K, u128>,
}

impl<K: Clone + Debug + Eq + Hash> BaselineOutputBuilder<K> {
    /// Create a new builder
    pub(crate) fn new() -> Self {
        Self {
            status: BaselineStatus::Success,
            read_values: vec![],
            resolved_deltas: vec![],
            group_reads: vec![],
            group_deltas: vec![],
            module_reads: vec![],
            current_world: HashMap::new(),
            module_world: HashMap::new(),
            txn_read_write_resolved_deltas: HashMap::new(),
        }
    }

    /// Build the final BaselineOutput
    pub(crate) fn build(self) -> BaselineOutput<K> {
        BaselineOutput {
            status: self.status,
            read_values: self.read_values,
            resolved_deltas: self.resolved_deltas,
            group_reads: self.group_reads,
            group_deltas: self.group_deltas,
            module_reads: self.module_reads,
            delayed_field_key_to_id_map: RefCell::new(HashMap::new()),
        }
    }

    /// Set the execution status
    fn with_status(&mut self, status: BaselineStatus) -> &mut Self {
        self.status = status;
        self
    }

    /// Add an empty successful transaction (for SkipRest)
    fn with_empty_successful_transaction(&mut self) -> &mut Self {
        self.read_values.push(Ok(vec![]));
        self.resolved_deltas.push(Ok(HashMap::new()));
        self
    }

    /// Mark the transaction as failed by pushing errors to all result vectors
    fn with_transaction_failed(&mut self) -> &mut Self {
        self.read_values.push(Err(()));
        self.resolved_deltas.push(Err(()));
        self.group_reads.push(Err(()));
        self.group_deltas.push(Err(()));
        self.module_reads.push(Err(()));
        self
    }

    fn with_group_deltas(&mut self, deltas: Vec<(K, DeltaOp)>) -> &mut Self {
        self.group_deltas.push(Ok(deltas));
        self
    }

    fn with_module_reads(&mut self, module_ids: &[ModuleId]) -> &mut Self {
        let result = Ok(module_ids
            .iter()
            .map(|module_id| self.module_world.get(module_id).cloned())
            .collect());
        self.module_reads.push(result);
        self
    }

    fn with_resource_reads(
        &mut self,
        reads: &[(K, bool)],
        delta_test_kind: DeltaTestKind,
    ) -> &mut Self {
        let base_value = BaselineValue::Empty(delta_test_kind);

        let result = Ok(reads
            .iter()
            .map(|(k, has_deltas)| {
                let baseline_value = self
                    .current_world
                    .entry(k.clone())
                    .or_insert(base_value.clone());

                let value = if delta_test_kind == DeltaTestKind::DelayedFields && *has_deltas {
                    match baseline_value {
                        BaselineValue::DelayedField(v, _) => {
                            self.txn_read_write_resolved_deltas.insert(k.clone(), *v);
                            baseline_value.clone()
                        },
                        BaselineValue::Empty(delta_test_kind) => {
                            assert_eq!(*delta_test_kind, DeltaTestKind::DelayedFields);
                            self.txn_read_write_resolved_deltas
                                .insert(k.clone(), STORAGE_AGGREGATOR_VALUE);
                            BaselineValue::DelayedField(STORAGE_AGGREGATOR_VALUE, u32::MAX)
                        },
                        BaselineValue::GenericWrite(_) => {
                            unreachable!("Delayed field testing should not have generic writes")
                        },
                        BaselineValue::Aggregator(_) => {
                            unreachable!("Delayed field testing should not have aggregators")
                        },
                    }
                } else {
                    baseline_value.clone()
                };
                (k.clone(), value)
            })
            .collect());

        self.read_values.push(result);
        self
    }

    fn with_resource_deltas(
        &mut self,
        resolved_deltas: Vec<(K, u128, Option<u32>)>,
        delta_test_kind: DeltaTestKind,
    ) -> &mut Self {
        let mut result: HashMap<K, u128> = resolved_deltas
            .into_iter()
            .map(|(k, v, delayed_field_last_write_version)| {
                match delta_test_kind {
                    DeltaTestKind::DelayedFields => {
                        self.current_world.insert(
                            k.clone(),
                            BaselineValue::DelayedField(
                                v,
                                delayed_field_last_write_version
                                    .expect("Must be set by delta pre-processing"),
                            ),
                        );
                    },
                    DeltaTestKind::AggregatorV1 => {
                        // In this case transaction did not fail due to delta application
                        // errors, and thus we should update written_ and resolved_ worlds.
                        self.current_world
                            .insert(k.clone(), BaselineValue::Aggregator(v));
                    },
                    DeltaTestKind::None => {
                        unreachable!("None delta test kind should not be used for resource deltas");
                    },
                }
                (k, v)
            })
            .collect();

        for (k, v) in std::mem::take(&mut self.txn_read_write_resolved_deltas) {
            result.entry(k).or_insert(v);
        }

        self.resolved_deltas.push(Ok(result));
        self
    }

    fn with_group_reads(
        &mut self,
        group_reads: &[(K, u32, bool)],
        delta_test_kind: DeltaTestKind,
    ) -> &mut Self {
        let result = Ok(group_reads
            .iter()
            .map(|(k, tag, has_delayed_field)| {
                if *has_delayed_field {
                    assert_eq!(*tag, RESERVED_TAG);
                    assert_eq!(delta_test_kind, DeltaTestKind::DelayedFields);
                }

                (k.clone(), *tag, *has_delayed_field)
            })
            .collect());
        self.group_reads.push(result);
        self
    }

    fn with_module_writes(
        &mut self,
        module_writes: &[ModuleWrite<ValueType>],
        txn_idx: TxnIndex,
    ) -> &mut Self {
        for module_write in module_writes {
            self.module_world
                .insert(module_write.module_id().clone(), txn_idx);
        }
        self
    }

    fn with_resource_writes(
        &mut self,
        writes: &[(K, ValueType, bool)],
        delta_test_kind: DeltaTestKind,
        txn_idx: usize,
    ) -> &mut Self {
        for (k, v, has_delta) in writes {
            // Here we don't know IDs but we know values, so use the GenericWrite to store the
            // expected value, and compare that against the actual read on delayed field that was
            // performed during committed execution.
            self.current_world.insert(
                k.clone(),
                if delta_test_kind == DeltaTestKind::DelayedFields && *has_delta {
                    BaselineValue::DelayedField(
                        match self.current_world.get(k) {
                            Some(BaselineValue::DelayedField(value, _)) => {
                                self.txn_read_write_resolved_deltas
                                    .insert(k.clone(), *value);
                                *value
                            },
                            Some(BaselineValue::GenericWrite(_)) => {
                                unreachable!("Delayed field testing should not have generic writes")
                            },
                            Some(BaselineValue::Aggregator(_)) => {
                                unreachable!("Delayed field testing should not have aggregators")
                            },
                            None | Some(BaselineValue::Empty(_)) => {
                                self.txn_read_write_resolved_deltas
                                    .insert(k.clone(), STORAGE_AGGREGATOR_VALUE);
                                STORAGE_AGGREGATOR_VALUE
                            },
                        },
                        txn_idx as u32,
                    )
                } else {
                    BaselineValue::GenericWrite(v.clone())
                },
            );
        }
        self
    }

    /// Process a single delta and return the appropriate result.
    ///
    /// Returns an optional resource delta, if None, the caller should
    /// mark the transaction as failed.
    fn process_delta(
        &mut self,
        key: &K,
        delta: &DeltaOp,
        delta_test_kind: DeltaTestKind,
    ) -> Option<(K, u128, Option<u32>)> {
        let base_value = BaselineValue::Empty(delta_test_kind);

        // Delayed field last write version is used for delayed field testing only, making
        // sure the writer index in the read results are compared against the correct write.
        let (base, delayed_field_last_write_version) =
            match self.current_world.entry(key.clone()).or_insert(base_value) {
                BaselineValue::DelayedField(value, last_write_version) => {
                    (*value, Some(*last_write_version))
                },
                // Get base value from the latest write.
                BaselineValue::GenericWrite(w_value) => {
                    if delta_test_kind == DeltaTestKind::DelayedFields {
                        let (value, last_write_version) = deserialize_to_delayed_field_u128(
                            &w_value
                                .extract_raw_bytes()
                                .expect("Deleted delayed fields not supported"),
                        )
                        .expect("Must deserialize the delayed field base value");
                        (value, Some(last_write_version))
                    } else {
                        (
                            w_value
                                .as_u128()
                                .expect("Delta to a non-existent aggregator")
                                .expect("Must deserialize the aggregator base value"),
                            None,
                        )
                    }
                },
                // Get base value from latest resolved aggregator value.
                BaselineValue::Aggregator(value) => (*value, None),
                // Storage always gets resolved to a default constant.
                BaselineValue::Empty(delta_test_kind) => (
                    STORAGE_AGGREGATOR_VALUE,
                    (*delta_test_kind == DeltaTestKind::DelayedFields).then_some(u32::MAX),
                ),
            };

        match delta.apply_to(base) {
            Err(_) => {
                // Transaction does not take effect and we record delta application failure.
                None
            },
            Ok(resolved_value) => {
                // Transaction succeeded, return the resolved delta
                Some((
                    key.clone(),
                    resolved_value,
                    delayed_field_last_write_version,
                ))
            },
        }
    }

    /// Process all deltas for a transaction and handle failures internally
    ///
    /// Returns (success, group_deltas, resource_deltas)
    /// If success is false, the transaction failed and the deltas should not be used
    fn process_transaction_deltas(
        &mut self,
        deltas: &[(K, DeltaOp, Option<u32>)],
        delta_test_kind: DeltaTestKind,
    ) -> (bool, Vec<(K, DeltaOp)>, Vec<(K, u128, Option<u32>)>) {
        let mut group_deltas = Vec::new();
        let mut resource_deltas = Vec::new();

        for (k, delta, maybe_tag) in deltas {
            if let Some(tag) = maybe_tag {
                assert_eq!(*tag, RESERVED_TAG);
                // This is a group delta
                group_deltas.push((k.clone(), *delta));
            } else {
                match self.process_delta(k, delta, delta_test_kind) {
                    Some(rd) => resource_deltas.push(rd),
                    None => {
                        self.with_transaction_failed();
                        return (false, Vec::new(), Vec::new());
                    },
                }
            }
        }

        (true, group_deltas, resource_deltas)
    }

    /// Process a complete transaction
    ///
    /// Returns whether the gas limit was exceeded
    fn process_transaction<E: Debug + Clone + TransactionEvent>(
        &mut self,
        behavior: &MockIncarnation<K, E>,
        delta_test_kind: DeltaTestKind,
        txn_idx: usize,
        accumulated_gas: &mut u64,
        maybe_block_gas_limit: Option<u64>,
    ) -> bool {
        // Process all deltas first
        let (success, group_deltas, resource_deltas) =
            self.process_transaction_deltas(&behavior.deltas, delta_test_kind);

        if !success {
            return false; // Gas limit not exceeded, transaction failed
        }

        // All remaining operations can be chained since the transaction is known to succeed
        self.with_resource_reads(&behavior.resource_reads, delta_test_kind)
            .with_module_reads(&behavior.module_reads)
            .with_group_reads(&behavior.group_reads, delta_test_kind)
            .with_group_deltas(group_deltas)
            .with_resource_writes(&behavior.resource_writes, delta_test_kind, txn_idx)
            .with_resource_deltas(resource_deltas, delta_test_kind)
            .with_module_writes(&behavior.module_writes, txn_idx as TxnIndex);

        // Apply gas
        *accumulated_gas += behavior.gas;

        // Check if gas limit exceeded
        let gas_limit_exceeded = maybe_block_gas_limit
            .map(|limit| *accumulated_gas >= limit)
            .unwrap_or(false);

        if gas_limit_exceeded {
            self.with_status(BaselineStatus::GasLimitExceeded);
        }

        gas_limit_exceeded
    }
}

impl<K> BaselineOutput<K>
where
    K: Debug + Hash + Clone + Ord + Send + Sync + ModulePath + 'static,
{
    /// Must be invoked after parallel execution to have incarnation information set and
    /// work with dynamic read/writes.
    pub(crate) fn generate<E: Debug + Clone + TransactionEvent>(
        txns: &[MockTransaction<K, E>],
        maybe_block_gas_limit: Option<u64>,
    ) -> Self {
        let mut builder = BaselineOutputBuilder::new();
        let mut accumulated_gas = 0;

        for (txn_idx, txn) in txns.iter().enumerate() {
            match txn {
                MockTransaction::Abort => {
                    builder.with_status(BaselineStatus::Aborted);
                    break;
                },
                MockTransaction::SkipRest(gas) => {
                    // In executor, SkipRest skips from the next index. Test assumes it's an empty
                    // transaction, so create a successful empty reads and deltas.
                    builder.with_empty_successful_transaction();

                    // gas in SkipRest is used for unit tests for now (can generalize when needed).
                    assert_eq!(*gas, 0);

                    builder.with_status(BaselineStatus::SkipRest);
                    break;
                },
                MockTransaction::Write {
                    incarnation_counter,
                    incarnation_behaviors,
                    delta_test_kind,
                } => {
                    // Determine the behavior of the latest incarnation of the transaction. The index
                    // is based on the value of the incarnation counter prior to the fetch_add during
                    // the last mock execution, and is >= 1 because there is at least one execution.
                    let incarnation_counter = incarnation_counter.swap(0, Ordering::SeqCst);
                    // Mock execute_transaction call always increments the incarnation counter. We
                    // perform a swap to 0 so later re-executions with the same transactions will
                    // also have a chance to start from scratch and e.g. assert below that at least
                    // one incarnation has been executed.
                    assert_gt!(
                        incarnation_counter,
                        0,
                        "Mock execution of txn {txn_idx} never incremented incarnation"
                    );
                    let last_incarnation = (incarnation_counter - 1) % incarnation_behaviors.len();

                    // Process the transaction
                    let gas_limit_exceeded = builder.process_transaction(
                        &incarnation_behaviors[last_incarnation],
                        *delta_test_kind,
                        txn_idx,
                        &mut accumulated_gas,
                        maybe_block_gas_limit,
                    );

                    // Break if gas limit exceeded
                    if gas_limit_exceeded {
                        break;
                    }
                },
                MockTransaction::InterruptRequested => unreachable!("Not tested with outputs"),
            }
        }

        // Initialize with empty delayed_field_key_to_id_map
        let mut result = builder.build();
        result.delayed_field_key_to_id_map = RefCell::new(HashMap::new());
        result
    }

    // Helper method to insert and validate delayed field IDs
    fn insert_or_verify_delayed_field_id(&self, key: K, id: DelayedFieldID) {
        let mut map = self.delayed_field_key_to_id_map.borrow_mut();
        assert!(
            map.insert(key, id)
                .map_or(true, |existing_id| existing_id == id),
            "Inconsistent delayed field ID mapping"
        );
    }

    // Verify the delayed field by checking ID, version, and value
    fn verify_delayed_field(
        &self,
        bytes: &[u8],
        baseline_key: &K,
        expected_version: u32,
        expected_value: u128,
        delayed_field_reads: &mut impl Iterator<Item = (DelayedFieldID, u128, K)>,
    ) {
        // Deserialize the ID and version from bytes
        let (id, version) =
            deserialize_to_delayed_field_id(bytes).expect("Must deserialize delayed field tuple");

        // Verify the version matches
        assert_eq!(
            expected_version, version,
            "Version mismatch for delayed field"
        );

        // Get the corresponding delayed field read
        let (delayed_id, value, key) = delayed_field_reads
            .next()
            .expect("Must have a delayed field read");

        // Verify the ID, key, and value match
        assert_eq!(id, delayed_id, "Delayed field ID mismatch");
        assert_eq!(*baseline_key, key, "Delayed field key mismatch");
        assert_eq!(expected_value, value, "Value mismatch for delayed field");

        // Add ID to key map and assert consistency if already present
        self.insert_or_verify_delayed_field_id(baseline_key.clone(), id);
    }

    fn assert_success<E: Clone + Debug + Send + Sync + TransactionEvent + 'static>(
        &self,
        block_output: &BlockOutput<MockTransaction<K, E>, MockOutput<K, E>>,
    ) {
        let mut group_world = HashMap::new();
        let mut group_metadata: HashMap<K, Option<StateValueMetadata>> = HashMap::new();

        let txn_outputs = block_output.get_transaction_outputs_forced();

        // Calculate the minimum number of valid iterations across all collections
        let valid_txn_count = [
            txn_outputs.len(),
            self.read_values.len(),
            self.resolved_deltas.len(),
            self.group_reads.len(),
            self.group_deltas.len(),
            self.module_reads.len(),
        ]
        .iter()
        .min()
        .copied()
        .unwrap_or(0);

        // Process transactions up to the minimum valid count
        for (txn_idx, txn_output) in txn_outputs.iter().enumerate().take(valid_txn_count) {
            // Verify transaction wasn't skipped
            assert!(
                !txn_output.skipped,
                "Error at txn {}: {:?}",
                txn_idx, txn_output.maybe_error_msg
            );

            // Compute group read information directly
            let group_read_infos = GroupReadInfo::compute_from_group_reads(
                &self.group_reads[txn_idx],
                &mut group_world,
            );

            // Process resource and group read results and delayed field reads
            let mut delayed_field_reads = txn_output.delayed_field_reads.clone().into_iter();
            let read_len = self.read_values[txn_idx].as_ref().unwrap().len();
            self.verify_resource_reads(
                &self.read_values[txn_idx],
                &txn_output.read_results[..read_len],
                &mut delayed_field_reads,
            );
            self.verify_group_reads(
                &group_read_infos,
                &txn_output.read_results[read_len..],
                &mut delayed_field_reads,
            );
            // Ensure all delayed field reads have been processed
            assert_none!(delayed_field_reads.next());

            self.verify_module_reads(
                &self.module_reads[txn_idx],
                &txn_output.module_read_results,
                txn_idx,
            );
            self.verify_group_size_metadata(txn_output, &mut group_world, &group_metadata);

            // Process writes and deltas and update the group world.
            self.process_group_writes(txn_output, &mut group_world, &mut group_metadata, txn_idx);

            let group_deltas = self.group_deltas[txn_idx].as_ref().unwrap();
            self.process_group_deltas(group_deltas, &mut group_world);
            self.verify_groups_patched_write_set(txn_output, &group_world, group_deltas);
            self.verify_materialized_deltas(txn_output, &self.resolved_deltas[txn_idx]);
        }

        // Check that remaining transactions are properly marked as skipped.
        let mut write_summary_flag = true;
        for txn_output in txn_outputs.iter().skip(valid_txn_count) {
            // Ensure the transaction is skipped based on the output
            assert!(txn_output.skipped);

            // materialized delta writes is only set by a callback for
            // committed transactions, which requires getting write summary.
            // However, the very first transaction that is not committed might
            // be an exception, which is why we use a boolean flag.
            if txn_output.materialized_delta_writes.get().is_some() {
                let called_write_summary = txn_output.called_write_summary.get().is_some();
                assert!(write_summary_flag || called_write_summary);
                write_summary_flag &= called_write_summary;
            }
        }
    }

    fn verify_resource_reads(
        &self,
        reads: &Result<Vec<(K, BaselineValue)>, ()>,
        read_results: &[Option<Vec<u8>>],
        delayed_field_reads: &mut impl Iterator<Item = (DelayedFieldID, u128, K)>,
    ) {
        for ((baseline_key, baseline_read), result_read) in reads
            .as_ref()
            .expect("Aggregator failures not yet tested")
            .iter()
            .zip(read_results)
        {
            match (baseline_read, result_read) {
                (BaselineValue::DelayedField(expected_value, expected_version), Some(bytes)) => {
                    self.verify_delayed_field(
                        bytes,
                        baseline_key,
                        *expected_version,
                        *expected_value,
                        delayed_field_reads,
                    );
                },
                (BaselineValue::DelayedField(_, _), None) => {
                    unreachable!("Deletes on delayed fields not yet tested");
                },
                (BaselineValue::GenericWrite(v), Some(bytes)) => {
                    assert_some_eq!(v.extract_raw_bytes(), *bytes);
                },
                (BaselineValue::GenericWrite(v), None) => {
                    assert_none!(v.extract_raw_bytes());
                },
                (BaselineValue::Aggregator(aggr_value), Some(bytes)) => {
                    assert_eq!(serialize(aggr_value), *bytes);
                },
                (BaselineValue::Aggregator(_), None) => {
                    unreachable!(
                        "Deleted or non-existent value from storage can't match aggregator value"
                    );
                },
                (BaselineValue::Empty(delta_test_kind), maybe_bytes) => match delta_test_kind {
                    DeltaTestKind::DelayedFields => {
                        assert_eq!(
                            maybe_bytes.as_ref().unwrap(),
                            &serialize_from_delayed_field_u128(STORAGE_AGGREGATOR_VALUE, u32::MAX)
                        );
                    },
                    DeltaTestKind::AggregatorV1 => {
                        assert_eq!(*maybe_bytes, Some(serialize(&STORAGE_AGGREGATOR_VALUE)));
                    },
                    DeltaTestKind::None => {
                        assert_none!(maybe_bytes);
                    },
                },
            }
        }
    }

    fn verify_group_reads(
        &self,
        group_infos: &[GroupReadInfo<K>],
        read_results: &[Option<Vec<u8>>],
        delayed_field_reads: &mut impl Iterator<Item = (DelayedFieldID, u128, K)>,
    ) {
        assert_eq!(group_infos.len(), read_results.len());

        for (group_info, result_group_read) in group_infos.iter().zip(read_results) {
            let result_bytes = result_group_read.clone().map(Into::<Bytes>::into);

            // Size check for all cases
            if let (Some(result), Some(baseline)) = (
                result_group_read.as_ref(),
                group_info.baseline_bytes.as_ref(),
            ) {
                assert_eq!(result.len(), baseline.len(), "Length mismatch for value");
            }

            match &group_info.maybe_delayed_field {
                Some((expected_value, expected_version)) => {
                    // Extract bytes from the result and verify delayed field invariants.
                    let result_bytes = result_bytes.expect("Must have a result for verification");
                    // Verify delayed field with all required parameters
                    self.verify_delayed_field(
                        result_bytes.as_ref(),
                        &group_info.group_key,
                        *expected_version,
                        *expected_value,
                        delayed_field_reads,
                    );
                },
                None => {
                    // Case 2: This is a regular value - just compare bytes directly
                    assert_eq!(
                        result_bytes, group_info.baseline_bytes,
                        "Result bytes don't match baseline value for regular field"
                    );
                },
            }
        }
    }

    fn verify_module_reads(
        &self,
        module_reads: &Result<Vec<Option<TxnIndex>>, ()>,
        module_read_results: &[Option<StateValueMetadata>],
        txn_idx: usize,
    ) {
        for (module_read, baseline_module_read) in module_read_results
            .iter()
            .zip(module_reads.as_ref().expect("No delta failures").iter())
        {
            assert_eq!(
                module_read
                    .as_ref()
                    .map(|m| m.creation_time_usecs())
                    .unwrap(),
                baseline_module_read
                    .map(|i| i as u64)
                    .unwrap_or(u32::MAX as u64),
                "for txn_idx = {}",
                txn_idx
            );
        }
    }

    fn verify_group_size_metadata<E: Debug>(
        &self,
        output: &MockOutput<K, E>,
        group_world: &mut HashMap<K, BTreeMap<u32, Bytes>>,
        group_metadata: &HashMap<K, Option<StateValueMetadata>>,
    ) {
        for (group_key, size_or_metadata) in output.read_group_size_or_metadata.iter() {
            let group_map = group_world
                .entry(group_key.clone())
                .or_insert_with(default_group_map);

            match size_or_metadata {
                GroupSizeOrMetadata::Size(size) => {
                    let baseline_size =
                        group_size_as_sum(group_map.iter().map(|(t, v)| (t, v.len())))
                            .unwrap()
                            .get();

                    assert_eq!(
                        baseline_size, *size,
                        "ERR: group_key {:?}, baseline size {} != output_size {}",
                        group_key, baseline_size, size
                    );
                },
                GroupSizeOrMetadata::Metadata(metadata) => {
                    if !group_metadata.contains_key(group_key) {
                        assert_eq!(*metadata, Some(raw_metadata(5)) /* default metadata */);
                    } else {
                        let baseline_metadata = group_metadata.get(group_key).cloned().flatten();
                        assert_eq!(*metadata, baseline_metadata);
                    }
                },
            }
        }
    }

    fn process_group_writes<E: Debug>(
        &self,
        output: &MockOutput<K, E>,
        group_world: &mut HashMap<K, BTreeMap<u32, Bytes>>,
        group_metadata: &mut HashMap<K, Option<StateValueMetadata>>,
        idx: usize,
    ) {
        for (group_key, v, group_size, updates) in output.group_writes.iter() {
            group_metadata.insert(group_key.clone(), v.as_state_value_metadata());

            let group_map = group_world
                .entry(group_key.clone())
                .or_insert_with(default_group_map);

            for (tag, (v, maybe_layout)) in updates {
                if v.is_deletion() {
                    assert_some!(group_map.remove(tag));
                } else {
                    let mut bytes = v.extract_raw_bytes().unwrap();

                    if maybe_layout.is_some() {
                        assert_eq!(*tag, RESERVED_TAG);
                        let (written_id, written_idx) =
                            deserialize_to_delayed_field_id(&bytes).unwrap();
                        let (current_value, _) = deserialize_to_delayed_field_u128(
                            group_map.get(&RESERVED_TAG).unwrap(),
                        )
                        .unwrap();
                        assert_eq!(written_idx, idx as u32);

                        // Use the helper method
                        self.insert_or_verify_delayed_field_id(group_key.clone(), written_id);

                        bytes = serialize_from_delayed_field_u128(current_value, written_idx);
                    }

                    let existed = group_map.insert(*tag, bytes).is_some();
                    assert_eq!(existed, v.is_modification());
                }
            }

            let computed_size =
                group_size_as_sum(group_map.iter().map(|(t, v)| (t, v.len()))).unwrap();
            assert_eq!(computed_size, *group_size);
        }
    }

    fn process_group_deltas(
        &self,
        group_deltas: &[(K, DeltaOp)],
        group_world: &mut HashMap<K, BTreeMap<u32, Bytes>>,
    ) {
        for (key, delta) in group_deltas.iter() {
            // Apply the delta and compute the new written value (retains txn_idx from the
            // previous write but updates the value).
            let value_with_delayed_field = group_world
                .entry(key.clone())
                .or_insert_with(default_group_map)
                .get_mut(&RESERVED_TAG)
                .expect("RESERVED_TAG must exist");

            let (value, version) =
                deserialize_to_delayed_field_u128(value_with_delayed_field).unwrap();

            let updated_value = delta
                .apply_to(value)
                .expect("Delta application failures not tested");

            *value_with_delayed_field = serialize_from_delayed_field_u128(updated_value, version);
        }
    }

    fn verify_groups_patched_write_set<E: Debug>(
        &self,
        output: &MockOutput<K, E>,
        group_world: &HashMap<K, BTreeMap<u32, Bytes>>,
        group_deltas: &[(K, DeltaOp)],
    ) {
        // TODO(BlockSTMv2: Do delta keys, as well as replaced_reads.
        let patched_resource_write_set = output
            .patched_resource_write_set
            .get()
            .expect("Patched resource write set must be set");

        for (key, maybe_size) in output
            .group_writes
            .iter()
            .map(|(k, _, size, _)| (k, Some(size)))
            .chain(group_deltas.iter().map(|(k, _)| (k, None)))
        {
            let patched_group_bytes = patched_resource_write_set.get(key).unwrap();
            let expected_group_map = group_world.get(key).unwrap();

            if patched_group_bytes.is_deletion() {
                assert!(maybe_size.map_or(true, |size| *size == ResourceGroupSize::zero_combined()));
            } else {
                let bytes = patched_group_bytes.extract_raw_bytes().unwrap();
                assert!(maybe_size.map_or(true, |size| size.get() == bytes.len() as u64));
                let patched_group_map: BTreeMap<u32, Bytes> = bcs::from_bytes(&bytes).unwrap();
                assert_eq!(patched_group_map, *expected_group_map);
            }
        }
    }

    fn verify_materialized_deltas<E: Debug>(
        &self,
        output: &MockOutput<K, E>,
        resolved_deltas: &Result<HashMap<K, u128>, ()>,
    ) {
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

        for (k, (_, _)) in output.reads_needing_exchange.iter() {
            let patched_resource = output
                .patched_resource_write_set
                .get()
                .unwrap()
                .get(k)
                .unwrap();

            let baseline_value = *baseline_deltas.get(k).expect("Baseline must contain delta");
            let (patched_value, _) =
                deserialize_to_delayed_field_u128(&patched_resource.extract_raw_bytes().unwrap())
                    .unwrap();
            assert_eq!(patched_value, baseline_value);
        }
    }

    // Used for testing, hence the function asserts the correctness conditions within
    // itself to be easily traceable in case of an error.
    pub(crate) fn assert_output<E: Clone + Debug + Send + Sync + TransactionEvent + 'static>(
        &self,
        results: &BlockExecutionResult<BlockOutput<MockTransaction<K, E>, MockOutput<K, E>>, usize>,
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

    pub(crate) fn assert_parallel_output<
        E: Clone + Debug + Send + Sync + TransactionEvent + 'static,
    >(
        &self,
        results: &Result<BlockOutput<MockTransaction<K, E>, MockOutput<K, E>>, ()>,
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
