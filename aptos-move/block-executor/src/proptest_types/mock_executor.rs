// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    proptest_types::types::{
        deserialize_to_delayed_field_id, serialize_from_delayed_field_id, 
        GroupSizeOrMetadata, MockIncarnation, MockTransaction, ValueType, RESERVED_TAG,
    },
    task::{ExecutionStatus, ExecutorTask, TransactionOutput},
    try_with,
};
use aptos_aggregator::{
    delayed_change::DelayedChange,
    delta_change_set::DeltaOp,
    resolver::TAggregatorV1View,
};
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::{
    contract_event::TransactionEvent,
    error::PanicError,
    executable::ModulePath,
    fee_statement::FeeStatement,
    state_store::{
        state_value::StateValueMetadata,
        TStateView,
    },
    transaction::BlockExecutableTransaction as Transaction,
    write_set::{TransactionWrite, WriteOp, WriteOpKind},
};
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_types::{
    module_and_script_storage::code_storage::AptosCodeStorage,
    module_write_set::ModuleWrite,
    resolver::{BlockSynchronizationKillSwitch, ResourceGroupSize, TExecutorView, TResourceGroupView},
    resource_group_adapter::{decrement_size_for_remove_tag, group_tagged_resource_size, increment_size_for_add_tag},
};
use bytes::Bytes;
use claims::assert_ok;
use move_core_types::{
    language_storage::ModuleId,
    value::{MoveStructLayout, MoveTypeLayout},
    vm_status::StatusCode,
};
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use once_cell::sync::OnceCell;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt::Debug,
    hash::Hash,
    marker::PhantomData,
    sync::{
        atomic::Ordering,
        Arc,
    },
};

/// A lazily initialized empty struct layout used throughout tests
/// 
/// This is an empty struct layout used specifically for testing delayed fields.
/// It's used when performing reads for resources that might contain delayed fields
/// to ensure consistent behavior across all test cases.
pub(crate) static MOCK_LAYOUT: once_cell::sync::Lazy<MoveTypeLayout> = 
    once_cell::sync::Lazy::new(|| MoveTypeLayout::Struct(MoveStructLayout::new(vec![])));

/// Macro for unwrapping Results or returning early on errors
/// 
/// This macro simplifies error handling by immediately returning the error value
/// from the current function when an error is encountered.
/// 
/// Supports different return modes:
/// - result: returns Err(e) for functions returning Result
/// - direct: directly returns e for functions returning errors
///
/// Usage:
///   try_with!(result_expr, result)  - For functions returning Result
///   try_with!(result_expr, direct)  - For functions directly returning errors
#[macro_export]
macro_rules! try_with {
    ($expr:expr, result) => {
        match $expr {
            Ok(val) => val,
            Err(e) => return Err(e),
        }
    };
    ($expr:expr, direct) => {
        match $expr {
            Ok(val) => val,
            Err(e) => return e,
        }
    };
}

#[derive(Debug)]
pub(crate) struct MockOutput<K, E> {
    pub(crate) writes: Vec<(K, ValueType)>,
    pub(crate) aggregator_v1_writes: Vec<(K, ValueType)>,
    // Key, metadata_op, inner_ops
    pub(crate) group_writes: Vec<(K, ValueType, ResourceGroupSize, HashMap<u32, ValueType>)>,
    pub(crate) module_writes: Vec<ModuleWrite<ValueType>>,
    pub(crate) deltas: Vec<(K, DeltaOp)>,
    pub(crate) events: Vec<E>,
    pub(crate) read_results: Vec<Option<Vec<u8>>>,
    pub(crate) delayed_field_reads: Vec<(DelayedFieldID, u128, K)>,
    pub(crate) module_read_results: Vec<Option<StateValueMetadata>>,
    pub(crate) read_group_size_or_metadata: Vec<(K, GroupSizeOrMetadata)>,
    pub(crate) materialized_delta_writes: OnceCell<Vec<(K, WriteOp)>>,
    pub(crate) total_gas: u64,
    pub(crate) skipped: bool,
    pub(crate) maybe_error_msg: Option<String>,
}

