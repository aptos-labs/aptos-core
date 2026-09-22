// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Translation between the stable value schema and MonoMove's BCS
//! representation.
//!
//! Arguments are encoded from the value schema alone: the encoding is
//! self-describing and matches the BCS encoding of the parameter type of a
//! well-typed call, so a mismatch between the declared value and the
//! function's signature surfaces as a placement failure inside MonoMove
//! rather than as adapter-side guessing. Results are serialized by the
//! runtime against the loaded return type (the BCS inverse of placement)
//! and decoded here against the same type; a shape the adapter cannot yet
//! decode is an explicit error, never a silently wrong value.

use crate::payload::Value;
use mono_move_core::types::{view_type, InternedType, Type};
use mono_move_runtime::InterpreterContext;
use move_core_types::{
    account_address::AccountAddress,
    int256::{I256, U256},
};

/// Encodes one value as its BCS bytes.
pub fn encode_bcs(value: &Value) -> Result<Vec<u8>, String> {
    match value {
        Value::Unit => Ok(vec![]),
        Value::Bool { value } => Ok(vec![u8::from(*value)]),
        Value::Integer {
            width,
            signed,
            value,
        } => encode_integer(*width, *signed, value),
        Value::Address { value } => {
            let address =
                AccountAddress::from_hex_literal(value).map_err(|error| format!("{error}"))?;
            Ok(address.into_bytes().to_vec())
        },
        Value::Vector { elements } => {
            let mut encoded = Vec::new();
            let mut width = None;
            for element in elements {
                let bytes = encode_bcs(element)?;
                match width {
                    None => width = Some(bytes.len()),
                    Some(width) if width != bytes.len() => {
                        return Err("vector elements must all encode to the same width".to_string())
                    },
                    Some(_) => {},
                }
                encoded.extend_from_slice(&bytes);
            }
            let mut out = uleb128(elements.len() as u64);
            out.extend_from_slice(&encoded);
            Ok(out)
        },
    }
}

/// Parses an address written as a canonical `0x…` hex literal.
pub fn parse_address(text: &str) -> Result<AccountAddress, String> {
    AccountAddress::from_hex_literal(text).map_err(|error| format!("{error}"))
}

/// Encodes a decimal integer literal as `width`-wide little-endian BCS.
fn encode_integer(width: u16, signed: bool, text: &str) -> Result<Vec<u8>, String> {
    if !matches!(width, 8 | 16 | 32 | 64 | 128 | 256) {
        return Err(format!("{width}-bit integers are not supported"));
    }
    let byte_width = width as usize / 8;
    if signed {
        if width == 256 {
            let value: I256 = text
                .parse()
                .map_err(|_| format!("invalid signed integer literal {text:?}"))?;
            return Ok(value.to_le_bytes().to_vec());
        }
        let value: i128 = text
            .parse()
            .map_err(|_| format!("invalid signed integer literal {text:?}"))?;
        let limit = 1i128 << (width - 1);
        if !(-limit..=limit - 1).contains(&value) {
            return Err(format!(
                "signed integer {value} does not fit in {width} bits"
            ));
        }
        Ok(value.to_le_bytes()[..byte_width].to_vec())
    } else {
        if width == 256 {
            let value: U256 = text
                .parse()
                .map_err(|_| format!("invalid unsigned integer literal {text:?}"))?;
            return Ok(value.to_le_bytes().to_vec());
        }
        let value: u128 = text
            .parse()
            .map_err(|_| format!("invalid unsigned integer literal {text:?}"))?;
        if width < 128 && value >= 1u128 << width {
            return Err(format!(
                "unsigned integer {value} does not fit in {width} bits"
            ));
        }
        Ok(value.to_le_bytes()[..byte_width].to_vec())
    }
}

/// Writes `value` as its ULEB128 encoding, the BCS vector length prefix.
fn uleb128(mut value: u64) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let byte = (value & 0x7F) as u8;
        value >>= 7;
        if value == 0 {
            out.push(byte);
            return out;
        }
        out.push(byte | 0x80);
    }
}

/// Decodes the root frame's results against the loaded return types.
///
/// The runtime serializes each completed call's result to BCS by its return
/// type; this decodes those bytes back into the value schema. Zero or one
/// result per call is the shape Move source produces; more is explicit
/// unsupported until the serializer covers multi-return calls.
pub fn read_root_results(
    interp: &InterpreterContext<'_>,
    returns: &[InternedType],
) -> Result<Vec<Value>, String> {
    match returns.len() {
        0 => Ok(vec![]),
        1 => {
            let bytes = interp
                .serialize_root_result(returns[0])
                .map_err(|error| format!("{error}"))?;
            Ok(vec![decode_bcs(&bytes, view_type(returns[0]))?])
        },
        count => Err(format!(
            "{count} return values per call are not yet supported"
        )),
    }
}

/// Strictly decodes a complete BCS value of the given type.
pub fn decode_bcs(bytes: &[u8], ty: &Type) -> Result<Value, String> {
    let mut cursor = 0;
    let value = decode_value(bytes, &mut cursor, ty)?;
    if cursor != bytes.len() {
        return Err(format!(
            "BCS value of this type decodes {cursor} of {} bytes",
            bytes.len()
        ));
    }
    Ok(value)
}

