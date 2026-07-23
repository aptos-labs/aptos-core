// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for the `from_bcs` and `util` modules, which share one
//! implementation that deserializes a value from its BCS encoding.

use crate::{polymorphic_natives, NativeEntry};
use mono_move_core::{
    native::{NativeContext, NativeContextFamily, NativeStatus, Vector},
    VMResult,
};

/// `0x1::from_bcs::from_bytes<T>(bytes: vector<u8>): T`, and the identical
/// `0x1::util::from_bytes<T>`.
///
/// Deserializes `bytes` as a value of type `T`; a malformed encoding propagates
/// as a VM error.
//
// TODO(metering): charge gas.
pub fn native_from_bytes<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let ty = ctx.ty_arg(0)?;
    // SAFETY: arg 0 is `vector<u8>`, passed by value.
    let v: Vector<u8> = unsafe { ctx.arg(0)? };
    // Copy off the VM heap first: deserialization allocates and may relocate it.
    // TODO(perf): avoid the copy to the Rust heap
    // SAFETY: the bytes are copied immediately, before any allocation.
    let bytes = unsafe { v.as_bytes() }.to_vec();
    let value = ctx.bcs_deserialize_value(ty, &bytes)?;
    // SAFETY: `value` is the in-frame representation of type `ty`, which is the
    // return type `T`; it is written before any further heap allocation.
    unsafe { ctx.set_return_raw(0, &value)? };
    Ok(NativeStatus::Success)
}

/// Natives for the `from_bcs` and `util` modules.
pub fn make_all_from_bytes_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    polymorphic_natives![
        ("0x1::from_bcs::from_bytes", native_from_bytes),
        ("0x1::util::from_bytes", native_from_bytes),
    ]
}
