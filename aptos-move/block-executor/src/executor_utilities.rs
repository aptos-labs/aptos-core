// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    captured_reads::{CapturedReads, DataRead, ReadKind, TxnInput},
    counters,
    errors::ResourceGroupSerializationError,
    task::{LegacyTxnOutput, TxnOutput},
    txn_last_input_output::TxnLastInputOutput,
    view::{LatestView, ViewState},
};
use aptos_logger::error;
use aptos_mvhashmap::{types::TxnIndex, MVHashMap};
use aptos_types::{
    block_executor::value::{SpeculativeValue, ValueWithLayout},
    contract_event::TransactionEvent,
    error::{code_invariant_error, PanicError, PanicOr},
    state_store::{state_value::StateValue, TStateView},
    transaction::BlockExecutableTransaction as Transaction,
    vm::modules::AptosModuleExtension,
    write_set::TransactionWrite,
};
use aptos_vm_logging::{alert, clear_speculative_txn_logs, prelude::*};
use aptos_vm_types::{change_set::randomly_check_layout_matches, resolver::ResourceGroupSize};
use bytes::Bytes;
use fail::fail_point;
use move_binary_format::CompiledModule;
use move_core_types::{language_storage::ModuleId, value::MoveTypeLayout};
use move_vm_runtime::{execution_tracing::Trace, Module};
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use rand::{thread_rng, Rng};
use std::collections::BTreeMap;
use triomphe::Arc as TriompheArc;

/// Block executor state access required to materialize a transaction output at
/// commit time: finalizing resource groups and exchanging delayed field
/// identifiers back to values.
pub trait Materializer<T: Transaction> {
    /// Returns the committed contents of a resource group and its size.
    fn finalize_group(
        &self,
        key: &T::Key,
    ) -> Result<(Vec<(T::Tag, ValueWithLayout<T::Value>)>, ResourceGroupSize), PanicError>;

    /// Replaces delayed field identifiers in the serialized value with the
    /// corresponding committed values.
    fn replace_ids_with_values(
        &self,
        bytes: &Bytes,
        layout: &MoveTypeLayout,
    ) -> Result<Bytes, PanicError>;

    /// Returns the value (with layout) that the transaction read for a key whose
    /// contents must be re-written due to delayed field changes.
    fn fetch_exchanged_read(
        &self,
        key: &T::Key,
    ) -> Result<(TriompheArc<T::Value>, TriompheArc<MoveTypeLayout>), PanicError>;
}

/// Materializer for parallel execution: resource groups are finalized from the
/// multi-versioned data structure, and values that were read (and exchanged)
/// during execution are fetched from the captured read-set.
pub(crate) struct ParallelMaterializer<'a, T: Transaction, S: TStateView<Key = T::Key>> {
    latest_view: &'a LatestView<'a, T, S>,
    read_set: &'a CapturedReads<T, ModuleId, CompiledModule, Module, AptosModuleExtension>,
}

impl<'a, T: Transaction, S: TStateView<Key = T::Key>> ParallelMaterializer<'a, T, S> {
    pub(crate) fn new(
        latest_view: &'a LatestView<'a, T, S>,
        read_set: &'a CapturedReads<T, ModuleId, CompiledModule, Module, AptosModuleExtension>,
    ) -> Self {
        Self {
            latest_view,
            read_set,
        }
    }
}

impl<T: Transaction, S: TStateView<Key = T::Key>> Materializer<T>
    for ParallelMaterializer<'_, T, S>
{
    fn finalize_group(
        &self,
        key: &T::Key,
    ) -> Result<(Vec<(T::Tag, ValueWithLayout<T::Value>)>, ResourceGroupSize), PanicError> {
        match &self.latest_view.latest_view {
            ViewState::Sync(state) => state
                .versioned_map
                .group_data()
                .finalize_group(key, self.latest_view.txn_idx),
            ViewState::Unsync(_) => Err(code_invariant_error(
                "Parallel materializer requires the sync view state",
            )),
        }
    }

    fn replace_ids_with_values(
        &self,
        bytes: &Bytes,
        layout: &MoveTypeLayout,
    ) -> Result<Bytes, PanicError> {
        exchange_bytes(self.latest_view, bytes, layout)
    }

    fn fetch_exchanged_read(
        &self,
        key: &T::Key,
    ) -> Result<(TriompheArc<T::Value>, TriompheArc<MoveTypeLayout>), PanicError> {
        let data_read = self.read_set.get_by_kind(key, None, ReadKind::Value);
        if let Some(DataRead::Versioned(_, value, Some(layout))) = data_read {
            Ok((value, layout))
        } else {
            Err(code_invariant_error(format!(
                "Read value needing exchange {:?} not in Exchanged format",
                data_read
            )))
        }
    }
}

