// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{value::MoveTypeLayout, vm_status::StatusCode};
use move_vm_types::values::{Struct, Value};
use std::str::FromStr;

/// Returns true if the type layout corresponds to a String, which should be a
/// struct with a single byte vector field.
pub(crate) fn is_string_layout(layout: &MoveTypeLayout) -> bool {
    use MoveTypeLayout::*;
    if let Struct(move_struct) = layout {
        if let [Vector(elem)] = move_struct.fields().iter().as_slice() {
            if let U8 = elem.as_ref() {
                return true;
            }
        }
    }
    false
}

pub fn bytes_to_string(bytes: Vec<u8>) -> Value {
    Value::struct_(Struct::pack(vec![Value::vector_u8(bytes)]))
}

pub fn string_to_bytes(value: Struct) -> PartialVMResult<Vec<u8>> {
    value.unpack()?.collect::<Vec<Value>>().pop().map_or_else(
        || {
            Err(PartialVMError::new(StatusCode::VM_EXTENSION_ERROR)
                .with_message("Unable to extract bytes from String".to_string()))
        },
        |v| v.value_as::<Vec<u8>>(),
    )
}

pub fn to_utf8_bytes(value: impl ToString) -> Vec<u8> {
    value.to_string().into_bytes()
}

pub fn from_utf8_bytes<T: FromStr>(bytes: Vec<u8>) -> PartialVMResult<T> {
    String::from_utf8(bytes)
        .map_err(|e| {
            PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                .with_message(format!("Unable to convert bytes to string: {}", e))
        })?
        .parse::<T>()
        .map_err(|_| {
            PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                .with_message("Unable to parse string".to_string())
        })
}

pub fn u128_to_u64(value: u128) -> PartialVMResult<u64> {
    u64::try_from(value).map_err(|_| {
        PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
            .with_message("Cannot cast u128 into u64".to_string())
    })
}
