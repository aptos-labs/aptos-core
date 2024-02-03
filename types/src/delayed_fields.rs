// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::serde_helper::bcs_utils::{bcs_size_of_byte_array, size_u32_as_uleb128};
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{value::MoveTypeLayout, vm_status::StatusCode};
use move_vm_types::values::{Struct, Value};
use once_cell::sync::Lazy;
use std::str::FromStr;

const BITS_FOR_SIZE: usize = 32;

/// Ephemeral identifier type used by delayed fields (aggregators, snapshots)
/// during execution.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct DelayedFieldID {
    unique_index: u32,
    // Exact number of bytes serialized delayed field will take.
    width: u32,
}

impl DelayedFieldID {
    pub fn new_with_width(unique_index: u32, width: u32) -> Self {
        Self {
            unique_index,
            width,
        }
    }

    pub fn new_for_test_for_u64(unique_index: u32) -> Self {
        Self::new_with_width(unique_index, 8)
    }

    pub fn as_u64(&self) -> u64 {
        ((self.unique_index as u64) << BITS_FOR_SIZE) | self.width as u64
    }

    pub fn extract_width(&self) -> u32 {
        self.width
    }

    pub fn into_derived_string_struct(self) -> Result<Value, PanicError> {
        let width = self.extract_width() as usize;

        // we need to create DerivedString struct that serializes to exactly match given `width`.
        // I.e: size_u32_as_uleb128(value.len()) + value.len() + size_u32_as_uleb128(padding.len()) + padding.len() == width
        // As padding has a fixed allowed max width, it is easiest to expand value to have the padding be minimal.
        // We cannot always make padding to be 0 byte vector (serialized into 1 byte) - as not all sizes are possible
        // for string due to variable encoding of string length.

        // So we will over-estimate the serialized length of the value a bit.
        let value_len_width_upper_bound = size_u32_as_uleb128(width - 2); // we subtract 2 because uleb sizes (for both value and padding fields) are at least 1 byte.

        // If we don't even have enough space to store the length of the value, we cannot proceed
        if width <= value_len_width_upper_bound + 1 {
            return Err(code_invariant_error(format!(
                "DerivedStringSnapshot size issue for id {self:?}: width: {width}, value_width_upper_bound: {value_len_width_upper_bound}"
            )));
        }

        let id_as_string = u64_to_fixed_size_utf8_bytes(
            self.as_u64(),
            // fill the string representation to leave 1 byte for padding and upper bound for it's own length serialization.
            width - value_len_width_upper_bound - 1,
        )?;

        bytes_and_width_to_derived_string_struct(id_as_string, width)
    }
}

// Used for ID generation from exchanged value/exchanges serialized value.
impl From<u64> for DelayedFieldID {
    fn from(value: u64) -> Self {
        Self {
            unique_index: u32::try_from(value >> BITS_FOR_SIZE).unwrap(),
            width: u32::try_from(value & ((1u64 << BITS_FOR_SIZE) - 1)).unwrap(),
        }
    }
}

// Used for ID generation from u32 counter with width.
impl From<(u32, u32)> for DelayedFieldID {
    fn from(value: (u32, u32)) -> Self {
        let (index, width) = value;
        Self::new_with_width(index, width)
    }
}

// Represents something that should never happen - i.e. a code invariant error,
// which we would generally just panic, but since we are inside of the VM,
// we cannot do that.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PanicError {
    CodeInvariantError(String),
}

impl ToString for PanicError {
    fn to_string(&self) -> String {
        match self {
            PanicError::CodeInvariantError(e) => e.clone(),
        }
    }
}

impl From<PanicError> for PartialVMError {
    fn from(err: PanicError) -> Self {
        match err {
            PanicError::CodeInvariantError(msg) => {
                PartialVMError::new(StatusCode::DELAYED_FIELDS_CODE_INVARIANT_ERROR)
                    .with_message(msg)
            },
        }
    }
}

pub trait ExtractUniqueIndex: Sized {
    fn extract_unique_index(&self) -> u32;
}

/// Types which implement this trait can be converted to a Move value.
pub trait TryIntoMoveValue: Sized {
    type Error: std::fmt::Debug;

    fn try_into_move_value(self, layout: &MoveTypeLayout) -> Result<Value, Self::Error>;
}

/// Types which implement this trait can be constructed from a Move value.
pub trait TryFromMoveValue: Sized {
    // Allows to pass extra information from the caller.
    type Hint;
    type Error: std::fmt::Debug;

    fn try_from_move_value(
        layout: &MoveTypeLayout,
        value: Value,
        hint: &Self::Hint,
    ) -> Result<(Self, u32), Self::Error>;
}

impl ExtractUniqueIndex for DelayedFieldID {
    fn extract_unique_index(&self) -> u32 {
        self.unique_index
    }
}

impl TryIntoMoveValue for DelayedFieldID {
    type Error = PanicError;

