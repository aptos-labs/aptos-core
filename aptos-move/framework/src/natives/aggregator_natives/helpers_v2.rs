// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::aggregator_extension::AggregatorID;
use move_binary_format::errors::PartialVMResult;
use move_vm_types::values::{Reference, StructRef, Value};
    

/// Indices of `value` and `limit` fields in the `Aggregator` Move
/// struct.
const VALUE_FIELD_INDEX: usize = 0;
const LIMIT_FIELD_INDEX: usize = 1;

/// Given a reference to `Aggregator` Move struct returns a field value at `index`.
pub(crate) fn get_aggregator_field(aggregator: &StructRef, index: usize) -> PartialVMResult<Value> {
    let field_ref = aggregator.borrow_field(index)?.value_as::<Reference>()?;
    field_ref.read_ref()
}

/// Returns ID and a limit of aggrgegator based on a reference to `Aggregator` Move struct.
pub(crate) fn aggregator_info(aggregator: &StructRef) -> PartialVMResult<(AggregatorID, u128)> {
    let (value, limit) = get_aggregator_fields(aggregator)?;
    assert!(
        value < u64::MAX as u128,
        "identifier in aggregator exceeds u64::MAX"
    );
    Ok((AggregatorID::ephemeral(value as u64), limit))
}

/// Returns ID of aggrgegator snapshot based on a reference to `AggregatorSnapshot` Move struct.
pub(crate) fn aggregator_snapshot_u128_info(aggregator_snapshot: &StructRef) -> PartialVMResult<u128> {
    let value = get_aggregator_field(aggregator_snapshot, VALUE_FIELD_INDEX)?.value_as::<u128>()?;
    Ok(value)
}

/// Returns ID of aggrgegator snapshot based on a reference to `AggregatorSnapshot` Move struct.
pub(crate) fn aggregator_snapshot_u64_info(aggregator_snapshot: &StructRef) -> PartialVMResult<u64> {
    let value = get_aggregator_field(aggregator_snapshot, VALUE_FIELD_INDEX)?.value_as::<u64>()?;
    Ok(value)
}

/// Given a reference to `Aggregator` Move struct, returns a tuple of its
/// fields: (`value`, `limit`).
pub fn get_aggregator_fields(aggregator: &StructRef) -> PartialVMResult<(u128, u128)> {
    let value = get_aggregator_field(aggregator, VALUE_FIELD_INDEX)?.value_as::<u128>()?;
    let limit = get_aggregator_field(aggregator, LIMIT_FIELD_INDEX)?.value_as::<u128>()?;
    Ok((value, limit))
}