/// A builder for incrementally constructing MockOutput instances for cleaner code.
pub(crate) struct MockOutputBuilder<K, E> {
    pub(crate) output: MockOutput<K, E>,
}

impl<K: Clone + Debug, E: Clone> MockOutputBuilder<K, E> {
    /// Create a new builder from behavior
    pub(crate) fn from_behavior(
        behavior: &MockIncarnation<K, E>,
        delayed_fields_or_aggregator_v1: bool,
    ) -> Self {
        let output = MockOutput {
            writes: Vec::with_capacity(behavior.writes.len()),
            aggregator_v1_writes: behavior
                .writes
                .clone()
                .into_iter()
                .filter_map(|(k, v, has_delta)| {
                    (has_delta && !delayed_fields_or_aggregator_v1).then_some((k, v))
                })
                .collect(),
            group_writes: Vec::with_capacity(behavior.group_writes.len()),
            module_writes: behavior.module_writes.clone(),
            deltas: if !delayed_fields_or_aggregator_v1 {
                behavior.deltas.clone()
            } else {
                Vec::new()
            },
            events: behavior.events.to_vec(),
            read_results: Vec::with_capacity(behavior.reads.len()),
            delayed_field_reads: vec![],
            module_read_results: Vec::with_capacity(behavior.module_reads.len()),
            read_group_size_or_metadata: Vec::with_capacity(behavior.group_queries.len()),
            materialized_delta_writes: OnceCell::new(),
            total_gas: behavior.gas,
            skipped: false,
            maybe_error_msg: None,
        };

        Self { output }
    }
    
    /// Helper method to create a standardized error response
    fn create_error<T, D: Debug>(&self, error_prefix: &str, error: D) -> Result<T, ExecutionStatus<MockOutput<K, E>, usize>> {
        Err(ExecutionStatus::Success(
            MockOutput::with_error(&format!("{}: {:?}", error_prefix, error))
        ))
    }

    /// This method reads metadata for each module ID in the provided list
    /// and adds the results to the output.
    ///
    /// Returns self for method chaining
    pub(crate) fn add_module_reads<S: AptosCodeStorage>(
        &mut self,
        view: &S,
        module_ids: &[ModuleId],
    ) -> Result<&mut Self, ExecutionStatus<MockOutput<K, E>, usize>> {
        for module_id in module_ids {
            match view.fetch_state_value_metadata(&module_id.address(), &module_id.name()) {
                Ok(metadata) => self.output.module_read_results.push(metadata),
                Err(e) => return self.create_error("Failed to fetch module metadata", e),
            }
        }
        
        Ok(self)
    }

    /// This method reads bytes for regular resources and handles delayed fields as needed.
    ///
    /// Returns self for method chaining
    pub(crate) fn add_resource_reads(
        &mut self,
        view: &impl TExecutorView<K, u32, MoveTypeLayout, ValueType>,
        key_pairs: &[(K, bool)],
        delayed_fields_enabled: bool,
    ) -> Result<&mut Self, ExecutionStatus<MockOutput<K, E>, usize>> {
        for (key, has_deltas) in key_pairs {
            match (has_deltas, delayed_fields_enabled) {
                // Regular resource read (no delayed fields)
                (false, false) | (false, true) => {
                    match view.get_resource_bytes(key, None) {
                        Ok(v) => self.add_read_result(v.map(Into::into)),
                        Err(e) => return self.create_error("Failed to get resource bytes", e),
                    }
                },
                // Aggregator V1 read
                (true, false) => {
                    match view.get_aggregator_v1_state_value(key) {
                        Ok(v) => self.add_read_result(v.map(|state_value| state_value.bytes().clone().into())),
                        Err(e) => return self.create_error("Failed to get aggregator v1 state value", e),
                    }
                },
                // Delayed field read
                (true, true) => {
                    let bytes = match view.get_resource_bytes(key, Some(&*MOCK_LAYOUT)) {
                        Ok(v) => v.expect(
                            "In current tests, delayed field is always initialized",
                        ),
                        Err(e) => return self.create_error("Failed to get resource bytes with layout", e),
                    };

                    // Add bytes to read_results first 
                    self.add_read_result(Some(bytes.to_vec()));

                    // Then perform delayed field read if bytes were returned
                    try_with!(self.add_delayed_field_from_read_result(view, key, bytes.as_ref()), result);
                },
            }
        }
        
        Ok(self)
    }

