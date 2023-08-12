// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::aggregator_extension::AggregatorID;
use move_binary_format::errors::PartialVMResult;
use move_vm_types::values::{Reference, StructRef, Value};

/// Indices of `value` and `max_value` fields in the `Aggregator` Move
/// struct.
const VALUE_FIELD_INDEX: usize = 0;
const MAX_VALUE_FIELD_INDEX: usize = 1;

/// Given a reference to `Aggregator` Move struct returns a field value at `index`.
pub(crate) fn get_aggregator_field(aggregator: &StructRef, index: usize) -> PartialVMResult<Value> {
    let field_ref = aggregator.borrow_field(index)?.value_as::<Reference>()?;
    field_ref.read_ref()
}

/// Returns ID and a max_value of aggrgegator based on a reference to `Aggregator` Move struct.
pub(crate) fn aggregator_info_u128(
    aggregator: &StructRef,
) -> PartialVMResult<(AggregatorID, u128)> {
    let (value, max_value) = get_aggregator_fields_u128(aggregator)?;
    assert!(
        value <= u64::MAX as u128,
        "identifier in aggregator exceeds u64::MAX"
    );
    Ok((AggregatorID::ephemeral(value as u64), max_value))
}

/// Returns ID and a max_value of aggrgegator based on a reference to `Aggregator` Move struct.
pub(crate) fn aggregator_info_u64(aggregator: &StructRef) -> PartialVMResult<(AggregatorID, u64)> {
    let (value, max_value) = get_aggregator_fields_u64(aggregator)?;
    Ok((AggregatorID::ephemeral(value), max_value))
}

/// Returns ID of aggrgegator snapshot based on a reference to `AggregatorSnapshot` Move struct.
pub(crate) fn aggregator_snapshot_u128_info(
    aggregator_snapshot: &StructRef,
) -> PartialVMResult<u128> {
    let value = get_aggregator_field(aggregator_snapshot, VALUE_FIELD_INDEX)?.value_as::<u128>()?;
    Ok(value)
}

/// Returns ID of aggrgegator snapshot based on a reference to `AggregatorSnapshot` Move struct.
pub(crate) fn aggregator_snapshot_u64_info(
    aggregator_snapshot: &StructRef,
) -> PartialVMResult<u64> {
    let value = get_aggregator_field(aggregator_snapshot, VALUE_FIELD_INDEX)?.value_as::<u64>()?;
    Ok(value)
}

/// Given a reference to `Aggregator` Move struct, returns a tuple of its
/// fields: (`value`, `max_value`).
pub fn get_aggregator_fields_u128(aggregator: &StructRef) -> PartialVMResult<(u128, u128)> {
    let value = get_aggregator_field(aggregator, VALUE_FIELD_INDEX)?.value_as::<u128>()?;
    let max_value = get_aggregator_field(aggregator, MAX_VALUE_FIELD_INDEX)?.value_as::<u128>()?;
    Ok((value, max_value))
}

/// Given a reference to `Aggregator` Move struct, returns a tuple of its
/// fields: (`value`, `max_value`).
pub fn get_aggregator_fields_u64(aggregator: &StructRef) -> PartialVMResult<(u64, u64)> {
    let value = get_aggregator_field(aggregator, VALUE_FIELD_INDEX)?.value_as::<u64>()?;
    let max_value = get_aggregator_field(aggregator, MAX_VALUE_FIELD_INDEX)?.value_as::<u64>()?;
    Ok((value, max_value))
}
