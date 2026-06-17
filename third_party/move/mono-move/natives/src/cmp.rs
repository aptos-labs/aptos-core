// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for the `cmp` module.

use crate::{polymorphic_natives, NativeEntry};
use mono_move_core::native::{NativeContext, NativeContextFamily, NativeStatus, VMInternalError};
use std::cmp::Ordering;

/// `0x1::cmp::compare<T>(first: &T, second: &T): Ordering`
///
/// Structurally compares the two referenced values with MonoMove's natural
/// ordering and returns the corresponding `0x1::cmp::Ordering` enum value.
//
// TODO: charge gas.
pub fn native_compare<C: NativeContext>(ctx: &C) -> Result<NativeStatus, VMInternalError> {
    let ty = ctx.ty_arg(0)?;
    // SAFETY: args 0 and 1 are the `&T` references to live values of type `ty`.
    let ordering = unsafe { ctx.compare_args(0, 1, ty)? };
    // `Ordering` is a fieldless enum `Less | Equal | Greater`; its BCS encoding
    // is the single ULEB128 variant-index byte (0, 1, or 2).
    let tag: u8 = match ordering {
        Ordering::Less => 0,
        Ordering::Equal => 1,
        Ordering::Greater => 2,
    };
    // The return type `Ordering` is not a type argument, so read it from the ABI
    // and build the enum value by deserializing its one-byte BCS encoding.
    let ordering_ty = ctx.return_ty(0)?;
    let value = ctx.bcs_deserialize_value(ordering_ty, &[tag])?;
    // SAFETY: `value` is the in-frame representation of the return type
    // `Ordering`; it is written before any further heap allocation.
    unsafe { ctx.set_return_raw(0, &value)? };
    Ok(NativeStatus::Success)
}

/// Natives for the `cmp` module.
pub fn make_all_cmp_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    polymorphic_natives![("0x1::cmp::compare", native_compare)]
}
