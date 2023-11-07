// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::errors::PartialVMError;
use move_core_types::{value::MoveTypeLayout, vm_status::StatusCode};
use move_vm_types::values::{Struct, Value};
use std::str::FromStr;

/// Ephemeral identifier type used by delayed fields (aggregators, snapshots)
/// during execution.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct DelayedFieldID(u64);

impl DelayedFieldID {
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

// Used for ID generation from u32/u64 counters.
impl From<u64> for DelayedFieldID {
    fn from(value: u64) -> Self {
        Self::new(value)
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
    ) -> Result<Self, Self::Error>;
}

impl TryIntoMoveValue for DelayedFieldID {
    type Error = PanicError;

    fn try_into_move_value(self, layout: &MoveTypeLayout) -> Result<Value, Self::Error> {
        Ok(match layout {
            MoveTypeLayout::U64 => Value::u64(self.as_u64()),
            MoveTypeLayout::U128 => Value::u128(self.as_u64() as u128),
            layout if is_string_layout(layout) => {
                // Here, we make sure we convert identifiers to fixed-size Move
                // values. This is needed because we charge gas based on the resource
                // size with identifiers inside, and so it has to be deterministic.
                bytes_to_string(u64_to_fixed_size_utf8_bytes(self.as_u64()))
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
        _hint: &Self::Hint,
    ) -> Result<Self, Self::Error> {
        // Since we put the value there, we should be able to read it back,
        // unless there is a bug in the code - so we expect_ok() throughout.
        match layout {
            MoveTypeLayout::U64 => expect_ok(value.value_as::<u64>()),
            MoveTypeLayout::U128 => expect_ok(value.value_as::<u128>()).and_then(u128_to_u64),
            layout if is_string_layout(layout) => expect_ok(value.value_as::<Struct>())
                .and_then(string_to_bytes)
                .and_then(from_utf8_bytes),
            // We use value to ID conversion in serialization.
            _ => Err(code_invariant_error(format!(
                "Failed to convert a Move value with {} layout into an identifier",
                layout
            ))),
        }
        .map(Self::new)
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

fn bytes_to_string(bytes: Vec<u8>) -> Value {
    Value::struct_(Struct::pack(vec![Value::vector_u8(bytes)]))
}

fn string_to_bytes(value: Struct) -> Result<Vec<u8>, PanicError> {
    expect_ok(value.unpack())?
        .collect::<Vec<Value>>()
        .pop()
        .map_or_else(
            || Err(code_invariant_error("Unable to extract bytes from String")),
            |v| expect_ok(v.value_as::<Vec<u8>>()),
        )
}

fn u64_to_fixed_size_utf8_bytes(value: u64) -> Vec<u8> {
    // Maximum u64 identifier size is 20 characters. We need a fixed size to
    // ensure identifiers have the same size all the time for all validators,
    // to ensure consistent and deterministic gas charging.
    format!("{:0>20}", value).to_string().into_bytes()
}

fn from_utf8_bytes<T: FromStr>(bytes: Vec<u8>) -> Result<T, PanicError> {
    String::from_utf8(bytes)
        .map_err(|e| code_invariant_error(format!("Unable to convert bytes to string: {}", e)))?
        .parse::<T>()
        .map_err(|_| code_invariant_error("Unable to parse string".to_string()))
}

fn u128_to_u64(value: u128) -> Result<u64, PanicError> {
    u64::try_from(value).map_err(|_| code_invariant_error("Cannot cast u128 into u64".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_ok, assert_ok_eq};

    #[test]
    fn test_fixed_string_id_1() {
        let encoded = u64_to_fixed_size_utf8_bytes(7);
        assert_eq!(encoded.len(), 20);

        let decoded_string = assert_ok!(String::from_utf8(encoded.clone()));
        assert_eq!(decoded_string, "00000000000000000007");

        let decoded = assert_ok!(decoded_string.parse::<u64>());
        assert_eq!(decoded, 7);
        assert_ok_eq!(from_utf8_bytes::<u64>(encoded), 7);
    }

    #[test]
    fn test_fixed_string_id_2() {
        let encoded = u64_to_fixed_size_utf8_bytes(u64::MAX);
        assert_eq!(encoded.len(), 20);

        let decoded_string = assert_ok!(String::from_utf8(encoded.clone()));
        assert_eq!(decoded_string, "18446744073709551615");

        let decoded = assert_ok!(decoded_string.parse::<u64>());
        assert_eq!(decoded, u64::MAX);
        assert_ok_eq!(from_utf8_bytes::<u64>(encoded), u64::MAX);
    }

    #[test]
    fn test_fixed_string_id_3() {
        let encoded = u64_to_fixed_size_utf8_bytes(0);
        assert_eq!(encoded.len(), 20);

        let decoded_string = assert_ok!(String::from_utf8(encoded.clone()));
        assert_eq!(decoded_string, "00000000000000000000");

        let decoded = assert_ok!(decoded_string.parse::<u64>());
        assert_eq!(decoded, 0);
        assert_ok_eq!(from_utf8_bytes::<u64>(encoded), 0);
    }
}
