// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::errors::PartialVMResult;
use move_vm_types::{
    natives::function::{PartialVMError, StatusCode},
    values::{Reference, Struct, StructRef, Value},
};

/// Indices of `value` and `limit` fields in the `Aggregator` Move
/// struct.
const VALUE_FIELD_INDEX: usize = 0;
const LIMIT_FIELD_INDEX: usize = 1;

/// Given a reference to `Aggregator` Move struct, returns a tuple of its
/// fields: (`value`, `limit`).
pub fn get_aggregator_fields_u128(aggregator: &StructRef) -> PartialVMResult<(u128, u128)> {
    let value = get_aggregator_field(aggregator, VALUE_FIELD_INDEX)?.value_as::<u128>()?;
    let limit = get_aggregator_field(aggregator, LIMIT_FIELD_INDEX)?.value_as::<u128>()?;
    Ok((value, limit))
}

pub fn set_aggregator_value_field(aggregator: &StructRef, value: Value) -> PartialVMResult<()> {
    set_aggregator_field(aggregator, VALUE_FIELD_INDEX, value)
}

/// Given a reference to `Aggregator` Move struct, returns a tuple of its
/// fields: (`value`, `limit`).
pub fn get_aggregator_fields_u64(aggregator: &StructRef) -> PartialVMResult<(u64, u64)> {
    let value = get_aggregator_field(aggregator, VALUE_FIELD_INDEX)?.value_as::<u64>()?;
    let limit = get_aggregator_field(aggregator, LIMIT_FIELD_INDEX)?.value_as::<u64>()?;
    Ok((value, limit))
}

/// Returns ID of aggregator snapshot based on a reference to `AggregatorSnapshot` Move struct.
pub(crate) fn aggregator_snapshot_field_value(
    aggregator_snapshot: &StructRef,
) -> PartialVMResult<Value> {
    get_aggregator_field(aggregator_snapshot, VALUE_FIELD_INDEX)
}

// ================= START TEMPORARY CODE =================
// TODO: aggregator_v2 branch will introduce these in different places in code

/// Given a reference to `Aggregator` Move struct returns a field value at `index`.
pub(crate) fn get_aggregator_field(aggregator: &StructRef, index: usize) -> PartialVMResult<Value> {
    let field_ref = aggregator.borrow_field(index)?.value_as::<Reference>()?;
    field_ref.read_ref()
}

/// Given a reference to `Aggregator` Move struct, updates a field value at `index`.
pub(crate) fn set_aggregator_field(
    aggregator: &StructRef,
    index: usize,
    value: Value,
) -> PartialVMResult<()> {
    let field_ref = aggregator.borrow_field(index)?.value_as::<Reference>()?;
    field_ref.write_ref(value)
}

pub fn string_to_bytes(value: Struct) -> PartialVMResult<Vec<u8>> {
    value.unpack()?.collect::<Vec<Value>>().pop().map_or(
        Err(PartialVMError::new(StatusCode::VM_EXTENSION_ERROR)
            .with_message("Unable to extract bytes from String".to_string())),
        |v| v.value_as::<Vec<u8>>(),
    )
}

pub fn to_utf8_bytes(value: impl ToString) -> Vec<u8> {
    value.to_string().into_bytes()
}

pub fn u128_to_u64(value: u128) -> PartialVMResult<u64> {
    u64::try_from(value).map_err(|_| {
        PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
            .with_message("Cannot cast u128 into u64".to_string())
    })
}

// ================= END TEMPORARY CODE =================
