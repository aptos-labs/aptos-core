// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for the `cmp` module.

use crate::{polymorphic_natives, NativeEntry};
use mono_move_core::{
    native::{NativeContext, NativeContextFamily, NativeStatus, Opaque, Ref},
    VMResult, TRIVIAL_DESCRIPTOR_ID,
};
use std::cmp::Ordering;

/// `0x1::cmp::compare<T>(first: &T, second: &T): Ordering`
//
// TODO(metering): charge gas.
pub fn native_compare<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let ty = ctx.ty_arg(0)?;
    // SAFETY: args 0 and 1 are `&T`, whose pointee type is `ty`.
    let first: Ref<Opaque> = unsafe { ctx.arg(0)? };
    let second: Ref<Opaque> = unsafe { ctx.arg(1)? };
    // SAFETY: both references are live values of type `ty` for the rest of the call.
    let tag: u64 = match unsafe { ctx.compare(first.ptr(), second.ptr(), ty)? } {
        Ordering::Less => 0,
        Ordering::Equal => 1,
        Ordering::Greater => 2,
    };
    // SAFETY: `Ordering`'s variants are fieldless and pointer-free, so the
    // trivial descriptor traces it and `tag` is a valid variant.
    let value = unsafe { ctx.new_enum(TRIVIAL_DESCRIPTOR_ID, tag, ())? };
    // SAFETY: return 0 is `Ordering`, which `value` is the heap object for.
    unsafe { ctx.set_return(0, value)? };
    Ok(NativeStatus::Success)
}

/// Natives for the `cmp` module.
pub fn make_all_cmp_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    polymorphic_natives![("0x1::cmp::compare", native_compare)]
}
