// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for the `bcs` module.

use crate::{polymorphic_natives, NativeEntry};
use mono_move_core::native::{
    NativeContext, NativeContextFamily, NativeStatus, Opaque, Ref, VMInternalError,
};

/// Abort code raised on a BCS serialization failure. Matches
/// `NFE_BCS_SERIALIZATION_FAILURE` in the legacy VM.
const NFE_BCS_SERIALIZATION_FAILURE: u64 = 0x1C5;

/// `0x1::bcs::to_bytes<MoveValue>(v: &MoveValue): vector<u8>`
///
/// BCS-serializes the referenced value. Aborts if the value cannot be
/// serialized.
//
// TODO(gas): charge for the serialized size, matching the legacy VM.
//
// TODO: enums and function values are not yet serializable in mono-move, so a
// `MoveValue` containing either is unsupported.
pub fn native_to_bytes<C: NativeContext>(ctx: &C) -> Result<NativeStatus, VMInternalError> {
    let ty = ctx.ty_arg(0)?;
    // SAFETY: arg 0 is `&MoveValue`, a reference read as an opaque pointer.
    let value: Ref<Opaque> = unsafe { ctx.arg(0)? };
    let bytes = match ctx.serialize(ty, &value)? {
        Some(bytes) => bytes,
        None => {
            return Ok(NativeStatus::Abort {
                code: NFE_BCS_SERIALIZATION_FAILURE,
                message: None,
            })
        },
    };
    let out = ctx.new_byte_vector(&bytes)?;
    // SAFETY: return 0 is `vector<u8>`.
    unsafe { ctx.set_return(0, out)? };
    Ok(NativeStatus::Success)
}

/// `0x1::bcs::serialized_size<MoveValue>(v: &MoveValue): u64`
///
/// Returns the BCS serialized size of the referenced value. Aborts if the value
/// cannot be serialized.
//
// TODO(gas): charge for the serialized size, matching the legacy VM.
pub fn native_serialized_size<C: NativeContext>(ctx: &C) -> Result<NativeStatus, VMInternalError> {
    let ty = ctx.ty_arg(0)?;
    // SAFETY: arg 0 is `&MoveValue`, a reference read as an opaque pointer.
    let value: Ref<Opaque> = unsafe { ctx.arg(0)? };
    let size = match ctx.serialized_size(ty, &value)? {
        Some(size) => size as u64,
        None => {
            return Ok(NativeStatus::Abort {
                code: NFE_BCS_SERIALIZATION_FAILURE,
                message: None,
            })
        },
    };
    // SAFETY: return 0 is `u64`.
    unsafe { ctx.set_return(0, size)? };
    Ok(NativeStatus::Success)
}

/// `0x1::bcs::constant_serialized_size<MoveValue>(): Option<u64>`
///
/// Not yet implemented: the return type `Option<u64>` is an enum, and mono-move
/// has no API to construct enum values from a native (the value serializer also
/// does not yet support enums). Registered so the `bcs` module links; calling it
/// surfaces this error.
//
// TODO: implement once natives can construct enum return values.
pub fn native_constant_serialized_size<C: NativeContext>(
    _ctx: &C,
) -> Result<NativeStatus, VMInternalError> {
    Err(VMInternalError::InvariantViolation(
        "bcs::constant_serialized_size is not yet implemented in mono-move".into(),
    ))
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
