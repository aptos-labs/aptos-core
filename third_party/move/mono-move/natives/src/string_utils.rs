// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for the `string_utils` module.

use crate::{polymorphic_natives, NativeEntry};
use mono_move_core::{
    native::{
        native_invariant_violation, NativeContext, NativeContextFamily, NativeStatus, Opaque, Ref,
    },
    types::{view_type, Type},
    VMResult,
};
use move_core_types::{account_address::AccountAddress, int256::U256};

/// Reads the scalar behind a Move reference.
///
/// # Safety
///
/// `value` must reference a live value whose in-memory representation is a `T`.
unsafe fn read<T: Copy>(value: &Ref<'_, Opaque>) -> T {
    unsafe { std::ptr::read_unaligned(value.ptr() as *const T) }
}

/// `0x1::string_utils::native_format<T>(s: &T, type_tag: bool, canonicalize: bool, single_line:
/// bool, include_int_types: bool): String`
///
/// Renders the referenced value the way the legacy VM's native does, for the scalar types.
//
// TODO(completeness): vectors, structs, enums and `signer`, which the legacy native walks
// recursively against the value's layout. A call on one of those fails rather than formatting it
// differently.
// TODO(metering): charge gas, which the legacy native charges per byte of output.
pub fn native_format<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let ty = ctx.ty_arg(0)?;
    // SAFETY: arg 0 is the reference `&T`, whose pointee type is `ty`.
    let value: Ref<Opaque> = unsafe { ctx.arg(0)? };
    // SAFETY: args 1..5 are the four `bool` flags.
    let include_type_tag: bool = unsafe { ctx.arg(1)? };
    let canonicalize: bool = unsafe { ctx.arg(2)? };
    let include_int_types: bool = unsafe { ctx.arg(4)? };

    // SAFETY: `value` references a live value of `ty`, and each arm reads it as the Rust type
    // `ty` is represented by.
    let (formatted, int_type) = unsafe {
        match view_type(ty) {
            Type::Bool => (read::<bool>(&value).to_string(), ""),
            Type::U8 => (read::<u8>(&value).to_string(), "u8"),
            Type::U16 => (read::<u16>(&value).to_string(), "u16"),
            Type::U32 => (read::<u32>(&value).to_string(), "u32"),
            Type::U64 => (read::<u64>(&value).to_string(), "u64"),
            Type::U128 => (read::<u128>(&value).to_string(), "u128"),
            Type::U256 => (read::<U256>(&value).to_string(), "u256"),
            Type::Address => {
                let address = read::<AccountAddress>(&value);
                let rendered = if canonicalize {
                    address.to_canonical_string()
                } else {
                    address.to_hex_literal()
                };
                (format!("@{rendered}"), "")
            },
            _ => {
                return Err(native_invariant_violation(
                    "string_utils::native_format formats scalars only".to_string(),
                ))
            },
        }
    };

    let mut out = formatted;
    if include_int_types && !int_type.is_empty() {
        out.push_str(int_type);
    }
    // The type tag prefixes an aggregate's rendering, and no scalar's.
    let _ = include_type_tag;

    let bytes = ctx.new_byte_vector(out.as_bytes())?;
    // SAFETY: return 0 is `String`, which is the struct `{ bytes: vector<u8> }` and so is laid out
    // as its one field.
    unsafe { ctx.set_return(0, bytes)? };
    Ok(NativeStatus::Success)
}

/// `0x1::string_utils::native_format_list<T>(fmt: &vector<u8>, val: &T): String`
//
// TODO(completeness): implement. It walks a tuple of values against a format string, which needs
// the same recursive formatter `native_format` is missing.
pub fn native_format_list<C: NativeContext>(_ctx: &C) -> VMResult<NativeStatus> {
    Err(native_invariant_violation(
        "string_utils::native_format_list is not implemented".to_string(),
    ))
}

/// Natives for the `string_utils` module.
pub fn make_all_string_utils_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    polymorphic_natives![
        ("0x1::string_utils::native_format", native_format),
        ("0x1::string_utils::native_format_list", native_format_list),
    ]
}