    /// This method reads resources from groups and handles delayed fields as needed.
    ///
    /// Returns self for method chaining
    pub(crate) fn add_group_reads(
        &mut self,
        view: &(impl TResourceGroupView<GroupKey = K, ResourceTag = u32, Layout = MoveTypeLayout> 
              + TExecutorView<K, u32, MoveTypeLayout, ValueType>),
        group_reads: &[(K, u32, bool)],
        delayed_fields_enabled: bool,
    ) -> Result<&mut Self, ExecutionStatus<MockOutput<K, E>, usize>> {
        for (group_key, resource_tag, has_delta) in group_reads {
            let maybe_layout = (*has_delta
                && delayed_fields_enabled
                && *resource_tag == RESERVED_TAG)
                .then(|| (*MOCK_LAYOUT).clone());
                
            match view.get_resource_from_group(
                group_key,
                resource_tag,
                maybe_layout.as_ref(),
            ) {
                Ok(v) => {
                    self.add_read_result(v.clone().map(Into::into));
                    
                    // Perform delayed field read if needed
                    if *has_delta && delayed_fields_enabled {
                        assert_eq!(*resource_tag, RESERVED_TAG);
                        try_with!(self.add_delayed_field_from_read_result(
                            view, 
                            group_key, 
                            v.expect("In current tests, reserved tag always has a value").as_ref(), 
                        ), result);
                    }
                },
                Err(e) => return self.create_error("Failed to get resource from group", e),
            }
        }
        
        Ok(self)
    }

    /// Add group size or metadata queries to the output
    ///
    /// This method queries either the size or metadata of resource groups
    /// based on the query_metadata flag.
    ///
    /// Returns self for method chaining
    pub(crate) fn add_group_queries(
        &mut self,
        view: &(impl TResourceGroupView<GroupKey = K, ResourceTag = u32, Layout = MoveTypeLayout>
              + TExecutorView<K, u32, MoveTypeLayout, ValueType>),
        group_queries: &[(K, bool)],
    ) -> Result<&mut Self, ExecutionStatus<MockOutput<K, E>, usize>> {
        for (group_key, query_metadata) in group_queries {
            let res = if *query_metadata {
                // Query metadata
                match view.get_resource_state_value_metadata(group_key) {
                    Ok(v) => GroupSizeOrMetadata::Metadata(v),
                    Err(e) => return self.create_error("Failed to get resource state value metadata", e),
                }
            } else {
                // Query size
                match view.resource_group_size(group_key) {
                    Ok(v) => GroupSizeOrMetadata::Size(v.get()),
                    Err(e) => return self.create_error("Failed to get resource group size", e),
                }
            };

            self.output.read_group_size_or_metadata.push((group_key.clone(), res));
        }
        
        Ok(self)
    }