/// Materializer for sequential execution, based on the unsync map.
pub(crate) struct SequentialMaterializer<'a, T: Transaction, S: TStateView<Key = T::Key>> {
    latest_view: &'a LatestView<'a, T, S>,
}

impl<'a, T: Transaction, S: TStateView<Key = T::Key>> SequentialMaterializer<'a, T, S> {
    pub(crate) fn new(latest_view: &'a LatestView<'a, T, S>) -> Self {
        Self { latest_view }
    }
}

impl<T: Transaction, S: TStateView<Key = T::Key>> Materializer<T>
    for SequentialMaterializer<'_, T, S>
{
    fn finalize_group(
        &self,
        key: &T::Key,
    ) -> Result<(Vec<(T::Tag, ValueWithLayout<T::Value>)>, ResourceGroupSize), PanicError> {
        match &self.latest_view.latest_view {
            ViewState::Unsync(state) => {
                let (group, group_size) = state.unsync_map.finalize_group(key);
                Ok((group.collect(), group_size))
            },
            ViewState::Sync(_) => Err(code_invariant_error(
                "Sequential materializer requires the unsync view state",
            )),
        }
    }

    fn replace_ids_with_values(
        &self,
        bytes: &Bytes,
        layout: &MoveTypeLayout,
    ) -> Result<Bytes, PanicError> {
        exchange_bytes(self.latest_view, bytes, layout)
    }

    fn fetch_exchanged_read(
        &self,
        key: &T::Key,
    ) -> Result<(TriompheArc<T::Value>, TriompheArc<MoveTypeLayout>), PanicError> {
        match &self.latest_view.latest_view {
            ViewState::Unsync(state) => match state.unsync_map.fetch_data(key) {
                Some(ValueWithLayout::Exchanged(value, Some(layout))) => Ok((value, layout)),
                data => Err(code_invariant_error(format!(
                    "Read value needing exchange {:?} does not exist or not in Exchanged format",
                    data
                ))),
            },
            ViewState::Sync(_) => Err(code_invariant_error(
                "Sequential materializer requires the unsync view state",
            )),
        }
    }
}

