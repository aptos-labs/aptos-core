// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::view::{LatestView, ViewState};
use aptos_aggregator::{
    resolver::TDelayedFieldView,
    types::{
        code_invariant_error, DelayedFieldValue, PanicError, ReadPosition, TryFromMoveValue,
        TryIntoMoveValue,
    },
};
use aptos_mvhashmap::{types::TxnIndex, versioned_delayed_fields::TVersionedDelayedFieldView};
use aptos_types::{
    executable::Executable,
    state_store::{state_value::StateValueMetadata, TStateView},
    transaction::BlockExecutableTransaction as Transaction,
    write_set::TransactionWrite,
};
use bytes::Bytes;
use move_core_types::value::{IdentifierMappingKind, MoveTypeLayout};
use move_vm_types::{
    value_transformation::{
        deserialize_and_replace_values_with_ids, TransformationError, TransformationResult,
        ValueToIdentifierMapping,
    },
    values::Value,
};
use std::{cell::RefCell, collections::HashSet, sync::Arc};

pub(crate) struct TemporaryValueToIdentifierMapping<
    'a,
    T: Transaction,
    S: TStateView<Key = T::Key>,
    X: Executable,
> {
    latest_view: &'a LatestView<'a, T, S, X>,
    txn_idx: TxnIndex,
    // These are the delayed field keys that were touched when utilizing this mapping
    // to replace ids with values or values with ids
    delayed_field_keys: RefCell<HashSet<T::Identifier>>,
}

impl<'a, T: Transaction, S: TStateView<Key = T::Key>, X: Executable>
    TemporaryValueToIdentifierMapping<'a, T, S, X>
{
    pub fn new(latest_view: &'a LatestView<'a, T, S, X>, txn_idx: TxnIndex) -> Self {
        Self {
            latest_view,
            txn_idx,
            delayed_field_keys: RefCell::new(HashSet::new()),
        }
    }

    fn generate_delayed_field_id(&self, width: u32) -> T::Identifier {
        self.latest_view.generate_delayed_field_id(width)
    }

    pub fn into_inner(self) -> HashSet<T::Identifier> {
        self.delayed_field_keys.into_inner()
    }
}

// For aggregators V2, values are replaced with identifiers at deserialization time,
// and are replaced back when the value is serialized. The "lifted" values are cached
// by the `LatestView` in the aggregators multi-version data structure.
impl<'a, T: Transaction, S: TStateView<Key = T::Key>, X: Executable> ValueToIdentifierMapping
    for TemporaryValueToIdentifierMapping<'a, T, S, X>
{
    fn value_to_identifier(
        &self,
        kind: &IdentifierMappingKind,
        layout: &MoveTypeLayout,
        value: Value,
    ) -> TransformationResult<Value> {
        let (base_value, width) = DelayedFieldValue::try_from_move_value(layout, value, kind)?;
        let id = self.generate_delayed_field_id(width);
        match &self.latest_view.latest_view {
            ViewState::Sync(state) => state.set_delayed_field_value(id, base_value),
            ViewState::Unsync(state) => {
                state.set_delayed_field_value(id, base_value);
            },
        };
        self.delayed_field_keys.borrow_mut().insert(id);
        id.try_into_move_value(layout)
            .map_err(|e| TransformationError(format!("{:?}", e)))
    }

    fn identifier_to_value(
        &self,
        layout: &MoveTypeLayout,
        identifier_value: Value,
    ) -> TransformationResult<Value> {
        let (id, width) = T::Identifier::try_from_move_value(layout, identifier_value, &())
            .map_err(|e| TransformationError(format!("{:?}", e)))?;
        self.delayed_field_keys.borrow_mut().insert(id);
        Ok(match &self.latest_view.latest_view {
            ViewState::Sync(state) => state
                .versioned_map
                .delayed_fields()
                .read_latest_committed_value(&id, self.txn_idx, ReadPosition::AfterCurrentTxn)
                .expect("Committed value for ID must always exist"),
            ViewState::Unsync(state) => state
                .read_delayed_field(id)
                .expect("Delayed field value for ID must always exist in sequential execution"),
        }
        .try_into_move_value(layout, width)?)
    }
}