    fn try_into_move_value(self, layout: &MoveTypeLayout) -> Result<Value, Self::Error> {
        Ok(match layout {
            MoveTypeLayout::U64 => Value::u64(self.as_u64()),
            MoveTypeLayout::U128 => Value::u128(self.as_u64() as u128),
            layout if is_derived_string_struct_layout(layout) => {
                // Here, we make sure we convert identifiers to fixed-size Move
                // values. This is needed because we charge gas based on the resource
                // size with identifiers inside, and so it has to be deterministic.

                self.into_derived_string_struct()?
            },
            _ => {
                return Err(code_invariant_error(format!(
                    "Failed to convert {:?} into a Move value with {} layout",
                    self, layout
                )))
            },
        })
    }
}

impl TryFromMoveValue for DelayedFieldID {
    type Error = PanicError;
    type Hint = ();

    fn try_from_move_value(
        layout: &MoveTypeLayout,
        value: Value,
        hint: &Self::Hint,
    ) -> Result<(Self, u32), Self::Error> {
        // Since we put the value there, we should be able to read it back,
        // unless there is a bug in the code - so we expect_ok() throughout.
        let (id, width) = match layout {
            MoveTypeLayout::U64 => (expect_ok(value.value_as::<u64>()).map(Self::from)?, 8),
            MoveTypeLayout::U128 => (
                expect_ok(value.value_as::<u128>()).and_then(u128_to_u64).map(Self::from)?,
                16,
            ),
            layout if is_derived_string_struct_layout(layout) => {
                let (bytes, width) = value
                    .value_as::<Struct>()
                    .and_then(derived_string_struct_to_bytes_and_length)
                    .map_err(|e| {
                        code_invariant_error(format!(
                            "couldn't extract derived string struct: {:?}",
                            e
                        ))
                    })?;
                let id = from_utf8_bytes::<u64>(bytes).map(Self::from)?;
                (id, width)
            },
            // We use value to ID conversion in serialization.
            _ => {
                return Err(code_invariant_error(format!(
                    "Failed to convert a Move value with {layout} layout into an identifier, tagged with {hint:?}, with value {value:?}",
                )))
            },
        };
        if id.extract_width() != width {
            return Err(code_invariant_error(format!(
                "Extracted identifier has a wrong width: id={id:?}, width={width}, expected={}",
                id.extract_width(),
            )));
        }

        Ok((id, width))
    }
}

fn code_invariant_error<M: std::fmt::Debug>(message: M) -> PanicError {
    let msg = format!(
        "Delayed logic code invariant broken (there is a bug in the code), {:?}",
        message
    );
    println!("ERROR: {}", msg);
    // cannot link aptos_logger in aptos-types crate
    // error!("{}", msg);
    PanicError::CodeInvariantError(msg)
}

fn expect_ok<V, E: std::fmt::Debug>(value: Result<V, E>) -> Result<V, PanicError> {
    value.map_err(code_invariant_error)
}

