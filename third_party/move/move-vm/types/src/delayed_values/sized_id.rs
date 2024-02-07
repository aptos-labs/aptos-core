// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    delayed_values::{
        bcs_utils,
        derived_string_snapshot::{
            bytes_and_width_to_derived_string_struct, derived_string_struct_to_bytes_and_length,
            from_utf8_bytes, is_derived_string_struct_layout, u128_to_u64,
            u64_to_fixed_size_utf8_bytes,
        },
    },
    values::{Struct, Value},
};
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{value::MoveTypeLayout, vm_status::StatusCode};

/// Represents a unique 32-bit identifier used for values which also stores their
/// serialized size (u32::MAX at most). Can be stored as a single 64-bit unsigned
/// integer.
/// TODO[agg_v2](cleanup): consolidate DelayedFiledID and this implementation!
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct SizedID {
    // Unique identifier for a value.
    id: u32,
    // Exact number of bytes a serialized value will take.
    serialized_size: u32,
}

const NUM_BITS_FOR_SERIALIZED_SIZE: usize = 32;

impl SizedID {
    pub fn new(id: u32, serialized_size: u32) -> Self {
        Self {
            id,
            serialized_size,
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn serialized_size(&self) -> u32 {
        self.serialized_size
    }

    pub fn into_derived_string_struct(self) -> PartialVMResult<Value> {
        let width = self.serialized_size() as usize;

        // we need to create DerivedString struct that serializes to exactly match given `width`.
        // I.e: size_u32_as_uleb128(value.len()) + value.len() + size_u32_as_uleb128(padding.len()) + padding.len() == width
        // As padding has a fixed allowed max width, it is easiest to expand value to have the padding be minimal.
        // We cannot always make padding to be 0 byte vector (serialized into 1 byte) - as not all sizes are possible
        // for string due to variable encoding of string length.

        // So we will over-estimate the serialized length of the value a bit.
        let value_len_width_upper_bound = bcs_utils::size_u32_as_uleb128(width - 2); // we subtract 2 because uleb sizes (for both value and padding fields) are at least 1 byte.

        // If we don't even have enough space to store the length of the value, we cannot proceed
        if width <= value_len_width_upper_bound + 1 {
            return Err(PartialVMError::new(StatusCode::DELAYED_FIELDS_CODE_INVARIANT_ERROR).with_message(format!(
                "DerivedStringSnapshot size issue for id {self:?}: width: {width}, value_width_upper_bound: {value_len_width_upper_bound}"
            )));
        }

        let id_as_string = u64_to_fixed_size_utf8_bytes(
            self.into(),
            // fill the string representation to leave 1 byte for padding and upper bound for it's own length serialization.
            width - value_len_width_upper_bound - 1,
        )?;

        bytes_and_width_to_derived_string_struct(id_as_string, width)
    }
}

impl From<u64> for SizedID {
    fn from(value: u64) -> Self {
        let id = value >> NUM_BITS_FOR_SERIALIZED_SIZE;
        let serialized_size = value & ((1u64 << NUM_BITS_FOR_SERIALIZED_SIZE) - 1);
        Self {
            id: id as u32,
            serialized_size: serialized_size as u32,
        }
    }
}

impl From<SizedID> for u64 {
    fn from(sized_id: SizedID) -> Self {
        let id = (sized_id.id as u64) << NUM_BITS_FOR_SERIALIZED_SIZE;
        id | sized_id.serialized_size as u64
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
    ) -> Result<(Self, u32), Self::Error>;
}

impl TryIntoMoveValue for SizedID {
    type Error = PartialVMError;

    fn try_into_move_value(self, layout: &MoveTypeLayout) -> Result<Value, Self::Error> {
        Ok(match layout {
            MoveTypeLayout::U64 => Value::u64(self.into()),
            MoveTypeLayout::U128 => {
                let v: u64 = self.into();
                Value::u128(v as u128)
            },
            layout if is_derived_string_struct_layout(layout) => {
                // Here, we make sure we convert identifiers to fixed-size Move
                // values. This is needed because we charge gas based on the resource
                // size with identifiers inside, and so it has to be deterministic.

                self.into_derived_string_struct()?
            },
            _ => {
                return Err(
                    PartialVMError::new(StatusCode::DELAYED_FIELDS_CODE_INVARIANT_ERROR)
                        .with_message(format!(
                            "Failed to convert {:?} into a Move value with {} layout",
                            self, layout
                        )),
                )
            },
        })
    }
}

impl TryFromMoveValue for SizedID {
    type Error = PartialVMError;
    type Hint = ();

    fn try_from_move_value(
        layout: &MoveTypeLayout,
        value: Value,
        hint: &Self::Hint,
    ) -> Result<(Self, u32), Self::Error> {
        // Since we put the value there, we should be able to read it back,
        // unless there is a bug in the code - so we expect_ok() throughout.
        let (id, width) = match layout {
            MoveTypeLayout::U64 => (value.value_as::<u64>().map(Self::from)?, 8),
            MoveTypeLayout::U128 => (
                value.value_as::<u128>().and_then(u128_to_u64).map(Self::from)?,
                16,
            ),
            layout if is_derived_string_struct_layout(layout) => {
                let (bytes, width) = value
                    .value_as::<Struct>()
                    .and_then(derived_string_struct_to_bytes_and_length)
                    .map_err(|e| {
                        PartialVMError::new(StatusCode::DELAYED_FIELDS_CODE_INVARIANT_ERROR).with_message(format!(
                            "couldn't extract derived string struct: {:?}",
                            e
                        ))
                    })?;
                let id = from_utf8_bytes::<u64>(bytes).map(Self::from)?;
                (id, width)
            },
            // We use value to ID conversion in serialization.
            _ => {
                return Err(PartialVMError::new(StatusCode::DELAYED_FIELDS_CODE_INVARIANT_ERROR).with_message(format!(
                    "Failed to convert a Move value with {layout} layout into an identifier, tagged with {hint:?}, with value {value:?}",
                )))
            },
        };
        if id.serialized_size() != width {
            return Err(
                PartialVMError::new(StatusCode::DELAYED_FIELDS_CODE_INVARIANT_ERROR).with_message(
                    format!(
                "Extracted identifier has a wrong width: id={id:?}, width={width}, expected={}",
                id.serialized_size(),
            ),
                ),
            );
        }

        Ok((id, width))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    macro_rules! assert_sized_id_roundtrip {
        ($start_value:expr) => {
            let sized_id: SizedID = $start_value.into();
            let end_value: u64 = sized_id.into();
            assert_eq!($start_value, end_value)
        };
    }

    #[test]
    fn test_sized_id_from_u64() {
        assert_sized_id_roundtrip!(0u64);
        assert_sized_id_roundtrip!(123456789u64);
        assert_sized_id_roundtrip!(u64::MAX);
    }
}
