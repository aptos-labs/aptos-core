// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::view::{LatestView, ViewState};
use aptos_aggregator::{
    resolver::TDelayedFieldView,
    types::{DelayedFieldValue, ReadPosition},
};
use aptos_mvhashmap::{types::TxnIndex, versioned_delayed_fields::TVersionedDelayedFieldView};
use aptos_types::{
    error::{code_invariant_error, PanicError},
    state_store::{state_value::StateValueMetadata, TStateView},
    transaction::BlockExecutableTransaction as Transaction,
    write_set::TransactionWrite,
};
use bytes::Bytes;
use move_binary_format::errors::PartialVMResult;
use move_core_types::value::{IdentifierMappingKind, MoveTypeLayout};
use move_vm_runtime::AsFunctionValueExtension;
use move_vm_types::{
    delayed_values::delayed_field_id::{DelayedFieldID, ExtractWidth, TryFromMoveValue},
    value_serde::{FunctionValueExtension, ValueSerDeContext, ValueToIdentifierMapping},
    value_traversal::find_identifiers_in_value,
    values::Value,
};
use std::{cell::RefCell, collections::HashSet, sync::Arc};

pub(crate) struct TemporaryValueToIdentifierMapping<'a, T: Transaction, S: TStateView<Key = T::Key>>
{
    latest_view: &'a LatestView<'a, T, S>,
    txn_idx: TxnIndex,
    // These are the delayed field keys that were touched when utilizing this mapping
    // to replace ids with values or values with ids
    delayed_field_ids: RefCell<HashSet<DelayedFieldID>>,
}

impl<'a, T: Transaction, S: TStateView<Key = T::Key>> TemporaryValueToIdentifierMapping<'a, T, S> {
    pub fn new(latest_view: &'a LatestView<'a, T, S>, txn_idx: TxnIndex) -> Self {
        Self {
            latest_view,
            txn_idx,
            delayed_field_ids: RefCell::new(HashSet::new()),
        }
    }

    fn generate_delayed_field_id(&self, width: u32) -> DelayedFieldID {
        self.latest_view.generate_delayed_field_id(width)
    }

    pub fn into_inner(self) -> HashSet<DelayedFieldID> {
        self.delayed_field_ids.into_inner()
    }
}

// For aggregators V2, values are replaced with identifiers at deserialization time,
// and are replaced back when the value is serialized. The "lifted" values are cached
// by the `LatestView` in the aggregators multi-version data structure.
impl<T: Transaction, S: TStateView<Key = T::Key>> ValueToIdentifierMapping
    for TemporaryValueToIdentifierMapping<'_, T, S>
{
    fn value_to_identifier(
        &self,
        kind: &IdentifierMappingKind,
        layout: &MoveTypeLayout,
        value: Value,
    ) -> PartialVMResult<DelayedFieldID> {
        let (base_value, width) = DelayedFieldValue::try_from_move_value(layout, value, kind)?;
        let id = self.generate_delayed_field_id(width);
        match &self.latest_view.latest_view {
            ViewState::Sync(state) => state.set_delayed_field_value(id, base_value),
            ViewState::Unsync(state) => state.set_delayed_field_value(id, base_value),
        };
        self.delayed_field_ids.borrow_mut().insert(id);
        Ok(id)
    }

    fn identifier_to_value(
        &self,
        layout: &MoveTypeLayout,
        identifier: DelayedFieldID,
    ) -> PartialVMResult<Value> {
        self.delayed_field_ids.borrow_mut().insert(identifier);
        let delayed_field = match &self.latest_view.latest_view {
            ViewState::Sync(state) => state
                .versioned_map
                .delayed_fields()
                .read_latest_predicted_value(
                    &identifier,
                    self.txn_idx,
                    ReadPosition::AfterCurrentTxn,
                )
                .expect("Committed value for ID must always exist"),
            ViewState::Unsync(state) => state
                .read_delayed_field(identifier)
                .expect("Delayed field value for ID must always exist in sequential execution"),
        };
        delayed_field.try_into_move_value(layout, identifier.extract_width())
    }
}

impl<T, S> LatestView<'_, T, S>
where
    T: Transaction,
    S: TStateView<Key = T::Key>,
{
    /// Given bytes, where values were already exchanged with identifiers, returns a list of
    /// identifiers present in it.
    fn extract_identifiers_from_value(
        &self,
        bytes: &Bytes,
        layout: &MoveTypeLayout,
    ) -> anyhow::Result<HashSet<DelayedFieldID>> {
        // TODO[agg_v2](optimize): this performs 2 traversals of a value:
        //   1) deserialize,
        //   2) find identifiers to populate the set.
        //   See if can cache identifiers in advance, or combine it with
        //   deserialization.
        let function_value_extension = self.as_function_value_extension();
        let value = ValueSerDeContext::new(function_value_extension.max_value_nest_depth())
            .with_func_args_deserialization(&function_value_extension)
            .with_delayed_fields_serde()
            .deserialize(bytes, layout)
            .ok_or_else(|| {
                anyhow::anyhow!("Failed to deserialize resource during id replacement")
            })?;

        let mut identifiers = HashSet::new();
        find_identifiers_in_value(&value, &mut identifiers)?;
        // TODO[agg_v2](cleanup): ugly way of converting delayed ids to generic type params.
        Ok(identifiers.into_iter().map(DelayedFieldID::from).collect())
    }

    // Deletion returns a PanicError.
    pub(crate) fn does_value_need_exchange(
        &self,
        value: &T::Value,
        layout: &MoveTypeLayout,
        delayed_write_set_ids: &HashSet<DelayedFieldID>,
    ) -> Result<bool, PanicError> {
        if let Some(bytes) = value.bytes() {
            self.extract_identifiers_from_value(bytes, layout)
                .map(|identifiers_in_read| !delayed_write_set_ids.is_disjoint(&identifiers_in_read))
                .map_err(|e| {
                    code_invariant_error(format!("Identifier extraction failed with {:?}", e))
                })
        } else {
            // Deletion returns an error.
            Err(code_invariant_error(
                "Delete shouldn't be in values considered for exchange",
            ))
        }
    }

    // Exclude deletions, and values that do not contain any delayed field IDs that were written to.
    pub(crate) fn filter_value_for_exchange(
        &self,
        value: &T::Value,
        layout: &Arc<MoveTypeLayout>,
        delayed_write_set_ids: &HashSet<DelayedFieldID>,
        key: &T::Key,
    ) -> Option<Result<(T::Key, (StateValueMetadata, u64, Arc<MoveTypeLayout>)), PanicError>> {
        if value.is_deletion() {
            None
        } else {
            self.does_value_need_exchange(value, layout, delayed_write_set_ids)
                .map_or_else(
                    |e| Some(Err(e)),
                    |needs_exchange| {
                        needs_exchange.then(|| {
                            Ok((
                                key.clone(),
                                (
                                    value.as_state_value_metadata().unwrap().clone(),
                                    value.write_op_size().write_len().unwrap(),
                                    layout.clone(),
                                ),
                            ))
                        })
                    },
                )
        }
    }
}