    /// This method handles the complex logic of processing group writes, including:
    /// - Getting the resource group size
    /// - Processing inner operations for each tag
    /// - Updating group size based on deletions and creations
    /// - Adding the final group write to the output
    ///
    /// Returns self for method chaining
    pub(crate) fn add_group_writes<View>(
        &mut self,
        view: &View,
        group_writes: &[(K, StateValueMetadata, HashMap<u32, (ValueType, bool)>)],
        delayed_fields_or_aggregator_v1: bool,
        txn_idx: u32,
    ) -> Result<&mut Self, ExecutionStatus<MockOutput<K, E>, usize>>
    where
        View: TResourceGroupView<GroupKey = K, ResourceTag = u32, Layout = MoveTypeLayout>
            + TExecutorView<K, u32, MoveTypeLayout, ValueType>,
    {
        // Group writes
        for (key, metadata, inner_ops) in group_writes {
            let mut new_inner_ops = HashMap::new();

            let mut new_group_size = match view.resource_group_size(key) {
                Ok(size) => size,
                Err(e) => return self.create_error("Failed to get resource group size", e),
            };
            let group_size = new_group_size.clone();

            for (tag, (inner_op, has_delayed_field)) in inner_ops.iter() {
                let exists = match view.get_resource_from_group(
                    key,
                    tag,
                    (*has_delayed_field
                        && delayed_fields_or_aggregator_v1
                        && *tag == RESERVED_TAG)
                        .then(|| &*MOCK_LAYOUT),
                ) {
                    Ok(v) => v.is_some(),
                    Err(e) => return self.create_error("Failed to get resource from group", e),
                };
                assert!(
                    *tag != RESERVED_TAG || exists,
                    "RESERVED_TAG must always be present in groups in tests"
                );

                // inner op is either deletion or creation.
                assert!(!inner_op.is_modification());

                let mut new_inner_op = inner_op.clone();
                if *has_delayed_field && delayed_fields_or_aggregator_v1 && new_inner_op.bytes().is_some() {
                    // For groups, delayed_fields_or_aggregator_v1 should always be
                    // true when has_delta is true & tag is RESERVED_TAG.
                    assert!(*tag == RESERVED_TAG);
                    let prev_id =
                        match view.get_resource_from_group(key, tag, Some(&*MOCK_LAYOUT)) {
                            Ok(bytes) => deserialize_to_delayed_field_id(&bytes.expect(
                                "In current tests, reserved tag always has a value",
                            ))
                            .expect(
                                "Mock deserialization failed in group delayed field test.",
                            )
                            .0,
                            Err(e) => return self.create_error("Failed to get resource from group", e),
                        };
                    new_inner_op
                        .set_bytes(serialize_from_delayed_field_id(prev_id, txn_idx));
                }

                let maybe_op = if exists {
                    Some(
                        if new_inner_op.is_creation()
                            && (new_inner_op.bytes().unwrap()[0] % 4 < 3
                                || *tag == RESERVED_TAG)
                        {
                            ValueType::new(
                                new_inner_op.bytes().cloned(),
                                StateValueMetadata::none(),
                                WriteOpKind::Modification,
                            )
                        } else {
                            ValueType::new(
                                None,
                                StateValueMetadata::none(),
                                WriteOpKind::Deletion,
                            )
                        },
                    )
                } else {
                    new_inner_op.is_creation().then(|| new_inner_op.clone())
                };

                if let Some(new_inner_op) = maybe_op {
                    if exists {
                        let old_tagged_value_size =
                            match view.resource_size_in_group(key, tag) {
                                Ok(size) => size,
                                Err(e) => return self.create_error("Failed to get resource size in group", e),
                            };
                        let old_size =
                            match group_tagged_resource_size(tag, old_tagged_value_size) {
                                Ok(size) => size,
                                Err(e) => return self.create_error("Failed to calculate group tagged resource size", e),
                            };
                        
                        if let Err(e) = decrement_size_for_remove_tag(&mut new_group_size, old_size) {
                            return self.create_error("Failed to decrement resource group size", e);
                        }
                    }
                    if !new_inner_op.is_deletion() {
                        let new_size = match group_tagged_resource_size(
                            tag,
                            new_inner_op.bytes().as_ref().unwrap().len(),
                        ) {
                            Ok(size) => size,
                            Err(e) => return self.create_error("Failed to calculate group tagged resource size", e),
                        };
                        
                        if let Err(e) = increment_size_for_add_tag(&mut new_group_size, new_size) {
                            return self.create_error("Failed to increment resource group size", e);
                        }
                    }

                    new_inner_ops.insert(*tag, new_inner_op);
                }
            }

            if !new_inner_ops.is_empty() {
                if group_size.get() > 0
                    && new_group_size == ResourceGroupSize::zero_combined()
                {
                    // TODO: reserved tag currently prevents this code from being run.
                    // Group got deleted.
                    self.output.group_writes.push((
                        key.clone(),
                        ValueType::new(None, metadata.clone(), WriteOpKind::Deletion),
                        new_group_size,
                        new_inner_ops,
                    ));
                } else {
                    let op_kind = if group_size.get() == 0 {
                        WriteOpKind::Creation
                    } else {
                        WriteOpKind::Modification
                    };

                    // Not testing metadata_op here, always modification.
                    self.output.group_writes.push((
                        key.clone(),
                        ValueType::new(Some(Bytes::new()), metadata.clone(), op_kind),
                        new_group_size,
                        new_inner_ops,
                    ));
                }
            }
        }
        
        Ok(self)
    }

