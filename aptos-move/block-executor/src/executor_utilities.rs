// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{counters, errors::*, record::Record, records::Records, view::LatestView};
use aptos_logger::error;
use aptos_mvhashmap::{
    types::{MVValue, TxnIndex, ValueWithLayout},
    MVHashMap,
};
use aptos_types::{
    error::{code_invariant_error, PanicError},
    state_store::TStateView,
    transaction::BlockExecutableTransaction as Transaction,
    write_set::TransactionWrite,
};
use aptos_vm_logging::{alert, clear_speculative_txn_logs, prelude::*};
use aptos_vm_types::resolver::ResourceGroupSize;
use bytes::Bytes;
use fail::fail_point;
use move_core_types::value::MoveTypeLayout;
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use rand::{thread_rng, Rng};
use std::collections::{BTreeMap, HashMap};
use triomphe::Arc as TriompheArc;

pub(crate) fn map_finalized_group<T: Transaction>(
    group_key: T::Key,
    finalized_group: Vec<(T::Tag, ValueWithLayout<T::Value>)>,
    group_size: ResourceGroupSize,
    metadata_op: ValueWithLayout<T::Value>,
    is_read_needing_exchange: bool,
) -> Result<
    (
        T::Key,
        ValueWithLayout<T::Value>,
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

pub(crate) fn serialize_groups<T: Transaction>(
    finalized_groups: Vec<(
        T::Key,
        T::Value,
        Vec<(T::Tag, TriompheArc<T::Value>)>,
        ResourceGroupSize,
    )>,
) -> Result<HashMap<T::Key, Bytes>, ResourceGroupSerializationError> {
    fail_point!(
        "fail-point-resource-group-serialization",
        !finalized_groups.is_empty(),
        |_| Err(ResourceGroupSerializationError)
    );

    finalized_groups
        .into_iter()
        .map(|(group_key, metadata_op, finalized_group, group_size)| {
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
                        Ok((group_key, group_bytes.into()))
                    }
                },
                Err(e) => {
                    alert!("Unexpected resource group error {:?}", e);
                    Err(ResourceGroupSerializationError)
                },
            }
        })
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

pub(crate) fn map_id_to_values_in_group_writes<
    T: Transaction,
    S: TStateView<Key = T::Key> + Sync,
>(
    finalized_groups: Vec<(
        T::Key,
        ValueWithLayout<T::Value>,
        Vec<(T::Tag, ValueWithLayout<T::Value>)>,
        ResourceGroupSize,
    )>,
    latest_view: &LatestView<T, S>,
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
                    replace_ids_with_values(&value, layout.as_ref(), latest_view)?,
                ),
            };
            patched_resource_vec.push((tag, value));
        }
        // Group metadata carries no layout, so it needs no id replacement. This is
        // the single point where group writes leave the multi-version representation.
        patched_finalized_groups.push((
            group_key,
            group_metadata_op.into_value(),
            patched_resource_vec,
            group_size,
        ));
    }
    Ok(patched_finalized_groups)
}

// Parse the input `value` and replace delayed field identifiers with corresponding values
fn replace_ids_with_values<T: Transaction, S: TStateView<Key = T::Key> + Sync>(
    value: &TriompheArc<T::Value>,
    layout: &MoveTypeLayout,
    latest_view: &LatestView<T, S>,
) -> Result<T::Value, PanicError> {
    let mut value = (**value).clone();

    if let Some(value_bytes) = value.bytes() {
        let patched_bytes = latest_view
            .replace_identifiers_with_values(value_bytes, layout)
            .map_err(|_| {
                code_invariant_error(format!(
                    "Failed to replace identifiers with values in a resource {:?}",
                    layout
                ))
            })?
            .0;
        value.set_bytes(patched_bytes);
        Ok(value)
    } else {
        Err(code_invariant_error(format!(
            "Value to be exchanged doesn't have bytes: {:?}",
            value,
        )))
    }
}

pub(crate) fn update_transaction_on_abort<T, R>(
    txn_idx: TxnIndex,
    last_input_output: &Records<T, R>,
    versioned_cache: &MVHashMap<T::Key, T::Tag, T::SpeculativeValue, DelayedFieldID>,
) where
    T: Transaction,
    R: Record<Txn = T>,
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