struct TemporaryExtractIdentifiersMapping<T: Transaction> {
    // These are the delayed field keys that were touched when utilizing this mapping
    // to replace ids with values or values with ids
    delayed_field_keys: RefCell<HashSet<T::Identifier>>,
}

impl<T: Transaction> TemporaryExtractIdentifiersMapping<T> {
    pub fn new() -> Self {
        Self {
            delayed_field_keys: RefCell::new(HashSet::new()),
        }
    }

    pub fn into_inner(self) -> HashSet<T::Identifier> {
        self.delayed_field_keys.into_inner()
    }
}

impl<T: Transaction> ValueToIdentifierMapping for TemporaryExtractIdentifiersMapping<T> {
    fn value_to_identifier(
        &self,
        _kind: &IdentifierMappingKind,
        layout: &MoveTypeLayout,
        value: Value,
    ) -> TransformationResult<Value> {
        let (id, _) = T::Identifier::try_from_move_value(layout, value, &())
            .map_err(|e| TransformationError(format!("{:?}", e)))?;
        self.delayed_field_keys.borrow_mut().insert(id);
        id.try_into_move_value(layout)
            .map_err(|e| TransformationError(format!("{:?}", e)))
    }

    fn identifier_to_value(
        &self,
        layout: &MoveTypeLayout,
        identifier_value: Value,
    ) -> TransformationResult<Value> {
        let (id, _) = T::Identifier::try_from_move_value(layout, identifier_value, &())
            .map_err(|e| TransformationError(format!("{:?}", e)))?;
        self.delayed_field_keys.borrow_mut().insert(id);
        id.try_into_move_value(layout)
            .map_err(|e| TransformationError(format!("{:?}", e)))
    }
}

// Given a Bytes, where values were already exchanged with identifiers,
// return a list of identifiers present in it.
// TODO[agg_v2](cleanup): store list of identifiers with the exchanged value (like layout),
// and remove this method.
fn extract_identifiers_from_value<T: Transaction>(
    bytes: &Bytes,
    layout: &MoveTypeLayout,
) -> Result<HashSet<T::Identifier>, PanicError> {
    let mapping = TemporaryExtractIdentifiersMapping::<T>::new();
    // TODO[agg_v2](cleanup) rename deserialize_and_replace_values_with_ids to not be specific
    // to mapping trait implementation.
    let _patched_value = deserialize_and_replace_values_with_ids(bytes.as_ref(), layout, &mapping)
        .ok_or_else(|| {
            code_invariant_error("Failed to deserialize a value to extract identifiers")
        })?;
    Ok(mapping.into_inner())
}

// Deletion returns a PanicError.
pub(crate) fn does_value_need_exchange<T: Transaction>(
    value: &T::Value,
    layout: &MoveTypeLayout,
    delayed_write_set_ids: &HashSet<T::Identifier>,
) -> Result<bool, PanicError> {
    if let Some(bytes) = value.bytes() {
        extract_identifiers_from_value::<T>(bytes, layout)
            .map(|identifiers_in_read| !delayed_write_set_ids.is_disjoint(&identifiers_in_read))
    } else {
        // Deletion returns an error.
        Err(code_invariant_error(
            "Delete shouldn't be in values considered for exchange",
        ))
    }
}

// Exclude deletions, and values that do not contain any delayed field IDs that were written to.
pub(crate) fn filter_value_for_exchange<T: Transaction>(
    value: &T::Value,
    layout: &Arc<MoveTypeLayout>,
    delayed_write_set_ids: &HashSet<T::Identifier>,
    key: &T::Key,
) -> Option<Result<(T::Key, (StateValueMetadata, u64, Arc<MoveTypeLayout>)), PanicError>> {
    if value.is_deletion() {
        None
    } else {
        does_value_need_exchange::<T>(value, layout, delayed_write_set_ids).map_or_else(
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