    /// This method handles regular resource writes and delayed fields as needed.
    /// It processes writes and sets proper bytes for delayed fields.
    ///
    /// Returns self for method chaining
    pub(crate) fn add_resource_writes<View>(
        &mut self,
        view: &View,
        resource_writes: &[(K, ValueType, bool)],
        delayed_fields_or_aggregator_v1: bool,
        txn_idx: u32,
    ) -> Result<&mut Self, ExecutionStatus<MockOutput<K, E>, usize>>
    where
        View: TExecutorView<K, u32, MoveTypeLayout, ValueType>
    {
        for (k, new_value, has_delta) in resource_writes.iter() {
            let mut value_to_add = new_value.clone();
            
            if *has_delta && !delayed_fields_or_aggregator_v1 {
                // Already handled by aggregator_v1_writes.
                continue;
            }
            
            if *has_delta && delayed_fields_or_aggregator_v1 && value_to_add.bytes().is_some() {
                let prev_id = match view.get_resource_bytes(k, Some(&*MOCK_LAYOUT)) {
                    Ok(bytes) => {
                        deserialize_to_delayed_field_id(&bytes.expect(
                            "In current tests, delayed field is always initialized",
                        ))
                        .expect("Mock deserialization failed in delayed field test.")
                        .0
                    },
                    Err(e) => return self.create_error("Failed to get resource bytes", e),
                };
                value_to_add.set_bytes(serialize_from_delayed_field_id(prev_id, txn_idx));
            }

            self.output.writes.push((k.clone(), value_to_add));
        }
        
        Ok(self)
    }

    /// Build and return the final MockOutput
    pub(crate) fn build(self) -> MockOutput<K, E> {
        self.output
    }
    
    /// Perform a delayed field read and update the output accordingly.
    /// Returns an error ExecutionStatus if the read fails.
    fn add_delayed_field_from_read_result(
        &mut self,
        view: &impl TExecutorView<K, u32, MoveTypeLayout, ValueType>,
        key: &K,
        bytes: &[u8],
    ) -> Result<(), ExecutionStatus<MockOutput<K, E>, usize>> {
        let id = deserialize_to_delayed_field_id(bytes)
            .expect("Must deserialize delayed field tuple")
            .0;

        match view.get_delayed_field_value(&id) {
            Ok(v) => {
                let value = v.into_aggregator_value().unwrap();
                self.output.delayed_field_reads.push((id, value, key.clone()));
                Ok(())
            },
            Err(e) => self.create_error("Failed to get delayed field value", e),
        }
    }

    /// Add a normal read result
    fn add_read_result(&mut self, result: Option<Vec<u8>>) {
        self.output.read_results.push(result);
    }
}