/// Decodes one value of `ty` starting at `cursor`, advancing it.
fn decode_value(bytes: &[u8], cursor: &mut usize, ty: &Type) -> Result<Value, String> {
    match ty {
        Type::Bool => {
            let byte = take(bytes, cursor, 1)?[0];
            Ok(Value::Bool { value: byte != 0 })
        },
        Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::U128 => {
            let width = int_width(ty);
            let raw = read_le_u128(bytes, cursor, width as usize / 8)?;
            Ok(integer_value(width, false, raw.to_string()))
        },
        Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::I128 => {
            let width = int_width(ty);
            let raw = read_le_u128(bytes, cursor, width as usize / 8)?;
            let limit = 1u128 << (width - 1);
            let signed = if raw >= limit {
                (raw as i128).wrapping_sub(1i128 << width)
            } else {
                raw as i128
            };
            Ok(integer_value(width, true, signed.to_string()))
        },
        Type::U256 => {
            let raw = U256::from_le_bytes(take_fixed::<32>(bytes, cursor)?);
            Ok(integer_value(256, false, raw.to_string()))
        },
        Type::I256 => {
            let raw = I256::from_le_bytes(take_fixed::<32>(bytes, cursor)?);
            Ok(integer_value(256, true, raw.to_string()))
        },
        Type::Address => {
            let raw = take_fixed::<{ AccountAddress::LENGTH }>(bytes, cursor)?;
            Ok(Value::Address {
                value: AccountAddress::new(raw).to_hex_literal(),
            })
        },
        Type::Vector { elem } => {
            let len = read_uleb128(bytes, cursor)?;
            let mut elements = Vec::new();
            for _ in 0..len {
                elements.push(decode_value(bytes, cursor, view_type(*elem))?);
            }
            Ok(Value::Vector { elements })
        },
        other => Err(format!(
            "a {} value is not yet supported by the adapter",
            describe_type(other)
        )),
    }
}

/// The bit width of a supported integer type.
fn int_width(ty: &Type) -> u16 {
    match ty {
        Type::U8 | Type::I8 => 8,
        Type::U16 | Type::I16 => 16,
        Type::U32 | Type::I32 => 32,
        Type::U64 | Type::I64 => 64,
        Type::U128 | Type::I128 => 128,
        _ => unreachable!("int_width is only called for integer types"),
    }
}

/// Takes `n` bytes at `cursor`.
fn take<'b>(bytes: &'b [u8], cursor: &mut usize, n: usize) -> Result<&'b [u8], String> {
    let end = cursor
        .checked_add(n)
        .filter(|end| *end <= bytes.len())
        .ok_or_else(|| format!("BCS input ends inside a value at byte {cursor}"))?;
    let slice = &bytes[*cursor..end];
    *cursor = end;
    Ok(slice)
}

/// Takes a fixed-width slice at `cursor`.
fn take_fixed<const N: usize>(bytes: &[u8], cursor: &mut usize) -> Result<[u8; N], String> {
    let slice = take(bytes, cursor, N)?;
    let mut out = [0; N];
    out.copy_from_slice(slice);
    Ok(out)
}

/// Reads `n` little-endian bytes as a `u128` at `cursor`.
fn read_le_u128(bytes: &[u8], cursor: &mut usize, n: usize) -> Result<u128, String> {
    let slice = take(bytes, cursor, n)?;
    Ok(slice
        .iter()
        .enumerate()
        .fold(0u128, |acc, (i, byte)| acc | (u128::from(*byte)) << (8 * i)))
}

/// Reads a ULEB128 length at `cursor`.
fn read_uleb128(bytes: &[u8], cursor: &mut usize) -> Result<u64, String> {
    let mut value = 0u64;
    for shift in (0..64).step_by(7) {
        let byte = take(bytes, cursor, 1)?[0];
        value |= u64::from(byte & 0x7F) << shift;
        if byte & 0x80 == 0 {
            return Ok(value);
        }
    }
    Err("BCS vector length is not a valid ULEB128".to_string())
}

/// Names a type in the unsupported-shape diagnostic; the runtime `Type` does
/// not implement `Debug`.
fn describe_type(ty: &Type) -> &'static str {
    match ty {
        Type::Bool => "bool",
        Type::U8 => "u8",
        Type::U16 => "u16",
        Type::U32 => "u32",
        Type::U64 => "u64",
        Type::U128 => "u128",
        Type::U256 => "u256",
        Type::I8 => "i8",
        Type::I16 => "i16",
        Type::I32 => "i32",
        Type::I64 => "i64",
        Type::I128 => "i128",
        Type::I256 => "i256",
        Type::Address => "address",
        Type::Signer => "signer",
        Type::Vector { .. } => "vector",
        Type::Nominal { .. } => "nominal",
        Type::ImmutRef { .. } => "immutable reference",
        Type::MutRef { .. } => "mutable reference",
        Type::Function { .. } => "function",
        Type::TypeParam { .. } => "type parameter",
    }
}

fn integer_value(width: u16, signed: bool, value: String) -> Value {
    Value::Integer {
        width,
        signed,
        value,
    }
}