/// Materializes a (speculative) transaction output into its committed form:
/// finalizes and serializes resource group updates, replaces delayed field
/// identifiers with values in resource writes and events, and incorporates the
/// results into the output.
pub(crate) fn materialize_output<T, O, M>(
    output: O,
    materializer: &M,
) -> Result<(O::CommittedOutput, Trace), PanicOr<ResourceGroupSerializationError>>
where
    T: Transaction,
    O: LegacyTxnOutput<Txn = T, Key = T::Key, Tag = T::Tag, Value = ValueWithLayout<T::Value>>,
    M: Materializer<T>,
{
    let group_metadata_ops = output.resource_group_metadata_ops();
    let group_reads_needing_exchange = output.group_reads_needing_delayed_field_exchange();
    let resource_write_set = output.resource_write_set();
    let reads_needing_exchange = output.reads_needing_delayed_field_exchange();
    let events = output.get_events();

    let finalized_groups = group_metadata_ops
        .into_iter()
        .map(|(group_key, metadata_op)| (group_key, metadata_op, false))
        .chain(
            group_reads_needing_exchange
                .into_iter()
                .map(|(group_key, metadata)| {
                    // Groups that were only read, but must be re-written because they
                    // contain delayed fields that changed: synthesize an empty write
                    // carrying the metadata.
                    let metadata_op = TransactionWrite::from_state_value(Some(
                        StateValue::new_with_metadata(Bytes::new(), metadata),
                    ));
                    (group_key, metadata_op, true)
                }),
        )
        .map(|(group_key, metadata_op, is_read_needing_exchange)| {
            let (finalized_group, group_size) = materializer.finalize_group(&group_key)?;
            map_finalized_group::<T>(
                group_key,
                finalized_group,
                group_size,
                metadata_op,
                is_read_needing_exchange,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let materialized_finalized_groups =
        map_id_to_values_in_group_writes(finalized_groups, materializer)?;
    let serialized_groups =
        serialize_groups::<T>(materialized_finalized_groups).map_err(PanicOr::Or)?;

    // Select resource writes that require ID replacement for delayed fields:
    // - reads needing replacement: error if data is not in Exchanged format,
    // - normal resource writes: writes that have a layout set and are not deletions.
    // Since reads needing exchange also do not contain deletions (see
    // 'does_value_need_exchange' logic in value_exchange.rs), it is guaranteed that
    // no returned value is a deletion.
    let resource_writes_to_materialize = reads_needing_exchange
        .into_iter()
        .map(|(key, metadata, layout)| -> Result<_, PanicError> {
            let (value, existing_layout) = materializer.fetch_exchanged_read(&key)?;
            randomly_check_layout_matches(Some(&existing_layout), Some(layout.as_ref()))?;
            let new_value = TriompheArc::new(TransactionWrite::from_state_value(Some(
                StateValue::new_with_metadata(
                    value.bytes().cloned().unwrap_or_else(Bytes::new),
                    metadata,
                ),
            )));
            Ok((key, ValueWithLayout::Exchanged(new_value, Some(layout))))
        })
        .chain(resource_write_set.into_iter().filter_map(|(key, value)| {
            (value.has_layout() && !value.is_deletion()).then_some(Ok((key, value)))
        }))
        .collect::<Result<Vec<_>, _>>()?;
    let materialized_resource_write_set =
        map_id_to_values_in_write_set(resource_writes_to_materialize, materializer)?;

    let materialized_events = map_id_to_values_events(events, materializer)?;

    Ok(output.incorporate_materialized_txn_output(
        materialized_resource_write_set
            .into_iter()
            .chain(serialized_groups)
            .collect(),
        materialized_events,
    )?)
}

/// Checks that every resource group this transaction finalizes will
/// BCS-serialize to its recorded size, merging the transaction's own group ops
/// onto the group's committed contents. Returns false if a finalized group
/// fails to serialize to its recorded size, so the caller can discard the
/// transaction.
pub fn check_resource_group_serialization<T, O, M>(output: &O, materializer: &M) -> bool
where
    T: Transaction,
    O: LegacyTxnOutput<Txn = T, Key = T::Key, Tag = T::Tag, Value = ValueWithLayout<T::Value>>,
    M: Materializer<T>,
{
    let serializes =
        |group: &BTreeMap<T::Tag, Bytes>, size: ResourceGroupSize| match bcs::to_bytes(group) {
            Ok(bytes) => {
                !((!group.is_empty() || size.get() != 0) && bytes.len() as u64 != size.get())
            },
            Err(_) => false,
        };
    let raw_bytes = |value: ValueWithLayout<T::Value>| {
        value
            .extract_value()
            .extract_raw_bytes()
            .expect("Deletions should already be applied")
    };

    // Groups only read but rewritten due to delayed-field changes: the finalized
    // contents are exactly the group already in the map.
    for (group_key, _) in output.group_reads_needing_delayed_field_exchange() {
        fail_point!("fail-point-resource-group-serialization", |_| false);
        let (finalized, size) = match materializer.finalize_group(&group_key) {
            Ok(finalized) => finalized,
            // finalize_group only errors outside sequential execution, where this
            // check does not run; treat as non-materializable defensively.
            Err(_) => return false,
        };
        let group = finalized
            .into_iter()
            .map(|(tag, value)| (tag, raw_bytes(value)))
            .collect();
        if !serializes(&group, size) {
            return false;
        }
    }

    // Groups written: merge this transaction's ops onto the prior contents.
    for (group_key, (_, output_size, group_ops)) in output.resource_group_write_set() {
        fail_point!("fail-point-resource-group-serialization", |_| false);
        let (finalized, size) = match materializer.finalize_group(&group_key) {
            Ok(finalized) => finalized,
            Err(_) => return false,
        };
        // The finalized size and the output's recorded size come from the same
        // output, so they are expected to be equal; this guard is defensive and
        // skips the group in the unexpected case that they diverge.
        if output_size.get() != size.get() {
            continue;
        }
        let mut group: BTreeMap<_, _> = finalized
            .into_iter()
            .map(|(tag, value)| (tag, raw_bytes(value)))
            .collect();
        for (tag, op) in group_ops {
            if op.is_deletion() {
                group.remove(&tag);
            } else {
                group.insert(
                    tag,
                    op.extract_value()
                        .extract_raw_bytes()
                        .expect("Non-deletion op must have raw bytes"),
                );
            }
        }
        if !serializes(&group, size) {
            return false;
        }
    }

    true
}

/// Given a state value, performs deserialization-serialization round-trip to
/// replace delayed field identifiers with the corresponding values.
fn exchange_bytes<T: Transaction, S: TStateView<Key = T::Key>>(
    latest_view: &LatestView<T, S>,
    bytes: &Bytes,
    layout: &MoveTypeLayout,
) -> Result<Bytes, PanicError> {
    latest_view
        .replace_identifiers_with_values(bytes, layout)
        .map(|(bytes, _)| bytes)
        .map_err(|e| {
            code_invariant_error(format!(
                "Failed to replace identifiers with values in {:?}: {:?}",
                layout, e
            ))
        })
}

fn map_finalized_group<T: Transaction>(
    group_key: T::Key,
    finalized_group: Vec<(T::Tag, ValueWithLayout<T::Value>)>,
    group_size: ResourceGroupSize,
    metadata_op: T::Value,
    is_read_needing_exchange: bool,
) -> Result<
    (
        T::Key,
        T::Value,
        Vec<(T::Tag, ValueWithLayout<T::Value>)>,
        ResourceGroupSize,
    ),
    PanicError,
> {
    let metadata_is_deletion = metadata_op.is_deletion();

    if is_read_needing_exchange && metadata_is_deletion {
        // Value needed exchange but was not written / modified during the txn
        // execution: may not be empty.
        Err(code_invariant_error(
            "Value only read and exchanged, but metadata op is Deletion".to_string(),
        ))
    } else if finalized_group.is_empty() != metadata_is_deletion {
        // finalize_group already applies the deletions.
        Err(code_invariant_error(format!(
            "Group is empty = {} but op is deletion = {} in parallel execution",
            finalized_group.is_empty(),
            metadata_is_deletion
        )))
    } else {
        Ok((group_key, metadata_op, finalized_group, group_size))
    }
}

fn serialize_groups<T: Transaction>(
    finalized_groups: Vec<(
        T::Key,
        T::Value,
        Vec<(T::Tag, TriompheArc<T::Value>)>,
        ResourceGroupSize,
    )>,
) -> Result<Vec<(T::Key, T::Value)>, ResourceGroupSerializationError> {
    fail_point!(
        "fail-point-resource-group-serialization",
        !finalized_groups.is_empty(),
        |_| Err(ResourceGroupSerializationError)
    );

    finalized_groups
        .into_iter()
        .map(
            |(group_key, mut metadata_op, finalized_group, group_size)| {
                let btree: BTreeMap<T::Tag, Bytes> = finalized_group
                    .into_iter()
                    .map(|(resource_tag, arc_v)| {
                        let bytes = arc_v
                            .extract_raw_bytes()
                            .expect("Deletions should already be applied");
                        (resource_tag, bytes)
                    })
                    .collect();

                match bcs::to_bytes(&btree) {
                    Ok(group_bytes) => {
                        if (!btree.is_empty() || group_size.get() != 0)
                            && group_bytes.len() as u64 != group_size.get()
                        {
                            alert!(
                                "Serialized resource group size mismatch key = {:?} num items {}, \
				 len {} recorded size {}, op {:?}",
                                group_key,
                                btree.len(),
                                group_bytes.len(),
                                group_size.get(),
                                metadata_op,
                            );
                            Err(ResourceGroupSerializationError)
                        } else {
                            metadata_op.set_bytes(group_bytes.into());
                            Ok((group_key, metadata_op))
                        }
                    },
                    Err(e) => {
                        alert!("Unexpected resource group error {:?}", e);
                        Err(ResourceGroupSerializationError)
                    },
                }
            },
        )
        .collect()
}

pub(crate) fn gen_id_start_value(sequential: bool) -> u32 {
    // IDs are ephemeral. Pick a random prefix, and different each time,
    // in case exchange is mistakenly not performed - to more easily catch it.
    // And in a bad case where it happens in prod, to and make sure incorrect
    // block doesn't get committed, but chain halts.
    // (take a different range from parallel execution, to even more easily differentiate)

    let offset = if sequential { 0 } else { 1000 };
    thread_rng().gen_range(1 + offset, 1000 + offset) * 1_000_000
}

fn map_id_to_values_in_group_writes<T: Transaction, M: Materializer<T>>(
    finalized_groups: Vec<(
        T::Key,
        T::Value,
        Vec<(T::Tag, ValueWithLayout<T::Value>)>,
        ResourceGroupSize,
    )>,
    materializer: &M,
) -> Result<
    Vec<(
        T::Key,
        T::Value,
        Vec<(T::Tag, TriompheArc<T::Value>)>,
        ResourceGroupSize,
    )>,
    PanicError,
> {
    let mut patched_finalized_groups = Vec::with_capacity(finalized_groups.len());
    for (group_key, group_metadata_op, resource_vec, group_size) in finalized_groups.into_iter() {
        let mut patched_resource_vec = Vec::with_capacity(resource_vec.len());
        for (tag, value_with_layout) in resource_vec.into_iter() {
            let value = match value_with_layout {
                ValueWithLayout::RawFromStorage(value) => value,
                ValueWithLayout::Exchanged(value, None) => value,
                ValueWithLayout::Exchanged(value, Some(layout)) => TriompheArc::new(
                    replace_ids_with_values(&value, layout.as_ref(), materializer)?,
                ),
            };
            patched_resource_vec.push((tag, value));
        }
        patched_finalized_groups.push((
            group_key,
            group_metadata_op,
            patched_resource_vec,
            group_size,
        ));
    }
    Ok(patched_finalized_groups)
}

// For each delayed field in resource write set, replace the identifiers with values
// (ignoring other writes). Currently also checks the keys are unique.
fn map_id_to_values_in_write_set<T: Transaction, M: Materializer<T>>(
    resource_write_set: Vec<(T::Key, ValueWithLayout<T::Value>)>,
    materializer: &M,
) -> Result<Vec<(T::Key, T::Value)>, PanicError> {
    resource_write_set
        .into_iter()
        .map(|(key, value)| match value {
            ValueWithLayout::Exchanged(write_op, Some(layout)) => Ok((
                key,
                replace_ids_with_values(&write_op, &layout, materializer)?,
            )),
            ValueWithLayout::Exchanged(_, None) | ValueWithLayout::RawFromStorage(_) => Err(
                code_invariant_error("Resource write to materialize must have a layout"),
            ),
        })
        .collect()
}

// For each delayed field in the event, replace delayed field identifier with value.
fn map_id_to_values_events<T: Transaction, M: Materializer<T>>(
    events: Vec<(T::Event, Option<MoveTypeLayout>)>,
    materializer: &M,
) -> Result<Vec<T::Event>, PanicError> {
    events
        .into_iter()
        .map(|(event, layout)| {
            if let Some(layout) = layout {
                let event_data = event.get_event_data();
                let patched_bytes = materializer
                    .replace_ids_with_values(&Bytes::from(event_data.to_vec()), &layout)?;
                let mut patched_event = event;
                patched_event.set_event_data(patched_bytes.to_vec());
                Ok(patched_event)
            } else {
                Ok(event)
            }
        })
        .collect()
}

// Parse the input `value` and replace delayed field identifiers with
// corresponding values.
fn replace_ids_with_values<T: Transaction, M: Materializer<T>>(
    value: &TriompheArc<T::Value>,
    layout: &MoveTypeLayout,
    materializer: &M,
) -> Result<T::Value, PanicError> {
    let mut value = (**value).clone();

    if let Some(value_bytes) = value.bytes() {
        let patched_bytes = materializer.replace_ids_with_values(value_bytes, layout)?;
        value.set_bytes(patched_bytes);
        Ok(value)
    } else {
        Err(code_invariant_error(format!(
            "Value to be exchanged doesn't have bytes: {:?}",
            value,
        )))
    }
}

pub(crate) fn update_transaction_on_abort<T, I, O>(
    txn_idx: TxnIndex,
    last_input_output: &TxnLastInputOutput<T, I, O>,
    versioned_cache: &MVHashMap<I::Key, I::Tag, I::Value, DelayedFieldID>,
) where
    T: Transaction,
    I: TxnInput,
    O: TxnOutput<Txn = T, Key = I::Key, Tag = I::Tag>,
{
    counters::SPECULATIVE_ABORT_COUNT.inc();

    // Any logs from the aborted execution should be cleared and not reported.
    clear_speculative_txn_logs(txn_idx as usize);

    // Not valid and successfully aborted, mark the latest write/delta sets as estimates.
    if let Some(keys) = last_input_output.modified_resource_keys(txn_idx) {
        for k in keys {
            versioned_cache.data().mark_estimate(&k, txn_idx);
        }
    }

    // Group metadata lives in same versioned cache as data / resources.
    // We are not marking metadata change as estimate, but after a transaction execution
    // changes metadata, suffix validation is guaranteed to be triggered. Estimation affecting
    // execution behavior is left to size, which uses a heuristic approach.
    last_input_output
        .for_each_resource_group_key_and_tags(txn_idx, |key, tags| {
            versioned_cache
                .group_data()
                .mark_estimate(key, txn_idx, tags);
            Ok(())
        })
        .expect("Passed closure always returns Ok");

    if let Some(keys) = last_input_output.delayed_field_keys(txn_idx) {
        for k in keys {
            versioned_cache.delayed_fields().mark_estimate(&k, txn_idx);
        }
    }
}
