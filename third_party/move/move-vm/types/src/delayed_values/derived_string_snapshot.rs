// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    delayed_values::error::{code_invariant_error, expect_ok},
    values::{Struct, Value},
};
use move_binary_format::{
    errors::PartialVMResult,
    file_format_common::{bcs_size_of_byte_array, size_u32_as_uleb128},
};
use move_core_types::value::MoveTypeLayout;
use std::str::FromStr;

fn is_string_layout(layout: &MoveTypeLayout) -> bool {
    use MoveTypeLayout as L;
    if let L::Struct(move_struct) = layout {
        if let [L::Vector(elem)] = move_struct.fields(None).iter().as_slice() {
            if let L::U8 = elem.as_ref() {
                return true;
            }
        }
    }
    false
}

pub fn is_derived_string_struct_layout(layout: &MoveTypeLayout) -> bool {
    use MoveTypeLayout as L;
    if let L::Struct(move_struct) = layout {
        if let [value_field, L::Vector(padding_elem)] = move_struct.fields(None).iter().as_slice() {
            if is_string_layout(value_field) {
                if let L::U8 = padding_elem.as_ref() {
                    return true;
                }
            }
        }
    }
    false
}

pub fn to_utf8_bytes(value: impl ToString) -> Vec<u8> {
    value.to_string().into_bytes()
}

pub fn u128_to_u64(value: u128) -> PartialVMResult<u64> {
    u64::try_from(value).map_err(|_| code_invariant_error("Cannot cast u128 into u64".to_string()))
}

pub fn from_utf8_bytes<T: FromStr>(bytes: Vec<u8>) -> PartialVMResult<T> {
    String::from_utf8(bytes)
        .map_err(|e| code_invariant_error(format!("Unable to convert bytes to string: {}", e)))?
        .parse::<T>()
        .map_err(|_| code_invariant_error("Unable to parse string".to_string()))
}

pub fn u64_to_fixed_size_utf8_bytes(value: u64, length: usize) -> PartialVMResult<Vec<u8>> {
    let result = format!("{:0>width$}", value, width = length)
        .to_string()
        .into_bytes();
    if result.len() != length {
        return Err(code_invariant_error(format!(
            "u64_to_fixed_size_utf8_bytes: width mismatch: value: {value}, length: {length}, result: {result:?}"
        )));
    }
    Ok(result)
}

pub fn bytes_to_string(bytes: Vec<u8>) -> Value {
    Value::struct_(Struct::pack(vec![Value::vector_u8(bytes)]))
}

pub fn bytes_and_width_to_derived_string_struct(
    bytes: Vec<u8>,
    width: usize,
) -> PartialVMResult<Value> {
    // We need to create DerivedStringSnapshot struct that serializes to exactly match given `width`.

    let value_width = bcs_size_of_byte_array(bytes.len());
    // padding field takes at list 1 byte (empty vector)
    if value_width + 1 > width {
        return Err(code_invariant_error(format!(
            "DerivedStringSnapshot size issue: no space left for padding: value_width: {value_width}, width: {width}"
        )));
    }

    // We assume/assert that padding never exceeds length that requires more than 1 byte for size:
    // (otherwise it complicates the logic to fill until the exact width, as padding can never be serialized into 129 bytes
    // (vec[0; 127] serializes into 128 bytes, and vec[0; 128] serializes into 130 bytes))
    let padding_len = width - value_width - 1;
    if size_u32_as_uleb128(padding_len) > 1 {
        return Err(code_invariant_error(format!(
            "DerivedStringSnapshot size issue: padding expected to be too large: value_width: {value_width}, width: {width}, padding_len: {padding_len}"
        )));
    }

    Ok(Value::struct_(Struct::pack(vec![
        bytes_to_string(bytes),
        Value::vector_u8(vec![0; padding_len]),
    ])))
}

pub fn string_to_bytes(value: Struct) -> PartialVMResult<Vec<u8>> {
    expect_ok(value.unpack())?
        .collect::<Vec<Value>>()
        .pop()
        .map_or_else(
            || Err(code_invariant_error("Unable to extract bytes from String")),
            |v| expect_ok(v.value_as::<Vec<u8>>()),
        )
}

pub fn derived_string_struct_to_bytes_and_length(value: Struct) -> PartialVMResult<(Vec<u8>, u32)> {
    let mut fields = value.unpack()?.collect::<Vec<Value>>();
    if fields.len() != 2 {
        return Err(code_invariant_error(format!(
            "DerivedStringSnapshot has wrong number of fields: {:?}",
            fields.len()
        )));
    }
    let padding = fields.pop().unwrap().value_as::<Vec<u8>>()?;
    let value = fields.pop().unwrap();
    let string_bytes = string_to_bytes(value.value_as::<Struct>()?)?;
    let string_len = string_bytes.len();
    Ok((
        string_bytes,
        u32::try_from(bcs_size_of_byte_array(string_len) + bcs_size_of_byte_array(padding.len()))
            .map_err(|_| {
            code_invariant_error(format!(
                "DerivedStringSnapshot size exceeds u32: string_len: {string_len}, padding_len: {}",
                padding.len()
            ))
        })?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_ok, assert_ok_eq};

    #[test]
    fn test_int_to_string_fails_on_small_width() {
        assert_err!(u64_to_fixed_size_utf8_bytes(1000, 1));
    }

    #[test]
    fn test_fixed_string_id_1() {
        let encoded = assert_ok!(u64_to_fixed_size_utf8_bytes(7, 30));
        assert_eq!(encoded.len(), 30);

        let decoded_string = assert_ok!(String::from_utf8(encoded.clone()));
        assert_eq!(decoded_string, "000000000000000000000000000007");

        let decoded = assert_ok!(decoded_string.parse::<u64>());
        assert_eq!(decoded, 7);
        assert_ok_eq!(from_utf8_bytes::<u64>(encoded), 7);
    }

    #[test]
    fn test_fixed_string_id_2() {
        let encoded = assert_ok!(u64_to_fixed_size_utf8_bytes(u64::MAX, 20));
        assert_eq!(encoded.len(), 20);

        let decoded_string = assert_ok!(String::from_utf8(encoded.clone()));
        assert_eq!(decoded_string, "18446744073709551615");

        let decoded = assert_ok!(decoded_string.parse::<u64>());
        assert_eq!(decoded, u64::MAX);
        assert_ok_eq!(from_utf8_bytes::<u64>(encoded), u64::MAX);
    }

    #[test]
    fn test_fixed_string_id_3() {
        let encoded = assert_ok!(u64_to_fixed_size_utf8_bytes(0, 20));
        assert_eq!(encoded.len(), 20);

        let decoded_string = assert_ok!(String::from_utf8(encoded.clone()));
        assert_eq!(decoded_string, "00000000000000000000");

        let decoded = assert_ok!(decoded_string.parse::<u64>());
        assert_eq!(decoded, 0);
        assert_ok_eq!(from_utf8_bytes::<u64>(encoded), 0);
    }
}