impl<K, E> MockOutput<K, E> {
    // Helper method to create an empty MockOutput with common settings
    pub(crate) fn empty_output(skipped: bool, error_msg: Option<String>) -> Self {
        Self {
            writes: vec![],
            aggregator_v1_writes: vec![],
            group_writes: vec![],
            module_writes: vec![],
            deltas: vec![],
            events: vec![],
            read_results: vec![],
            delayed_field_reads: vec![],
            module_read_results: vec![],
            read_group_size_or_metadata: vec![],
            materialized_delta_writes: OnceCell::new(),
            total_gas: 0,
            skipped,
            maybe_error_msg: error_msg,
        }
    }

    // Helper method to create a MockOutput with an error message
    pub(crate) fn with_error(error: impl std::fmt::Display) -> Self {
        Self::empty_output(true, Some(format!("{}", error)))
    }

    // Helper method to create a MockOutput with a discard code
    pub(crate) fn with_discard_code(code: StatusCode) -> Self {
        Self::empty_output(true, Some(format!("Discarded with code: {:?}", code)))
    }

    // Helper for skip output
    pub(crate) fn skip_output() -> Self {
        Self::empty_output(true, None)
    }
}

impl<K, E> TransactionOutput for MockOutput<K, E>
where
    K: PartialOrd + Ord + Send + Sync + Clone + Hash + Eq + ModulePath + Debug + 'static,
    E: Send + Sync + Debug + Clone + TransactionEvent + 'static,
{
    type Txn = MockTransaction<K, E>;

    // TODO[agg_v2](tests): Assigning MoveTypeLayout as None for all the writes for now.
    // That means, the resources do not have any DelayedFields embedded in them.
    // Change it to test resources with DelayedFields as well.
    fn resource_write_set(&self) -> Vec<(K, Arc<ValueType>, Option<Arc<MoveTypeLayout>>)> {
        self.writes
            .iter()
            .map(|(key, value)| (key.clone(), Arc::new(value.clone()), None))
            .collect()
    }

    fn module_write_set(&self) -> Vec<ModuleWrite<ValueType>> {
        self.module_writes.clone()
    }

    // Aggregator v1 writes are included in resource_write_set for tests (writes are produced
    // for all keys including ones for v1_aggregators without distinguishing).
    fn aggregator_v1_write_set(&self) -> BTreeMap<K, ValueType> {
        self.aggregator_v1_writes.clone().into_iter().collect()
    }

    fn aggregator_v1_delta_set(&self) -> Vec<(K, DeltaOp)> {
        self.deltas.clone()
    }

    fn delayed_field_change_set(&self) -> BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>> {
        // TODO(BlockSTMv2): When we do delayed field deltas in execute, they are queried here.
        BTreeMap::new()
    }

    fn reads_needing_delayed_field_exchange(
        &self,
    ) -> Vec<(
        <Self::Txn as Transaction>::Key,
        StateValueMetadata,
        Arc<MoveTypeLayout>,
    )> {
        // TODO(BlockSTMv2): should implement this.
        Vec::new()
    }

    fn group_reads_needing_delayed_field_exchange(
        &self,
    ) -> Vec<(<Self::Txn as Transaction>::Key, StateValueMetadata)> {
        // TODO(BlockSTMv2): should implement this.
        Vec::new()
    }

    // TODO[agg_v2](cleanup) Using the concrete type layout here. Should we find a way to use generics?
    fn resource_group_write_set(
        &self,
    ) -> Vec<(
        K,
        ValueType,
        ResourceGroupSize,
        BTreeMap<u32, (ValueType, Option<Arc<MoveTypeLayout>>)>,
    )> {
        self.group_writes
            .iter()
            .cloned()
            .map(|(group_key, metadata_v, group_size, inner_ops)| {
                (
                    group_key,
                    metadata_v,
                    group_size,
                    inner_ops
                        .into_iter()
                        .map(|(tag, v)| (tag, (v, None)))
                        .collect(),
                )
            })
            .collect()
    }

    fn skip_output() -> Self {
        Self::empty_output(true, None)
    }

    fn discard_output(discard_code: StatusCode) -> Self {
        Self::with_discard_code(discard_code)
    }

    fn output_approx_size(&self) -> u64 {
        // TODO add block output limit testing
        0
    }

    fn get_write_summary(
        &self,
    ) -> HashSet<
        crate::types::InputOutputKey<
            <Self::Txn as Transaction>::Key,
            <Self::Txn as Transaction>::Tag,
        >,
    > {
        HashSet::new()
    }

    fn materialize_agg_v1(
        &self,
        _view: &impl TAggregatorV1View<Identifier = <Self::Txn as Transaction>::Key>,
    ) {
        // TODO[agg_v2](tests): implement this method and compare
        // against sequential execution results v. aggregator v1.
    }

    // TODO[agg_v2](tests): Currently, appending None to all events, which means none of the
    // events have aggregators. Test it with aggregators as well.
    fn get_events(&self) -> Vec<(E, Option<MoveTypeLayout>)> {
        self.events.iter().map(|e| (e.clone(), None)).collect()
    }

    fn incorporate_materialized_txn_output(
        &self,
        aggregator_v1_writes: Vec<(<Self::Txn as Transaction>::Key, WriteOp)>,
        patched_resource_write_set: Vec<(
            <Self::Txn as Transaction>::Key,
            <Self::Txn as Transaction>::Value,
        )>,
        _patched_events: Vec<<Self::Txn as Transaction>::Event>,
    ) -> Result<(), PanicError> {
        let resources: HashMap<<Self::Txn as Transaction>::Key, <Self::Txn as Transaction>::Value> =
            patched_resource_write_set.clone().into_iter().collect();
        for (key, _, size, _) in &self.group_writes {
            let v = resources.get(key).unwrap();
            if v.is_deletion() {
                assert_eq!(*size, ResourceGroupSize::zero_combined());
            } else {
                assert_eq!(
                    size.get(),
                    resources.get(key).unwrap().bytes().map_or(0, |b| b.len()) as u64
                );
            }
        }

        assert_ok!(self.materialized_delta_writes.set(aggregator_v1_writes));
        // TODO[agg_v2](tests): Set the patched resource write set and events. But that requires the function
        // to take &mut self as input
        Ok(())
    }

    fn set_txn_output_for_non_dynamic_change_set(&self) {
        // No compatibility issues here since the move-vm doesn't use the dynamic flag.
    }

    fn fee_statement(&self) -> FeeStatement {
        // First argument is supposed to be total (not important for the test though).
        // Next two arguments are different kinds of execution gas that are counted
        // towards the block limit. We split the total into two pieces for these arguments.
        // TODO: add variety to generating fee statement based on total gas.
        FeeStatement::new(
            self.total_gas,
            self.total_gas / 2,
            (self.total_gas + 1) / 2,
            0,
            0,
        )
    }
}

