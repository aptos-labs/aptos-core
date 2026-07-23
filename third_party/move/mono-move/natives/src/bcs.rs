// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for the `bcs` module.

use crate::{polymorphic_natives, NativeEntry};
use mono_move_core::{
    native::{NativeContext, NativeContextFamily, NativeStatus, Opaque, Ref},
    VMResult, TRIVIAL_DESCRIPTOR_ID,
};

// Variant tags of `std::option::Option`, in declaration order.
const OPTION_NONE_TAG: u64 = 0;
const OPTION_SOME_TAG: u64 = 1;

/// `0x1::bcs::to_bytes<T>(v: &T): vector<u8>`
///
/// BCS-serializes the referenced value; a serialization failure propagates as a
/// VM error.
//
// TODO(metering): charge gas.
pub fn native_to_bytes<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let ty = ctx.ty_arg(0)?;
    // SAFETY: arg 0 is the reference `&T`, whose pointee type is `ty`.
    let arg: Ref<Opaque> = unsafe { ctx.arg(0)? };
    // SAFETY: `arg` references a live value of type `ty` for the rest of the call.
    let bytes = unsafe { ctx.bcs_serialize_value(arg.ptr(), ty)? };
    let out = ctx.new_byte_vector(&bytes)?;
    // SAFETY: return 0 is `vector<u8>`.
    unsafe { ctx.set_return(0, out)? };
    Ok(NativeStatus::Success)
}

/// `0x1::bcs::serialized_size<T>(v: &T): u64`
///
/// Returns the BCS serialized size of the referenced value; a serialization
/// failure propagates as a VM error.
//
// TODO(metering): charge gas.
pub fn native_serialized_size<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let ty = ctx.ty_arg(0)?;
    // SAFETY: arg 0 is the reference `&T`, whose pointee type is `ty`.
    let arg: Ref<Opaque> = unsafe { ctx.arg(0)? };
    // SAFETY: `arg` references a live value of type `ty` for the rest of the call.
    let size = unsafe { ctx.bcs_serialized_size(arg.ptr(), ty)? };
    // SAFETY: return 0 is `u64`.
    unsafe { ctx.set_return(0, size as u64)? };
    Ok(NativeStatus::Success)
}

/// `0x1::bcs::constant_serialized_size<T>(): Option<u64>`
///
/// `Some(n)` if every value of `T` serializes to `n` bytes, else `None`.
//
// TODO(metering): charge gas.
pub fn native_constant_serialized_size<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let ty = ctx.ty_arg(0)?;
    // `Option<u64>` is the enum `{ None, Some { e: u64 } }`; both variants are
    // pointer-free, so the value is built under the trivial descriptor.
    // SAFETY: return 0 is `Option<u64>`, matching the variant built here.
    let value = match ctx.constant_serialized_size(ty)? {
        Some(size) => unsafe { ctx.new_enum(TRIVIAL_DESCRIPTOR_ID, OPTION_SOME_TAG, size)? },
        None => unsafe { ctx.new_enum(TRIVIAL_DESCRIPTOR_ID, OPTION_NONE_TAG, ())? },
    };
    // SAFETY: return 0 is `Option<u64>`, which `value` is the heap object for.
    unsafe { ctx.set_return(0, value)? };
    Ok(NativeStatus::Success)
}

/// Natives for the `bcs` module.
pub fn make_all_bcs_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    polymorphic_natives![
        ("0x1::bcs::to_bytes", native_to_bytes),
        ("0x1::bcs::serialized_size", native_serialized_size),
        (
            "0x1::bcs::constant_serialized_size",
            native_constant_serialized_size
        ),
    ]
}
