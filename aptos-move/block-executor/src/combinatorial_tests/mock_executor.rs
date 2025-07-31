// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    combinatorial_tests::types::{
        DeltaTestKind, GroupSizeOrMetadata, MockIncarnation, MockTransaction, ValueType,
        RESERVED_TAG,
    },
    task::{
        AfterMaterializationOutput, BeforeMaterializationOutput, ExecutionStatus, ExecutorTask,
        TransactionOutput,
    },
    types::delayed_field_mock_serialization::{
        deserialize_to_delayed_field_id, serialize_from_delayed_field_id,
    },
};
use aptos_aggregator::{
    bounded_math::SignedU128,
    delayed_change::{DelayedApplyChange, DelayedChange},
    delta_change_set::{DeltaOp, DeltaWithMax},
    resolver::TAggregatorV1View,
};
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::{
    contract_event::TransactionEvent,
    error::PanicError,
    executable::ModulePath,
    fee_statement::FeeStatement,
    state_store::{state_value::StateValueMetadata, TStateView},
    transaction::AuxiliaryInfo,
    write_set::{TransactionWrite, WriteOp, WriteOpKind},
};
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_types::{
    module_and_script_storage::code_storage::AptosCodeStorage,
    module_write_set::ModuleWrite,
    resolver::{
        BlockSynchronizationKillSwitch, ResourceGroupSize, TExecutorView, TResourceGroupView,
    },
    resource_group_adapter::{
        decrement_size_for_remove_tag, group_tagged_resource_size, increment_size_for_add_tag,
    },
};
use bytes::Bytes;
use claims::{assert_none, assert_ok};
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
    sync::{atomic::Ordering, Arc},
};

/// A lazily initialized empty struct layout used throughout tests
///
/// This is an empty struct layout used specifically for testing delayed fields.
/// It's used when performing reads for resources that might contain delayed fields
/// to ensure consistent behavior across all test cases.
pub(crate) static MOCK_LAYOUT: once_cell::sync::Lazy<MoveTypeLayout> =
    once_cell::sync::Lazy::new(|| MoveTypeLayout::Struct(MoveStructLayout::new(vec![])));

/// Macro for returning an error directly when Result is an error
///
/// This macro unwraps a Result or returns the error directly.
/// Used when the function returns the same error type as the Result.
///
/// Usage:
///   try_with_direct!(result_expr)
#[macro_export]
macro_rules! try_with_direct {
    ($expr:expr) => {
        match $expr {
            Ok(val) => val,
            Err(e) => return e,
        }
    };
}

/// Macro for returning Err(e) when Result is an error
///
/// This macro unwraps a Result or returns Err(e).
/// Used when the function returns Result<T, E>.
///
/// Usage:
///   try_with_error!(result_expr)
#[macro_export]
macro_rules! try_with_error {
    ($expr:expr) => {
        match $expr {
            Ok(val) => val,
            Err(e) => return Err(e),
        }
    };
}

/// Macro for returning an ExecutionStatus with error message
///
/// This macro unwraps a Result or returns an error wrapped in
/// ExecutionStatus::Success(MockOutput::with_error(...)).
///
/// Usage:
///   try_with_status!(result_expr, "error message")
#[macro_export]
macro_rules! try_with_status {
    ($expr:expr, $msg:expr) => {
        match $expr {
            Ok(val) => val,
            Err(e) => {
                return Err(ExecutionStatus::Success(MockOutput::with_error(&format!(
                    "{}: {:?}",
                    $msg, e
                ))))
            },
        }
    };
}

#[derive(Debug)]
pub(crate) struct MockOutput<K, E> {
    pub(crate) writes: Vec<(K, ValueType, Option<Arc<MoveTypeLayout>>)>,
    pub(crate) aggregator_v1_writes: Vec<(K, ValueType)>,
    // Key, metadata_op, inner_ops
    pub(crate) group_writes: Vec<(
        K,
        ValueType,
        ResourceGroupSize,
        BTreeMap<u32, (ValueType, Option<Arc<MoveTypeLayout>>)>,
    )>,
    pub(crate) module_writes: BTreeMap<K, ModuleWrite<ValueType>>,
    pub(crate) deltas: Vec<(K, DeltaOp, Option<(DelayedFieldID, bool)>)>,
    pub(crate) events: Vec<E>,
    pub(crate) read_results: Vec<Option<Vec<u8>>>,
    pub(crate) delayed_field_reads: Vec<(DelayedFieldID, u128, K)>,
    pub(crate) module_read_results: Vec<Option<StateValueMetadata>>,
    pub(crate) read_group_size_or_metadata: Vec<(K, GroupSizeOrMetadata)>,
    pub(crate) materialized_delta_writes: OnceCell<Vec<(K, WriteOp)>>,
    pub(crate) patched_resource_write_set: OnceCell<HashMap<K, ValueType>>,
    pub(crate) total_gas: u64,
    pub(crate) called_write_summary: OnceCell<()>,
    pub(crate) skipped: bool,
    pub(crate) maybe_error_msg: Option<String>,
    pub(crate) reads_needing_exchange: HashMap<K, (StateValueMetadata, Arc<MoveTypeLayout>)>,
    pub(crate) group_reads_needing_exchange: HashMap<K, StateValueMetadata>,
}