#[derive(Clone, Debug)]
pub(crate) struct MockEvent {
    event_data: Vec<u8>,
}

impl TransactionEvent for MockEvent {
    fn get_event_data(&self) -> &[u8] {
        &self.event_data
    }

    fn set_event_data(&mut self, event_data: Vec<u8>) {
        self.event_data = event_data;
    }
}

pub(crate) struct MockTask<K, E> {
    phantom_data: PhantomData<(K, E)>,
}

impl<K, E> MockTask<K, E> {
    pub fn new() -> Self {
        Self {
            phantom_data: PhantomData,
        }
    }
}

impl<K, E> ExecutorTask for MockTask<K, E>
where
    K: PartialOrd + Ord + Send + Sync + Clone + Hash + Eq + ModulePath + Debug + 'static,
    E: Send + Sync + Debug + Clone + TransactionEvent + 'static,
{
    type Error = usize;
    type Output = MockOutput<K, E>;
    type Txn = MockTransaction<K, E>;

    fn init(_environment: &AptosEnvironment, _state_view: &impl TStateView<Key = K>) -> Self {
        Self::new()
    }

    fn execute_transaction(
        &self,
        view: &(impl TExecutorView<K, u32, MoveTypeLayout, ValueType>
              + TResourceGroupView<GroupKey = K, ResourceTag = u32, Layout = MoveTypeLayout>
              + AptosCodeStorage
              + BlockSynchronizationKillSwitch),
        txn: &Self::Txn,
        txn_idx: TxnIndex,
    ) -> ExecutionStatus<Self::Output, Self::Error> {
        match txn {
            MockTransaction::Write {
                incarnation_counter,
                incarnation_behaviors,
                delayed_fields_or_aggregator_v1,
            } => {
                // Use incarnation counter value as an index to determine the read-
                // and write-sets of the execution. Increment incarnation counter to
                // simulate dynamic behavior when there are multiple possible read-
                // and write-sets (i.e. each are selected round-robin).
                let idx = incarnation_counter.fetch_add(1, Ordering::SeqCst);
                let behavior = &incarnation_behaviors[idx % incarnation_behaviors.len()];

                // Initialize the builder and use the railway pattern to execute builder operations.
                let mut builder = MockOutputBuilder::from_behavior(behavior, *delayed_fields_or_aggregator_v1);
                let builder_result = BuilderOperation::new(&mut builder)
                    .and_then(|b| b.add_module_reads(view, &behavior.module_reads))
                    .and_then(|b| b.add_resource_reads(view, &behavior.reads, *delayed_fields_or_aggregator_v1))
                    .and_then(|b| b.add_group_reads(view, &behavior.group_reads, *delayed_fields_or_aggregator_v1))
                    .and_then(|b| b.add_group_queries(view, &behavior.group_queries))
                    .and_then(|b| b.add_group_writes(view, &behavior.group_writes, *delayed_fields_or_aggregator_v1, txn_idx))
                    .and_then(|b| b.add_resource_writes(view, &behavior.writes, *delayed_fields_or_aggregator_v1, txn_idx))
                    .finish();
                
                // Use the direct return variant for ExecutionStatus functions
                try_with!(builder_result, direct);

                ExecutionStatus::Success(builder.build())
            },
            MockTransaction::SkipRest(gas) => {
                let mut mock_output = MockOutput::skip_output();
                mock_output.total_gas = *gas;
                ExecutionStatus::SkipRest(mock_output)
            },
            MockTransaction::Abort => ExecutionStatus::Abort(txn_idx as usize),
            MockTransaction::InterruptRequested => {
                while !view.interrupt_requested() {}
                ExecutionStatus::SkipRest(MockOutput::skip_output())
            },
        }
    }

    fn is_transaction_dynamic_change_set_capable(_txn: &Self::Txn) -> bool {
        true
    }
}

