// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for the `bcs` module.

use crate::{polymorphic_natives, NativeEntry};
use mono_move_core::native::{
    NativeContext, NativeContextFamily, NativeStatus, Opaque, Ref, VMInternalError,
};

/// Abort code raised when BCS serialization fails. Matches the legacy VM's
/// `NFE_BCS_SERIALIZATION_FAILURE`.
const NFE_BCS_SERIALIZATION_FAILURE: u64 = 0x1C5;

/// `0x1::bcs::to_bytes<T>(v: &T): vector<u8>`
///
/// BCS-serializes the referenced value. Aborts if serialization fails.
//
// TODO: charge gas.
pub fn native_to_bytes<C: NativeContext>(ctx: &C) -> Result<NativeStatus, VMInternalError> {
    let ty = ctx.ty_arg(0)?;
    // SAFETY: arg 0 is the reference `&T`, whose pointee type is `ty`.
    let arg: Ref<Opaque> = unsafe { ctx.arg(0)? };
    // SAFETY: `arg` references a live value of type `ty` for the rest of the call.
    let bytes = match unsafe { ctx.bcs_serialize_value(arg.ptr(), ty) }? {
        Ok(bytes) => bytes,
        Err(_) => return Ok(serialization_failure()),
    };
    let out = ctx.new_byte_vector(&bytes)?;
    // SAFETY: return 0 is `vector<u8>`.
    unsafe { ctx.set_return(0, out)? };
    Ok(NativeStatus::Success)
}

/// `0x1::bcs::serialized_size<T>(v: &T): u64`
///
/// Returns the BCS serialized size of the referenced value. Aborts if
/// serialization fails.
//
// TODO: charge gas; compute the size without building the byte buffer.
pub fn native_serialized_size<C: NativeContext>(ctx: &C) -> Result<NativeStatus, VMInternalError> {
    let ty = ctx.ty_arg(0)?;
    // SAFETY: arg 0 is the reference `&T`, whose pointee type is `ty`.
    let arg: Ref<Opaque> = unsafe { ctx.arg(0)? };
    // SAFETY: `arg` references a live value of type `ty` for the rest of the call.
    let size = match unsafe { ctx.bcs_serialize_value(arg.ptr(), ty) }? {
        Ok(bytes) => bytes.len() as u64,
        Err(_) => return Ok(serialization_failure()),
    };
    // SAFETY: return 0 is `u64`.
    unsafe { ctx.set_return(0, size)? };
    Ok(NativeStatus::Success)
}

fn serialization_failure() -> NativeStatus {
    NativeStatus::Abort {
        code: NFE_BCS_SERIALIZATION_FAILURE,
        message: None,
    }
}

/// Natives for the `bcs` module.
pub fn make_all_bcs_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    polymorphic_natives![
        ("0x1::bcs::to_bytes", native_to_bytes),
        ("0x1::bcs::serialized_size", native_serialized_size),
    ]
}