/// A builder for incrementally constructing MockOutput instances for cleaner code.
pub(crate) struct MockOutputBuilder<K, E> {
    pub(crate) output: MockOutput<K, E>,
}

impl<K: Ord + Clone + Debug + Eq + PartialEq + Hash, E: Clone> MockOutputBuilder<K, E> {
    /// Create a new builder from mock incarnation.
    pub(crate) fn from_mock_incarnation(
        mock_incarnation: &MockIncarnation<K, E>,
        delta_test_kind: DeltaTestKind,
    ) -> Self {
        let output = MockOutput {
            writes: Vec::with_capacity(mock_incarnation.resource_writes.len()),
            aggregator_v1_writes: mock_incarnation
                .resource_writes
                .clone()
                .into_iter()
                .filter_map(|(k, v, has_delta)| {
                    (has_delta && delta_test_kind == DeltaTestKind::AggregatorV1).then_some((k, v))
                })
                .collect(),
            group_writes: Vec::with_capacity(mock_incarnation.group_writes.len()),
            module_writes: mock_incarnation.module_writes.clone().into_iter().collect(),
            deltas: Vec::with_capacity(mock_incarnation.deltas.len()),
            events: mock_incarnation.events.to_vec(),
            read_results: Vec::with_capacity(mock_incarnation.resource_reads.len()),
            delayed_field_reads: vec![],
            module_read_results: Vec::with_capacity(mock_incarnation.module_reads.len()),
            read_group_size_or_metadata: Vec::with_capacity(mock_incarnation.group_queries.len()),
            materialized_delta_writes: OnceCell::new(),
            patched_resource_write_set: OnceCell::new(),
            total_gas: mock_incarnation.gas,
            called_write_summary: OnceCell::new(),
            skipped: false,
            maybe_error_msg: None,
            reads_needing_exchange: HashMap::new(),
            group_reads_needing_exchange: HashMap::new(),
        };

        Self { output }
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
            let metadata = try_with_status!(
                view.unmetered_get_module_state_value_metadata(
                    module_id.address(),
                    module_id.name()
                ),
                "Failed to fetch module metadata"
            );
            self.output.module_read_results.push(metadata);
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
                    let v = try_with_status!(
                        view.get_resource_bytes(key, None),
                        "Failed to get resource bytes"
                    );
                    self.add_read_result(v.map(Into::into));
                },
                // Aggregator V1 read
                (true, false) => {
                    let v = try_with_status!(
                        view.get_aggregator_v1_state_value(key),
                        "Failed to get aggregator v1 state value"
                    );
                    self.add_read_result(v.map(|state_value| state_value.bytes().clone().into()));
                },
                // Delayed field read
                (true, true) => {
                    let bytes = try_with_status!(
                        view.get_resource_bytes(key, Some(&*MOCK_LAYOUT)),
                        "Failed to get resource bytes with layout"
                    )
                    .expect("In current tests, delayed field is always initialized");

                    // Add bytes to read_results first
                    self.add_read_result(Some(bytes.to_vec()));

                    // Then perform delayed field read if bytes were returned
                    try_with_error!(self.add_delayed_field_from_read_result(
                        view,
                        key,
                        bytes.as_ref()
                    ));
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
            let maybe_layout =
                (*has_delta && delayed_fields_enabled && *resource_tag == RESERVED_TAG)
                    .then(|| (*MOCK_LAYOUT).clone());

            let v = try_with_status!(
                view.get_resource_from_group(group_key, resource_tag, maybe_layout.as_ref()),
                "Failed to get resource from group"
            );

            self.add_read_result(v.clone().map(Into::into));

            // Perform delayed field read if needed
            if *has_delta && delayed_fields_enabled {
                assert_eq!(*resource_tag, RESERVED_TAG);
                try_with_error!(self.add_delayed_field_from_read_result(
                    view,
                    group_key,
                    v.expect("RESERVED_TAG always contains a value").as_ref(),
                ));
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
                let v = try_with_status!(
                    view.get_resource_state_value_metadata(group_key),
                    "Failed to get resource state value metadata"
                );
                GroupSizeOrMetadata::Metadata(v)
            } else {
                // Query size
                let v = try_with_status!(
                    view.resource_group_size(group_key),
                    "Failed to get resource group size"
                );
                GroupSizeOrMetadata::Size(v.get())
            };

            self.output
                .read_group_size_or_metadata
                .push((group_key.clone(), res));
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
        delayed_fields_enabled: bool,
        txn_idx: u32,
    ) -> Result<&mut Self, ExecutionStatus<MockOutput<K, E>, usize>>
    where
        View: TResourceGroupView<GroupKey = K, ResourceTag = u32, Layout = MoveTypeLayout>
            + TExecutorView<K, u32, MoveTypeLayout, ValueType>,
    {
        // Group writes
        for (key, metadata, inner_ops) in group_writes {
            let mut new_inner_ops = BTreeMap::new();

            let mut new_group_size = try_with_status!(
                view.resource_group_size(key),
                "Failed to get resource group size"
            );
            let group_size = new_group_size;

            for (tag, (inner_op, has_delayed_field)) in inner_ops.iter() {
                let maybe_layout =
                    (*has_delayed_field && delayed_fields_enabled && *tag == RESERVED_TAG)
                        .then(|| MOCK_LAYOUT.clone());
                let exists = try_with_status!(
                    view.get_resource_from_group(key, tag, maybe_layout.as_ref(),),
                    "Failed to get resource from group"
                )
                .is_some();
                assert!(
                    *tag != RESERVED_TAG || exists,
                    "RESERVED_TAG must always be present in groups in tests"
                );

                // inner op is either deletion or creation.
                assert!(!inner_op.is_modification());

                let mut new_inner_op = inner_op.clone();
                let mut new_inner_op_layout = None;
                if *has_delayed_field && delayed_fields_enabled && new_inner_op.bytes().is_some() {
                    // For groups, delayed_fields_enabled should always be
                    // true when has_delta is true & tag is RESERVED_TAG.
                    assert!(*tag == RESERVED_TAG);
                    let prev_id = self.get_delayed_field_id_from_resource(view, key, Some(*tag))?;
                    new_inner_op.set_bytes(serialize_from_delayed_field_id(prev_id, txn_idx));
                    new_inner_op_layout = Some(Arc::new(MOCK_LAYOUT.clone()));
                }

                let maybe_op = if exists {
                    Some(
                        if new_inner_op.is_creation()
                            && (new_inner_op.bytes().unwrap()[0] % 4 < 3 || *tag == RESERVED_TAG)
                        {
                            ValueType::new(
                                new_inner_op.bytes().cloned(),
                                StateValueMetadata::none(),
                                WriteOpKind::Modification,
                            )
                        } else {
                            ValueType::new(None, StateValueMetadata::none(), WriteOpKind::Deletion)
                        },
                    )
                } else {
                    new_inner_op.is_creation().then(|| new_inner_op.clone())
                };

                if let Some(new_inner_op) = maybe_op {
                    if exists {
                        let old_tagged_value_size = try_with_status!(
                            view.resource_size_in_group(key, tag),
                            "Failed to get resource size in group"
                        );
                        let old_size = try_with_status!(
                            group_tagged_resource_size(tag, old_tagged_value_size),
                            "Failed to calculate group tagged resource size"
                        );

                        try_with_status!(
                            decrement_size_for_remove_tag(&mut new_group_size, old_size),
                            "Failed to decrement resource group size"
                        );
                    }
                    if !new_inner_op.is_deletion() {
                        let new_size = try_with_status!(
                            group_tagged_resource_size(
                                tag,
                                new_inner_op.bytes().as_ref().unwrap().len(),
                            ),
                            "Failed to calculate group tagged resource size"
                        );

                        try_with_status!(
                            increment_size_for_add_tag(&mut new_group_size, new_size),
                            "Failed to increment resource group size"
                        );
                    }

                    new_inner_ops.insert(*tag, (new_inner_op, new_inner_op_layout));
                }
            }

            if !new_inner_ops.is_empty() {
                if group_size.get() > 0 && new_group_size.get() == 0 {
                    // Note: Even though currently the groups are never empty, speculatively the new
                    // size may still become zero, because atomicity is not guaranteed across
                    // existence queries: so even if RESERVED_TAG is present, a different tag might
                    // have been removed for exactly the same size.
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
        delayed_fields_enabled: bool,
        txn_idx: u32,
    ) -> Result<&mut Self, ExecutionStatus<MockOutput<K, E>, usize>>
    // Group view is because get_delayed_field_id_from_resource dispatches, but there is
    // a TODO to have TExecutorView contain TResourceGroupView anyway.
    where
        View: TExecutorView<K, u32, MoveTypeLayout, ValueType>
            + TResourceGroupView<GroupKey = K, ResourceTag = u32, Layout = MoveTypeLayout>,
    {
        for (k, new_value, has_delta) in resource_writes.iter() {
            let mut value_to_add = new_value.clone();
            let mut value_to_add_layout = None;
            if *has_delta && !delayed_fields_enabled {
                // Already handled by aggregator_v1_writes.
                continue;
            }

            if *has_delta && delayed_fields_enabled && value_to_add.bytes().is_some() {
                let prev_id = self.get_delayed_field_id_from_resource(view, k, None)?;
                value_to_add.set_bytes(serialize_from_delayed_field_id(prev_id, txn_idx));
                value_to_add_layout = Some(Arc::new(MOCK_LAYOUT.clone()));
            }

            self.output
                .writes
                .push((k.clone(), value_to_add, value_to_add_layout));
        }

        Ok(self)
    }

    /// This method processes the deltas and adds them to the output.
    /// It skips this step if delayed_fields_or_aggregator_v1 is true.
    ///
    /// Returns self for method chaining
    pub(crate) fn add_deltas(
        &mut self,
        view: &(impl TExecutorView<K, u32, MoveTypeLayout, ValueType>
              + TResourceGroupView<GroupKey = K, ResourceTag = u32, Layout = MoveTypeLayout>),
        deltas: &[(K, DeltaOp, Option<u32>)],
        delta_test_kind: DeltaTestKind,
    ) -> Result<&mut Self, ExecutionStatus<MockOutput<K, E>, usize>> {
        match delta_test_kind {
            DeltaTestKind::DelayedFields => {
                for (k, delta, maybe_tag) in deltas {
                    let id = self.get_delayed_field_id_from_resource(view, k, *maybe_tag)?;

                    // Currently, we test with base delta of 0 and a max value of u128::MAX.
                    let base_delta = &SignedU128::Positive(0);
                    let (delta_op, _, max_value) = delta.into_inner();
                    let success = try_with_status!(
                        view.delayed_field_try_add_delta_outcome(
                            &id, base_delta, &delta_op, max_value
                        ),
                        "Failed to apply delta to delayed field"
                    );

                    self.output
                        .deltas
                        .push((k.clone(), *delta, Some((id, success))));
                }
            },
            DeltaTestKind::AggregatorV1 => {
                self.output
                    .deltas
                    .extend(deltas.iter().map(|(k, delta, maybe_tag)| {
                        assert_none!(maybe_tag, "AggregatorV1 not supported in groups");
                        (k.clone(), *delta, None)
                    }));
            },
            DeltaTestKind::None => {},
        }

        Ok(self)
    }

    /// Build and return the final MockOutput
    pub(crate) fn build(self) -> MockOutput<K, E> {
        self.output
    }

    /// Helper to extract a delayed field ID for a resource key (assuming value is exchanged).
    fn get_delayed_field_id_from_resource(
        &mut self,
        view: &(impl TExecutorView<K, u32, MoveTypeLayout, ValueType>
              + TResourceGroupView<GroupKey = K, ResourceTag = u32, Layout = MoveTypeLayout>),
        key: &K,
        maybe_tag: Option<u32>,
    ) -> Result<DelayedFieldID, ExecutionStatus<MockOutput<K, E>, usize>> {
        let bytes = match maybe_tag {
            None => try_with_status!(
                view.get_resource_bytes(key, Some(&*MOCK_LAYOUT)),
                "Failed to get resource bytes"
            ),
            Some(tag) => try_with_status!(
                view.get_resource_from_group(key, &tag, Some(&*MOCK_LAYOUT)),
                "Failed to get resource bytes from group"
            ),
        }
        .expect("In current tests, delayed field is always initialized");

        if maybe_tag.is_some() {
            // TODO: test metadata.
            self.output
                .group_reads_needing_exchange
                .insert(key.clone(), StateValueMetadata::none());
        } else {
            self.output.reads_needing_exchange.insert(
                key.clone(),
                (StateValueMetadata::none(), Arc::new(MOCK_LAYOUT.clone())),
            );
        }

        Ok(deserialize_to_delayed_field_id(&bytes)
            .expect("Must deserialize delayed field tuple")
            .0)
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

        let v = try_with_status!(
            view.get_delayed_field_value(&id),
            "Failed to get delayed field value"
        );

        let value = v.into_aggregator_value().unwrap();
        self.output
            .delayed_field_reads
            .push((id, value, key.clone()));
        Ok(())
    }

    /// Add a normal read result
    fn add_read_result(&mut self, result: Option<Vec<u8>>) {
        self.output.read_results.push(result);
    }
}

impl<K, E> MockOutput<K, E> {
    fn empty_success_output() -> Self {
        Self {
            writes: vec![],
            aggregator_v1_writes: vec![],
            group_writes: vec![],
            module_writes: BTreeMap::new(),
            deltas: vec![],
            events: vec![],
            read_results: vec![],
            delayed_field_reads: vec![],
            module_read_results: vec![],
            read_group_size_or_metadata: vec![],
            materialized_delta_writes: OnceCell::new(),
            patched_resource_write_set: OnceCell::new(),
            total_gas: 0,
            called_write_summary: OnceCell::new(),
            skipped: false,
            maybe_error_msg: None,
            reads_needing_exchange: HashMap::new(),
            group_reads_needing_exchange: HashMap::new(),
        }
    }

    // Helper method to create an empty MockOutput with common settings
    pub(crate) fn skipped_output(error_msg: Option<String>) -> Self {
        Self {
            writes: vec![],
            aggregator_v1_writes: vec![],
            group_writes: vec![],
            module_writes: BTreeMap::new(),
            deltas: vec![],
            events: vec![],
            read_results: vec![],
            delayed_field_reads: vec![],
            module_read_results: vec![],
            read_group_size_or_metadata: vec![],
            materialized_delta_writes: OnceCell::new(),
            patched_resource_write_set: OnceCell::new(),
            total_gas: 0,
            called_write_summary: OnceCell::new(),
            skipped: true,
            maybe_error_msg: error_msg,
            reads_needing_exchange: HashMap::new(),
            group_reads_needing_exchange: HashMap::new(),
        }
    }

    // Helper method to create a MockOutput with an error message
    pub(crate) fn with_error(error: impl std::fmt::Display) -> Self {
        Self::skipped_output(Some(format!("{}", error)))
    }

    // Helper method to create a MockOutput with a discard code
    pub(crate) fn with_discard_code(code: StatusCode) -> Self {
        Self::skipped_output(Some(format!("Discarded with code: {:?}", code)))
    }
}

fn mock_fee_statement(total_gas: u64) -> FeeStatement {
    // First argument is supposed to be total (not important for the test though).
    // Next two arguments are different kinds of execution gas that are counted
    // towards the block limit. We split the total into two pieces for these arguments.
    // TODO: add variety to generating fee statement based on total gas.
    FeeStatement::new(total_gas, total_gas / 2, (total_gas + 1) / 2, 0, 0)
}

impl<K, E> TransactionOutput for MockOutput<K, E>
where
    K: PartialOrd + Ord + Send + Sync + Clone + Hash + Eq + ModulePath + Debug + 'static,
    E: Send + Sync + Debug + Clone + TransactionEvent + 'static,
{
    type AfterMaterializationGuard<'a> = &'a Self;
    type BeforeMaterializationGuard<'a> = &'a Self;
    type Txn = MockTransaction<K, E>;

    fn skip_output() -> Self {
        Self::skipped_output(None)
    }

    fn discard_output(discard_code: StatusCode) -> Self {
        Self::with_discard_code(discard_code)
    }

    fn before_materialization(&self) -> Result<Self::BeforeMaterializationGuard<'_>, PanicError> {
        Ok(self)
    }

    fn after_materialization(&self) -> Result<Self::AfterMaterializationGuard<'_>, PanicError> {
        Ok(self)
    }

    fn legacy_sequential_materialize_agg_v1(&self, _view: &impl TAggregatorV1View<Identifier = K>) {
        // TODO[agg_v2](tests): implement this method and compare
        // against sequential execution results v. aggregator v1.
    }

    fn incorporate_materialized_txn_output(
        &self,
        aggregator_v1_writes: Vec<(K, WriteOp)>,
        patched_resource_write_set: Vec<(K, ValueType)>,
        _patched_events: Vec<E>,
    ) -> Result<(), PanicError> {
        assert_ok!(self
            .patched_resource_write_set
            .set(patched_resource_write_set.clone().into_iter().collect()));
        assert_ok!(self.materialized_delta_writes.set(aggregator_v1_writes));
        // TODO: Also test patched events.
        Ok(())
    }

    fn set_txn_output_for_non_dynamic_change_set(&self) {
        // No compatibility issues here since the move-vm doesn't use the dynamic flag.
    }
}

impl<'a, K, E> BeforeMaterializationOutput<MockTransaction<K, E>> for &'a MockOutput<K, E>
where
    K: PartialOrd + Ord + Send + Sync + Clone + Hash + Eq + ModulePath + Debug + 'static,
    E: Send + Sync + Debug + Clone + TransactionEvent + 'static,
{
    fn resource_write_set(&self) -> Vec<(K, Arc<ValueType>, Option<Arc<MoveTypeLayout>>)> {
        self.writes
            .iter()
            .map(|(key, value, maybe_layout)| {
                (key.clone(), Arc::new(value.clone()), maybe_layout.clone())
            })
            .collect()
    }

    fn module_write_set(&self) -> &BTreeMap<K, ModuleWrite<ValueType>> {
        &self.module_writes
    }

    fn aggregator_v1_write_set(&self) -> BTreeMap<K, ValueType> {
        self.aggregator_v1_writes.clone().into_iter().collect()
    }

    fn aggregator_v1_delta_set(&self) -> BTreeMap<K, DeltaOp> {
        if !self.deltas.is_empty() && self.deltas[0].2.is_none() {
            // When testing with delayed fields the Option is Some(id, success).
            self.deltas
                .iter()
                .map(|(k, delta, _)| (k.clone(), *delta))
                .collect()
        } else {
            BTreeMap::new()
        }
    }

    fn delayed_field_change_set(&self) -> BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>> {
        // TODO: also test creation of delayed fields.
        if !self.deltas.is_empty() && self.deltas[0].2.is_some() {
            self.deltas
                .iter()
                .filter_map(|(_, delta, maybe_id)| {
                    let (id, success) = maybe_id.unwrap();
                    let (delta, _, _) = delta.into_inner();
                    success.then(|| {
                        (
                            id,
                            DelayedChange::Apply(DelayedApplyChange::AggregatorDelta {
                                delta: DeltaWithMax::new(delta, u128::MAX),
                            }),
                        )
                    })
                })
                .collect()
        } else {
            BTreeMap::new()
        }
    }

    fn reads_needing_delayed_field_exchange(
        &self,
    ) -> Vec<(K, StateValueMetadata, Arc<MoveTypeLayout>)> {
        self.reads_needing_exchange
            .iter()
            .map(|(key, (metadata, layout))| (key.clone(), metadata.clone(), layout.clone()))
            .collect()
    }

    fn group_reads_needing_delayed_field_exchange(&self) -> Vec<(K, StateValueMetadata)> {
        self.group_reads_needing_exchange
            .iter()
            .map(|(key, metadata)| (key.clone(), metadata.clone()))
            .collect()
    }

    fn resource_group_write_set(
        &self,
    ) -> HashMap<
        K,
        (
            ValueType,
            ResourceGroupSize,
            BTreeMap<u32, (ValueType, Option<Arc<MoveTypeLayout>>)>,
        ),
    > {
        self.group_writes
            .iter()
            .map(|(key, value, size, ops)| (key.clone(), (value.clone(), *size, ops.clone())))
            .collect()
    }

    fn for_each_resource_group_key_and_tags<F>(&self, mut callback: F) -> Result<(), PanicError>
    where
        F: FnMut(&K, HashSet<&u32>) -> Result<(), PanicError>,
    {
        for (key, _, _, ops) in self.group_writes.iter() {
            callback(key, ops.iter().map(|(tag, _)| tag).collect())?;
        }
        Ok(())
    }

    fn output_approx_size(&self) -> u64 {
        // TODO add block output limit testing
        0
    }

    fn get_write_summary(&self) -> HashSet<crate::types::InputOutputKey<K, u32>> {
        _ = self.called_write_summary.set(());
        HashSet::new()
    }

    fn get_events(&self) -> Vec<(E, Option<MoveTypeLayout>)> {
        self.events.iter().map(|e| (e.clone(), None)).collect()
    }

    fn fee_statement(&self) -> FeeStatement {
        mock_fee_statement(self.total_gas)
    }

    fn has_new_epoch_event(&self) -> bool {
        // For tests, it is ok to return false.
        false
    }
}

impl<'a, K, E> AfterMaterializationOutput<MockTransaction<K, E>> for &'a MockOutput<K, E>
where
    K: PartialOrd + Ord + Send + Sync + Clone + Hash + Eq + ModulePath + Debug + 'static,
    E: Send + Sync + Debug + Clone + TransactionEvent + 'static,
{
    fn fee_statement(&self) -> FeeStatement {
        mock_fee_statement(self.total_gas)
    }

    fn has_new_epoch_event(&self) -> bool {
        // For tests, it is ok to return false.
        false
    }

    fn is_kept_success(&self) -> bool {
        // A skipped transaction is not a success.
        !self.skipped
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
    type AuxiliaryInfo = AuxiliaryInfo;
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
        _auxiliary_info: &Self::AuxiliaryInfo,
        txn_idx: TxnIndex,
    ) -> ExecutionStatus<Self::Output, Self::Error> {
        match txn {
            MockTransaction::Write {
                incarnation_counter,
                incarnation_behaviors,
                delta_test_kind,
            } => {
                // Use incarnation counter value as an index to determine the read-
                // and write-sets of the execution. Increment incarnation counter to
                // simulate dynamic behavior when there are multiple possible read-
                // and write-sets (i.e. each are selected round-robin).
                let idx = incarnation_counter.fetch_add(1, Ordering::SeqCst);
                let behavior = &incarnation_behaviors[idx % incarnation_behaviors.len()];

                // Initialize the builder and use the railway pattern to execute builder operations.
                let mut builder =
                    MockOutputBuilder::from_mock_incarnation(behavior, *delta_test_kind);
                let builder_result = BuilderOperation::new(&mut builder)
                    .and_then(|b| b.add_module_reads(view, &behavior.module_reads))
                    .and_then(|b| {
                        b.add_resource_reads(
                            view,
                            &behavior.resource_reads,
                            *delta_test_kind == DeltaTestKind::DelayedFields,
                        )
                    })
                    .and_then(|b| {
                        b.add_group_reads(
                            view,
                            &behavior.group_reads,
                            *delta_test_kind == DeltaTestKind::DelayedFields,
                        )
                    })
                    .and_then(|b| b.add_group_queries(view, &behavior.group_queries))
                    .and_then(|b| {
                        b.add_group_writes(
                            view,
                            &behavior.group_writes,
                            *delta_test_kind == DeltaTestKind::DelayedFields,
                            txn_idx,
                        )
                    })
                    .and_then(|b| {
                        b.add_resource_writes(
                            view,
                            &behavior.resource_writes,
                            *delta_test_kind == DeltaTestKind::DelayedFields,
                            txn_idx,
                        )
                    })
                    .and_then(|b| b.add_deltas(view, &behavior.deltas, *delta_test_kind))
                    .finish();

                // Use the direct return variant for ExecutionStatus functions
                try_with_direct!(builder_result);

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
            MockTransaction::StateCheckpoint => {
                ExecutionStatus::Success(MockOutput::empty_success_output())
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
        Self {
            builder,
            status: None,
        }
    }

    fn and_then<F>(mut self, op: F) -> Self
    where
        F: FnOnce(
            &mut MockOutputBuilder<K, E>,
        )
            -> Result<&mut MockOutputBuilder<K, E>, ExecutionStatus<MockOutput<K, E>, usize>>,
    {
        if self.status.is_none() {
            if let Err(status) = op(self.builder) {
                self.status = Some(status);
            }
        }
        self
    }

    fn finish(
        self,
    ) -> Result<&'a mut MockOutputBuilder<K, E>, ExecutionStatus<MockOutput<K, E>, usize>> {
        match self.status {
            None => Ok(self.builder),
            Some(status) => Err(status),
        }
    }
}
