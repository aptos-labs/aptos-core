// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{errors::*, view::LatestView};
use aptos_logger::error;
use aptos_mvhashmap::types::ValueWithLayout;
use aptos_types::{
    contract_event::TransactionEvent,
    error::{code_invariant_error, PanicError},
    state_store::TStateView,
    transaction::BlockExecutableTransaction as Transaction,
    write_set::TransactionWrite,
};
use aptos_vm_logging::{alert, prelude::*};
use aptos_vm_types::resolver::ResourceGroupSize;
use bytes::Bytes;
use fail::fail_point;
use move_core_types::value::MoveTypeLayout;
use rand::{thread_rng, Rng};
use std::{collections::BTreeMap, sync::Arc};

// TODO(clean-up): refactor & replace these macros with functions for code clarity. Currently
// not possible due to type & API mismatch.
macro_rules! groups_to_finalize {
    ($outputs:expr, $($txn_idx:expr),*) => {{
	let group_write_ops = $outputs.resource_group_metadata_ops($($txn_idx),*);

        group_write_ops.into_iter()
            .map(|val| (val, false))
            .chain([()].into_iter().flat_map(|_| {
		// Lazily evaluated only after iterating over group_write_ops.
                $outputs.group_reads_needing_delayed_field_exchange($($txn_idx),*)
                    .into_iter()
                    .map(|(key, metadata)|
			 ((key, TransactionWrite::from_state_value(
			     Some(StateValue::new_with_metadata(Bytes::new(), metadata)))), true))
            }))
    }};
}

// Selects and prepares resource writes that require ID replacement for delayed fields.
// - reads needing replacement: returns the error if data is not in Exchanged format.
// - normal resource writes: select writes that have layout set and are not a deletion.
//
// Since reads needing exchange also do not contain deletions (see 'does_value_need_exchange')
// logic in value_exchange.rs, it is guaranteed that no returned values is a deletion.
macro_rules! resource_writes_to_materialize {
    ($writes:expr, $outputs:expr, $data_source:expr, $($txn_idx:expr),*) => {{
        $outputs
            .reads_needing_delayed_field_exchange($($txn_idx),*)
            .into_iter()
            .map(|(key, metadata, layout)| {
                match $data_source.fetch_exchanged_data(&key, $($txn_idx),*) {
                    Ok((value, existing_layout)) => {
                        randomly_check_layout_matches(
                            Some(&existing_layout),
                            Some(layout.as_ref()),
                        )?;
                        let new_value = Arc::new(TransactionWrite::from_state_value(Some(
                            StateValue::new_with_metadata(
                                value.bytes().cloned().unwrap_or_else(Bytes::new), metadata)
                        )));
                        Ok((key, new_value, layout))
                    },
                    Err(e) => Err(e),
                }
            }).chain(
                $writes.into_iter().filter_map(|(key, value, maybe_layout)| {
                    // layout is Some(_) if it contains a delayed field
                    if let Some(layout) = maybe_layout {
                        // No need to exchange anything if a resource with delayed field is deleted.
                        if !value.is_deletion() {
                            return Some(Ok((key, value, layout)))
                        }
                    }
                    None
                })
            ).collect::<std::result::Result<Vec<_>, _>>()
    }};
}

pub(crate) use groups_to_finalize;
pub(crate) use resource_writes_to_materialize;

pub(crate) fn map_finalized_group<T: Transaction>(
    group_key: T::Key,
    finalized_group: anyhow::Result<(Vec<(T::Tag, ValueWithLayout<T::Value>)>, ResourceGroupSize)>,
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

    match finalized_group {
        Ok((finalized_group, group_size)) => {
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
        },
        Err(e) => Err(code_invariant_error(format!(
            "Error committing resource group {:?}",
            e
        ))),
    }
}

pub(crate) fn serialize_groups<T: Transaction>(
    finalized_groups: Vec<(
        T::Key,
        T::Value,
        Vec<(T::Tag, Arc<T::Value>)>,
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

pub(crate) fn map_id_to_values_in_group_writes<
    T: Transaction,
    S: TStateView<Key = T::Key> + Sync,
>(
    finalized_groups: Vec<(
        T::Key,
        T::Value,
        Vec<(T::Tag, ValueWithLayout<T::Value>)>,
        ResourceGroupSize,
    )>,
    latest_view: &LatestView<T, S>,
) -> Result<
    Vec<(
        T::Key,
        T::Value,
        Vec<(T::Tag, Arc<T::Value>)>,
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
                ValueWithLayout::Exchanged(value, Some(layout)) => Arc::new(
                    replace_ids_with_values(&value, layout.as_ref(), latest_view)?,
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
pub(crate) fn map_id_to_values_in_write_set<T: Transaction, S: TStateView<Key = T::Key> + Sync>(
    resource_write_set: Vec<(T::Key, Arc<T::Value>, Arc<MoveTypeLayout>)>,
    latest_view: &LatestView<T, S>,
) -> Result<Vec<(T::Key, T::Value)>, PanicError> {
    resource_write_set
        .into_iter()
        .map(|(key, write_op, layout)| {
            Ok::<_, PanicError>((
                key,
                replace_ids_with_values(&write_op, &layout, latest_view)?,
            ))
        })
        .collect::<std::result::Result<_, PanicError>>()
}

// For each delayed field in the event, replace delayed field identifier with value.
pub(crate) fn map_id_to_values_events<T: Transaction, S: TStateView<Key = T::Key> + Sync>(
    events: Box<dyn Iterator<Item = (T::Event, Option<MoveTypeLayout>)>>,
    latest_view: &LatestView<T, S>,
) -> Result<Vec<T::Event>, PanicError> {
    events
        .map(|(event, layout)| {
            if let Some(layout) = layout {
                let event_data = event.get_event_data();
                latest_view
                    .replace_identifiers_with_values(&Bytes::from(event_data.to_vec()), &layout)
                    .map(|(bytes, _)| {
                        let mut patched_event = event;
                        patched_event.set_event_data(bytes.to_vec());
                        patched_event
                    })
                    .map_err(|_| {
                        code_invariant_error(format!(
                            "Failed to replace identifiers with values in an event {:?}",
                            layout
                        ))
                    })
            } else {
                Ok(event)
            }
        })
        .collect::<Result<Vec<_>, PanicError>>()
}

// Parse the input `value` and replace delayed field identifiers with corresponding values
fn replace_ids_with_values<T: Transaction, S: TStateView<Key = T::Key> + Sync>(
    value: &Arc<T::Value>,
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