/// Railway-oriented pattern wrapper for builder operations
/// 
/// This implements a simple railway-oriented pattern for chaining operations
/// that might fail, allowing for a cleaner code flow.
struct BuilderOperation<'a, K: Clone + Debug, E: Clone> {
    builder: &'a mut MockOutputBuilder<K, E>,
    status: Option<ExecutionStatus<MockOutput<K, E>, usize>>,
}

impl<'a, K: Clone + Debug, E: Clone> BuilderOperation<'a, K, E> {
    fn new(builder: &'a mut MockOutputBuilder<K, E>) -> Self {
        Self { builder, status: None }
    }
    
    fn and_then<F>(mut self, op: F) -> Self 
    where 
        F: FnOnce(&mut MockOutputBuilder<K, E>) -> Result<&mut MockOutputBuilder<K, E>, ExecutionStatus<MockOutput<K, E>, usize>> 
    {
        if self.status.is_none() {
            if let Err(status) = op(self.builder) {
                self.status = Some(status);
            }
        }
        self
    }
    
    fn finish(self) -> Result<&'a mut MockOutputBuilder<K, E>, ExecutionStatus<MockOutput<K, E>, usize>> {
        match self.status {
            None => Ok(self.builder),
            Some(status) => Err(status),
        }
    }
} 