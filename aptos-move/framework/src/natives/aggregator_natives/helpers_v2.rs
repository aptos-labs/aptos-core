// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::helpers_v1::set_aggregator_field;
use crate::natives::aggregator_natives::helpers_v1::get_aggregator_field;
use aptos_aggregator::{
    aggregator_extension::extension_error,
    types::{AggregatorID, SnapshotValue},
};
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::vm_status::StatusCode;
use move_vm_types::values::{Struct, StructRef, Value};

/// Indices of `value` and `limit` fields in the `Aggregator` Move
/// struct.
const VALUE_FIELD_INDEX: usize = 0;
const LIMIT_FIELD_INDEX: usize = 1;

pub fn u128_to_u64(value: u128) -> PartialVMResult<u64> {
    u64::try_from(value).map_err(|_| {
        PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
            .with_message("Cannot cast u128 into u64".to_string())
    })
}

pub(crate) fn aggregator_value_field_as_id(value: u128) -> PartialVMResult<AggregatorID> {
    u128_to_u64(value).map(AggregatorID::new)
}

pub(crate) fn aggregator_snapshot_value_field_as_id(
    value: SnapshotValue,
) -> PartialVMResult<AggregatorID> {
    match value {
        SnapshotValue::Integer(v) => aggregator_value_field_as_id(v),
        SnapshotValue::String(v) => Ok(String::from_utf8(v)
            .map_err(|e| {
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                    .with_message(format!("Unable to convert bytes to string: {}", e))
            })?
            .parse::<u64>()
            .map_err(|e| {
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                    .with_message(format!("Unable to parse string as u64: {}", e))
            })?)
        .map(AggregatorID::new),
    }
}

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

pub(crate) fn string_to_bytes(string_value: Struct) -> PartialVMResult<Vec<u8>> {
    string_value.unpack()?.collect::<Vec<Value>>().pop().map_or(
        Err(extension_error("unable to extract string value")),
        |v| v.value_as::<Vec<u8>>(),
    )
}
