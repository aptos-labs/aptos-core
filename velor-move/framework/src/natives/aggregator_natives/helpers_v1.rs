// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_aggregator::aggregator_v1_extension::{extension_error, AggregatorID};
use velor_types::{account_address::AccountAddress, state_store::table::TableHandle};
use move_binary_format::errors::PartialVMResult;
use move_vm_types::values::{Reference, Struct, StructRef, Value};

/// The index of the `phantom_table` field in the `AggregatorFactory` Move
/// struct.
const PHANTOM_TABLE_FIELD_INDEX: usize = 0;

/// The index of the `handle` field in the `Table` Move struct.
const TABLE_HANDLE_FIELD_INDEX: usize = 0;

/// Indices of `handle`, `key` and `limit` fields in the `Aggregator` Move
/// struct.
const HANDLE_FIELD_INDEX: usize = 0;
const KEY_FIELD_INDEX: usize = 1;
const LIMIT_FIELD_INDEX: usize = 2;

/// Given a reference to `AggregatorFactory` Move struct, returns the value of
/// `handle` field (from underlying `Table` struct).
pub(crate) fn get_handle(aggregator_table: &StructRef) -> PartialVMResult<TableHandle> {
    Ok(TableHandle(
        aggregator_table
            .borrow_field(PHANTOM_TABLE_FIELD_INDEX)?
            .value_as::<StructRef>()?
            .borrow_field(TABLE_HANDLE_FIELD_INDEX)?
            .value_as::<Reference>()?
            .read_ref()?
            .value_as::<AccountAddress>()?,
    ))
}

/// Given a reference to `Aggregator` Move struct returns a field value at `index`.
pub(crate) fn get_struct_field(value: &StructRef, index: usize) -> PartialVMResult<Value> {
    let field_ref = value.borrow_field(index)?.value_as::<Reference>()?;
    field_ref.read_ref()
}

/// Returns ID and a limit of aggregator based on a reference to `Aggregator` Move struct.
pub(crate) fn aggregator_info(aggregator: &StructRef) -> PartialVMResult<(AggregatorID, u128)> {
    let handle = get_struct_field(aggregator, HANDLE_FIELD_INDEX)?.value_as::<AccountAddress>()?;
    let key = get_struct_field(aggregator, KEY_FIELD_INDEX)?.value_as::<AccountAddress>()?;
    let limit = get_struct_field(aggregator, LIMIT_FIELD_INDEX)?.value_as::<u128>()?;
    Ok((AggregatorID::new(TableHandle(handle), key), limit))
}

/// Given an `Aggregator` Move struct, unpacks it into fields: (`handle`, `key`, `limit`).
pub(crate) fn unpack_aggregator_struct(
    aggregator_struct: Struct,
) -> PartialVMResult<(TableHandle, AccountAddress, u128)> {
    let mut fields: Vec<Value> = aggregator_struct.unpack()?.collect();
    assert!(fields.len() == 3);

    let pop_with_err = |vec: &mut Vec<Value>, msg: &str| {
        vec.pop()
            .map_or_else(|| Err(extension_error(msg)), |v| v.value_as::<u128>())
    };

    let limit = pop_with_err(&mut fields, "unable to pop 'limit' field")?;
    let key = fields.pop().map_or_else(
        || Err(extension_error("unable to pop `handle` field")),
        |v| v.value_as::<AccountAddress>(),
    )?;
    let handle = fields.pop().map_or_else(
        || Err(extension_error("unable to pop `handle` field")),
        |v| v.value_as::<AccountAddress>(),
    )?;
    Ok((TableHandle(handle), key, limit))
}
