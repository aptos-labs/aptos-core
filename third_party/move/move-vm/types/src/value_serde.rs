// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::values::{
    Container, DeserializationSeed, SerializationReadyValue, SizedID, Value, ValueImpl,
};
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{
    value::{IdentifierMappingKind, MoveTypeLayout},
    vm_status::StatusCode,
};
use serde::{
    de::{DeserializeSeed, Error as DeError},
    ser::Error as SerError,
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::{collections::HashSet, hash::Hash};
use std::str::FromStr;
use crate::values::Struct;

pub trait CustomDeserialize {
    fn custom_deserialize<'d, D: Deserializer<'d>>(
        &self,
        deserializer: D,
        tag: &IdentifierMappingKind,
        layout: &MoveTypeLayout,
    ) -> Result<Value, D::Error>;
}

pub trait CustomSerialize {
    fn custom_serialize<S: Serializer>(
        &self,
        serializer: S,
        tag: &IdentifierMappingKind,
        layout: &MoveTypeLayout,
        sized_id: SizedID,
    ) -> Result<S::Ok, S::Error>;
}

// FIXME: consolidate
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

fn is_derived_string_struct_layout(layout: &MoveTypeLayout) -> bool {
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

fn size_u32_as_uleb128(mut value: usize) -> usize {
    let mut len = 1;
    while value >= 0x80 {
        // 7 (lowest) bits of data get written in a single byte.
        len += 1;
        value >>= 7;
    }
    len
}

fn u64_to_fixed_size_utf8_bytes(value: u64, length: usize) -> Option<Vec<u8>> {
    let result = format!("{:0>width$}", value, width = length)
        .to_string()
        .into_bytes();
    if result.len() != length {
        None
    } else {
        Some(result)
    }
}

fn bcs_size_of_byte_array(length: usize) -> usize {
    size_u32_as_uleb128(length) + length
}

fn bytes_and_width_to_derived_string_struct(
    bytes: Vec<u8>,
    width: usize,
) -> Option<Value> {
    // We need to create DerivedStringSnapshot struct that serializes to exactly match given `width`.

    let value_width = bcs_size_of_byte_array(bytes.len());
    // padding field takes at list 1 byte (empty vector)
    if value_width + 1 > width {
        return None
    }

    // We assume/assert that padding never exceeds length that requires more than 1 byte for size:
    // (otherwise it complicates the logic to fill until the exact width, as padding can never be serialized into 129 bytes
    // (vec[0; 127] serializes into 128 bytes, and vec[0; 128] serializes into 130 bytes))
    let padding_len = width - value_width - 1;
    if size_u32_as_uleb128(padding_len) > 1 {
        return None
    }

    Some(Value::struct_(Struct::pack(vec![
        bytes_to_string(bytes),
        Value::vector_u8(vec![0; padding_len]),
    ])))
}

fn bytes_to_string(bytes: Vec<u8>) -> Value {
    Value::struct_(Struct::pack(vec![Value::vector_u8(bytes)]))
}

fn into_derived_string_struct(sized_id: SizedID) -> Option<Value> {
    let width = sized_id.serialized_size() as usize;

    // we need to create DerivedString struct that serializes to exactly match given `width`.
    // I.e: size_u32_as_uleb128(value.len()) + value.len() + size_u32_as_uleb128(padding.len()) + padding.len() == width
    // As padding has a fixed allowed max width, it is easiest to expand value to have the padding be minimal.
    // We cannot always make padding to be 0 byte vector (serialized into 1 byte) - as not all sizes are possible
    // for string due to variable encoding of string length.

    // So we will over-estimate the serialized length of the value a bit.
    let value_len_width_upper_bound = size_u32_as_uleb128(width - 2); // we subtract 2 because uleb sizes (for both value and padding fields) are at least 1 byte.

    // If we don't even have enough space to store the length of the value, we cannot proceed
    if width <= value_len_width_upper_bound + 1 {
        return None
    }

    let id_as_string = u64_to_fixed_size_utf8_bytes(
        sized_id.into(),
        // fill the string representation to leave 1 byte for padding and upper bound for it's own length serialization.
        width - value_len_width_upper_bound - 1,
    )?;

    bytes_and_width_to_derived_string_struct(id_as_string, width)
}

fn string_to_bytes(value: Struct) -> Option<Vec<u8>> {
    value.unpack().ok()?
        .collect::<Vec<Value>>()
        .pop()?
        .value_as::<Vec<u8>>().ok()
}

fn derived_string_struct_to_bytes_and_length(value: Struct) -> Option<(Vec<u8>, u32)> {
    let mut fields = value.unpack().ok()?.collect::<Vec<Value>>();
    if fields.len() != 2 {
        return None;
    }
    let padding = fields.pop().unwrap().value_as::<Vec<u8>>().ok()?;
    let value = fields.pop().unwrap();
    let string_bytes = string_to_bytes(value.value_as::<Struct>().ok()?)?;
    let string_len = string_bytes.len();
    Some((
        string_bytes,
        u32::try_from(bcs_size_of_byte_array(string_len) + bcs_size_of_byte_array(padding.len())).ok()?,
    ))
}

fn from_utf8_bytes<T: FromStr>(bytes: Vec<u8>) -> Option<T> {
    String::from_utf8(bytes).ok()?
        .parse::<T>().ok()
}

pub struct NativeValueSimpleSerDe;

impl CustomDeserialize for NativeValueSimpleSerDe {
    fn custom_deserialize<'d, D: Deserializer<'d>>(
        &self,
        deserializer: D,
        _tag: &IdentifierMappingKind,
        layout: &MoveTypeLayout,
    ) -> Result<Value, D::Error> {
        let encoded = match layout {
            // FIXME Add checks!
            MoveTypeLayout::U64 => u64::deserialize(deserializer)?,
            MoveTypeLayout::U128 => u128::deserialize(deserializer)? as u64,
            layout if is_derived_string_struct_layout(layout) => {
                // Here, we make sure we convert identifiers to fixed-size Move
                // values. This is needed because we charge gas based on the resource
                // size with identifiers inside, and so it has to be deterministic.
                let value = DeserializationSeed {
                    native_deserializer: None::<&NativeValueSimpleSerDe>,
                    layout,
                }.deserialize(deserializer)?;
                let (bytes, width) = value
                    .value_as::<Struct>()
                    .map(derived_string_struct_to_bytes_and_length).expect("TODO").expect("TODO");
                from_utf8_bytes::<u64>(bytes).expect("TODO")
            },
            _ => {
                return Err(D::Error::custom("Failed serialization"))
            },
        };
        let sized_id = SizedID::from(encoded);
        Ok(Value::native_value(sized_id))
    }
}

impl CustomSerialize for NativeValueSimpleSerDe {
    fn custom_serialize<S: Serializer>(
        &self,
        serializer: S,
        _tag: &IdentifierMappingKind,
        layout: &MoveTypeLayout,
        sized_id: SizedID,
    ) -> Result<S::Ok, S::Error> {
        match layout {
            // FIXME Add checks!
            MoveTypeLayout::U64 => serializer.serialize_u64(sized_id.into()),
            MoveTypeLayout::U128 => {
                let encoded: u64 = sized_id.into();
                serializer.serialize_u128(encoded as u128)
            },
            layout if is_derived_string_struct_layout(layout) => {
                let value = into_derived_string_struct(sized_id).ok_or_else(|| S::Error::custom("Failed serialization"))?;
                SerializationReadyValue {
                    native_serializer: None::<&NativeValueSimpleSerDe>,
                    layout,
                    value: &value.0,
                }.serialize(serializer)
            },
            _ => {
                return Err(S::Error::custom("Failed serialization"))
            },
        }
    }
}

pub fn deserialize_and_allow_native_values(bytes: &[u8], layout: &MoveTypeLayout) -> Option<Value> {
    let native_deserializer = NativeValueSimpleSerDe;
    let seed = DeserializationSeed {
        native_deserializer: Some(&native_deserializer),
        layout,
    };
    bcs::from_bytes_seed(seed, bytes).ok()
}

pub fn serialize_and_allow_native_values(
    value: &Value,
    layout: &MoveTypeLayout,
) -> Option<Vec<u8>> {
    let native_serializer = NativeValueSimpleSerDe;
    let value = SerializationReadyValue {
        native_serializer: Some(&native_serializer),
        layout,
        value: &value.0,
    };
    bcs::to_bytes(&value).ok()
}

pub trait ValueToIdentifierMapping {
    fn value_to_identifier(
        &self,
        // We need kind to distinguish between aggregators and snapshots
        // of the same type.
        kind: &IdentifierMappingKind,
        layout: &MoveTypeLayout,
        value: Value,
    ) -> PartialVMResult<SizedID>;

    fn identifier_to_value(
        &self,
        layout: &MoveTypeLayout,
        identifier: SizedID,
    ) -> PartialVMResult<Value>;
}

pub struct NativeValueSerDeWithExchange<'a> {
    mapping: &'a dyn ValueToIdentifierMapping,
}

impl<'a> CustomSerialize for NativeValueSerDeWithExchange<'a> {
    fn custom_serialize<S: Serializer>(
        &self,
        serializer: S,
        _tag: &IdentifierMappingKind,
        layout: &MoveTypeLayout,
        sized_id: SizedID,
    ) -> Result<S::Ok, S::Error> {
        let value = self
            .mapping
            .identifier_to_value(layout, sized_id)
            .map_err(|e| S::Error::custom(format!("{}", e)))?;
        SerializationReadyValue {
            native_serializer: None::<&NativeValueSimpleSerDe>,
            layout,
            value: &value.0,
        }
        .serialize(serializer)
    }
}

impl<'a> CustomDeserialize for NativeValueSerDeWithExchange<'a> {
    fn custom_deserialize<'d, D: Deserializer<'d>>(
        &self,
        deserializer: D,
        tag: &IdentifierMappingKind,
        layout: &MoveTypeLayout,
    ) -> Result<Value, D::Error> {
        let value = DeserializationSeed {
            native_deserializer: None::<&NativeValueSimpleSerDe>,
            layout,
        }
        .deserialize(deserializer)?;
        let size_id = self
            .mapping
            .value_to_identifier(tag, layout, value)
            .map_err(|e| D::Error::custom(format!("{}", e)))?;
        Ok(Value::native_value(size_id))
    }
}

pub fn deserialize_and_replace_values_with_ids(
    bytes: &[u8],
    layout: &MoveTypeLayout,
    mapping: &impl ValueToIdentifierMapping,
) -> Option<Value> {
    let native_deserializer = NativeValueSerDeWithExchange { mapping };
    let seed = DeserializationSeed {
        native_deserializer: Some(&native_deserializer),
        layout,
    };
    bcs::from_bytes_seed(seed, bytes).ok()
}

pub fn serialize_and_replace_ids_with_values(
    value: &Value,
    layout: &MoveTypeLayout,
    mapping: &impl ValueToIdentifierMapping,
) -> Option<Vec<u8>> {
    let native_serializer = NativeValueSerDeWithExchange { mapping };
    let value = SerializationReadyValue {
        native_serializer: Some(&native_serializer),
        layout,
        value: &value.0,
    };
    bcs::to_bytes(&value).ok()
}

// TODO[agg_v2](cleanup): Implement this traversal properly! Also this can be a general utility?
pub fn find_identifiers_in_value<I: From<SizedID> + Hash + Eq>(
    value: &Value,
    identifiers: &mut HashSet<I>,
) -> PartialVMResult<()> {
    find_identifiers_in_value_impl(&value.0, identifiers)
}

fn find_identifiers_in_value_impl<I: From<SizedID> + Hash + Eq>(
    value: &ValueImpl,
    identifiers: &mut HashSet<I>,
) -> PartialVMResult<()> {
    match value {
        ValueImpl::U8(_)
        | ValueImpl::U16(_)
        | ValueImpl::U32(_)
        | ValueImpl::U64(_)
        | ValueImpl::U128(_)
        | ValueImpl::U256(_)
        | ValueImpl::Bool(_)
        | ValueImpl::Address(_) => {},

        ValueImpl::Container(c) => match c {
            Container::Locals(_) => {
                return Err(PartialVMError::new(
                    StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                ))
            },

            Container::VecU8(_)
            | Container::VecU64(_)
            | Container::VecU128(_)
            | Container::VecBool(_)
            | Container::VecAddress(_)
            | Container::VecU16(_)
            | Container::VecU32(_)
            | Container::VecU256(_) => {},

            Container::Vec(v) | Container::Struct(v) => {
                for val in v.borrow().iter() {
                    find_identifiers_in_value_impl(val, identifiers)?;
                }
            },
        },

        ValueImpl::Invalid | ValueImpl::ContainerRef(_) | ValueImpl::IndexedRef(_) => {
            return Err(PartialVMError::new(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
            ))
        },

        ValueImpl::Native { id } => {
            if !identifiers.insert(I::from(*id)) {
                return Err(
                    PartialVMError::new(StatusCode::DELAYED_FIELDS_CODE_INVARIANT_ERROR)
                        .with_message("Duplicated identifiers for Move value".to_string()),
                );
            }
        },
    }
    Ok(())
}
