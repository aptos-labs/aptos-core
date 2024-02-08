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

    pub fn into_derived_string_struct(self) -> PartialVMResult<Value> {
        let width = self.extract_width() as usize;

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

pub trait ExtractUniqueIndex: Sized {
    fn extract_unique_index(&self) -> u32;
}

impl ExtractUniqueIndex for DelayedFieldID {
    fn extract_unique_index(&self) -> u32 {
        self.unique_index
    }
}

pub trait ExtractWidth: Sized {
    fn extract_width(&self) -> u32;
}

impl ExtractWidth for DelayedFieldID {
    fn extract_width(&self) -> u32 {
        self.width
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

impl TryIntoMoveValue for DelayedFieldID {
    type Error = PartialVMError;

    fn try_into_move_value(self, layout: &MoveTypeLayout) -> Result<Value, Self::Error> {
        Ok(match layout {
            MoveTypeLayout::U64 => Value::u64(self.as_u64()),
            MoveTypeLayout::U128 => {
                let v: u64 = self.as_u64();
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

impl TryFromMoveValue for DelayedFieldID {
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
        if id.extract_width() != width {
            return Err(
                PartialVMError::new(StatusCode::DELAYED_FIELDS_CODE_INVARIANT_ERROR).with_message(
                    format!(
                "Extracted identifier has a wrong width: id={id:?}, width={width}, expected={}",
                id.extract_width(),
            ),
                ),
            );
        }

        Ok((id, width))
    }
}
