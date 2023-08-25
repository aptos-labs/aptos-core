// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use move_binary_format::errors::PartialVMResult;
use move_vm_types::values::{Reference, StructRef, Value};

const VALUE_FIELD_INDEX: usize = 0;
/// Given a reference to `Aggregator` Move struct returns a field value at `index`.
pub(crate) fn get_aggregator_field(aggregator: &StructRef, index: usize) -> PartialVMResult<Value> {
    let field_ref = aggregator.borrow_field(index)?.value_as::<Reference>()?;
    field_ref.read_ref()
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

pub(crate) fn aggregator_snapshot_string_info(
    aggregator_snapshot: &StructRef,
) -> PartialVMResult<Vec<u8>> {
    let value =
        get_aggregator_field(aggregator_snapshot, VALUE_FIELD_INDEX)?.value_as::<Vec<u8>>()?;
    Ok(value)
}