/// Returns true if the type layout corresponds to a String, which should be a
/// struct with a single byte vector field.
fn is_string_layout(layout: &MoveTypeLayout) -> bool {
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

pub fn is_derived_string_struct_layout(layout: &MoveTypeLayout) -> bool {
    use MoveTypeLayout::*;
    if let Struct(move_struct) = layout {
        if let [value_field, Vector(padding_elem)] = move_struct.fields().iter().as_slice() {
            if is_string_layout(value_field) {
                if let U8 = padding_elem.as_ref() {
                    return true;
                }
            }
        }
    }
    false
}

pub fn bytes_to_string(bytes: Vec<u8>) -> Value {
    Value::struct_(Struct::pack(vec![Value::vector_u8(bytes)]))
}

pub fn string_to_bytes(value: Struct) -> Result<Vec<u8>, PanicError> {
    expect_ok(value.unpack())?
        .collect::<Vec<Value>>()
        .pop()
        .map_or_else(
            || Err(code_invariant_error("Unable to extract bytes from String")),
            |v| expect_ok(v.value_as::<Vec<u8>>()),
        )
}

pub fn bytes_and_width_to_derived_string_struct(
    bytes: Vec<u8>,
    width: usize,
) -> Result<Value, PanicError> {
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

pub fn u64_to_fixed_size_utf8_bytes(value: u64, length: usize) -> Result<Vec<u8>, PanicError> {
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

static U64_MAX_DIGITS: Lazy<usize> = Lazy::new(|| u64::MAX.to_string().len());
static U128_MAX_DIGITS: Lazy<usize> = Lazy::new(|| u128::MAX.to_string().len());

pub fn to_utf8_bytes(value: impl ToString) -> Vec<u8> {
    value.to_string().into_bytes()
}

pub fn from_utf8_bytes<T: FromStr>(bytes: Vec<u8>) -> Result<T, PanicError> {
    String::from_utf8(bytes)
        .map_err(|e| code_invariant_error(format!("Unable to convert bytes to string: {}", e)))?
        .parse::<T>()
        .map_err(|_| code_invariant_error("Unable to parse string".to_string()))
}

pub fn derived_string_struct_to_bytes_and_length(value: Struct) -> PartialVMResult<(Vec<u8>, u32)> {
    let mut fields = value.unpack()?.collect::<Vec<Value>>();
    if fields.len() != 2 {
        return Err(
            PartialVMError::new(StatusCode::DELAYED_FIELDS_CODE_INVARIANT_ERROR).with_message(
                format!(
                    "DerivedStringSnapshot has wrong number of fields: {:?}",
                    fields.len()
                ),
            ),
        );
    }
    let padding = fields.pop().unwrap().value_as::<Vec<u8>>()?;
    let value = fields.pop().unwrap();
    let string_bytes = string_to_bytes(value.value_as::<Struct>()?)?;
    let string_len = string_bytes.len();
    Ok((
        string_bytes,
        u32::try_from(bcs_size_of_byte_array(string_len) + bcs_size_of_byte_array(padding.len()))
            .map_err(|_| {
            PartialVMError::new(StatusCode::DELAYED_FIELDS_CODE_INVARIANT_ERROR).with_message(
                format!(
                "DerivedStringSnapshot size exceeds u32: string_len: {string_len}, padding_len: {}",
                padding.len()
            ),
            )
        })?,
    ))
}

pub fn u128_to_u64(value: u128) -> Result<u64, PanicError> {
    u64::try_from(value).map_err(|_| code_invariant_error("Cannot cast u128 into u64".to_string()))
}

pub fn calculate_width_for_constant_string(byte_len: usize) -> usize {
    // we need to be able to store it both raw, as well as when it is exchanged with u64 DelayedFieldID.
    // so the width needs to be larger of the two options
    (bcs_size_of_byte_array(byte_len) + 1) // 1 is for empty padding serialized length
        .max(*U64_MAX_DIGITS + 2) // largest exchanged u64 DelayedFieldID is u64 max digits, plus 1 for each of the value and padding serialized length
}

pub fn calculate_width_for_integer_embeded_string(
    rest_byte_len: usize,
    snapshot_id: DelayedFieldID,
) -> Result<usize, PanicError> {
    // we need to translate byte width into string character width.
    let max_snapshot_string_width = match snapshot_id.extract_width() {
        8 => *U64_MAX_DIGITS,
        16 => *U128_MAX_DIGITS,
        x => {
            return Err(code_invariant_error(format!(
                "unexpected width ({x}) for integer snapshot id: {snapshot_id:?}"
            )))
        },
    };

    Ok(bcs_size_of_byte_array(rest_byte_len + max_snapshot_string_width) + 1) // 1 for padding length
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SnapshotToStringFormula {
    Concat { prefix: Vec<u8>, suffix: Vec<u8> },
}

impl SnapshotToStringFormula {
    pub fn apply_to(&self, base: u128) -> Vec<u8> {
        match self {
            SnapshotToStringFormula::Concat { prefix, suffix } => {
                let middle_string = base.to_string();
                let middle = middle_string.as_bytes();
                let mut result = Vec::with_capacity(prefix.len() + middle.len() + suffix.len());
                result.extend(prefix);
                result.extend(middle);
                result.extend(suffix);
                result
            },
        }
    }
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
    fn test_width_calculation() {
        for data in [
            vec![],
            vec![60; 1],
            vec![60; 5],
            vec![60; 127],
            vec![60; 128],
            vec![60; 129],
        ] {
            {
                let width = calculate_width_for_constant_string(data.len());
                assert_ok!(bytes_and_width_to_derived_string_struct(
                    data.clone(),
                    width
                ));
                assert_ok!(DelayedFieldID::new_with_width(u32::MAX, width as u32)
                    .into_derived_string_struct());
            }
            {
                let width = assert_ok!(calculate_width_for_integer_embeded_string(
                    data.len(),
                    DelayedFieldID::new_with_width(u32::MAX, 8)
                ));
                assert_ok!(bytes_and_width_to_derived_string_struct(
                    SnapshotToStringFormula::Concat {
                        prefix: data.clone(),
                        suffix: vec![]
                    }
                    .apply_to(u64::MAX as u128),
                    width
                ));
                assert_ok!(DelayedFieldID::new_with_width(u32::MAX, width as u32)
                    .into_derived_string_struct());
            }
            {
                let width = assert_ok!(calculate_width_for_integer_embeded_string(
                    data.len(),
                    DelayedFieldID::new_with_width(u32::MAX, 16)
                ));
                assert_ok!(bytes_and_width_to_derived_string_struct(
                    SnapshotToStringFormula::Concat {
                        prefix: data.clone(),
                        suffix: vec![]
                    }
                    .apply_to(u128::MAX),
                    width
                ));
                assert_ok!(DelayedFieldID::new_with_width(u32::MAX, width as u32)
                    .into_derived_string_struct());
            }
        }
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
