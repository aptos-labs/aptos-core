// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::view::{LatestView, ViewState};
use aptos_aggregator::{
    resolver::TDelayedFieldView,
    types::{code_invariant_error, DelayedFieldValue, ReadPosition},
};
use aptos_mvhashmap::{types::TxnIndex, versioned_delayed_fields::TVersionedDelayedFieldView};
use aptos_types::{
    delayed_fields::PanicError,
    executable::Executable,
    state_store::{state_value::StateValueMetadata, TStateView},
    transaction::BlockExecutableTransaction as Transaction,
    write_set::TransactionWrite,
};
use bytes::Bytes;
use move_binary_format::errors::PartialVMResult;
use move_core_types::value::{IdentifierMappingKind, MoveTypeLayout};
use move_vm_types::{
    delayed_values::delayed_field_id::{ExtractWidth, TryFromMoveValue},
    value_serde::{deserialize_and_allow_delayed_values, ValueToIdentifierMapping},
    value_traversal::find_identifiers_in_value,
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
    delayed_field_ids: RefCell<HashSet<T::Identifier>>,
}

impl<'a, T: Transaction, S: TStateView<Key = T::Key>, X: Executable>
    TemporaryValueToIdentifierMapping<'a, T, S, X>
{
    pub fn new(latest_view: &'a LatestView<'a, T, S, X>, txn_idx: TxnIndex) -> Self {
        Self {
            latest_view,
            txn_idx,
            delayed_field_ids: RefCell::new(HashSet::new()),
        }
    }

    fn generate_delayed_field_id(&self, width: u32) -> T::Identifier {
        self.latest_view.generate_delayed_field_id(width)
    }

    pub fn into_inner(self) -> HashSet<T::Identifier> {
        self.delayed_field_ids.into_inner()
    }
}

// For aggregators V2, values are replaced with identifiers at deserialization time,
// and are replaced back when the value is serialized. The "lifted" values are cached
// by the `LatestView` in the aggregators multi-version data structure.
impl<'a, T: Transaction, S: TStateView<Key = T::Key>, X: Executable> ValueToIdentifierMapping
    for TemporaryValueToIdentifierMapping<'a, T, S, X>
{
    type Identifier = T::Identifier;

    fn value_to_identifier(
        &self,
        kind: &IdentifierMappingKind,
        layout: &MoveTypeLayout,
        value: Value,
    ) -> PartialVMResult<Self::Identifier> {
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
        identifier: Self::Identifier,
    ) -> PartialVMResult<Value> {
        self.delayed_field_ids.borrow_mut().insert(identifier);
        let delayed_field = match &self.latest_view.latest_view {
            ViewState::Sync(state) => state
                .versioned_map
                .delayed_fields()
                .read_latest_committed_value(
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

// Given bytes, where values were already exchanged with identifiers,
// return a list of identifiers present in it.
fn extract_identifiers_from_value<T: Transaction>(
    bytes: &Bytes,
    layout: &MoveTypeLayout,
) -> anyhow::Result<HashSet<T::Identifier>> {
    // TODO[agg_v2](optimize): this performs 2 traversals of a value:
    //   1) deserialize,
    //   2) find identifiers to populate the set.
    //   See if can cache identifiers in advance, or combine it with
    //   deserialization.
    let value = deserialize_and_allow_delayed_values(bytes, layout)
        .ok_or_else(|| anyhow::anyhow!("Failed to deserialize resource during id replacement"))?;

    let mut identifiers = HashSet::new();
    find_identifiers_in_value(&value, &mut identifiers)?;
    // TODO[agg_v2](cleanup): ugly way of converting delayed ids to generic type params.
    Ok(identifiers.into_iter().map(T::Identifier::from).collect())
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
            .map_err(|e| code_invariant_error(format!("Identifier extraction failed with {:?}", e)))
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
