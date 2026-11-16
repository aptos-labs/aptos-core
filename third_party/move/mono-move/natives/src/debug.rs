// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for the `debug` module. Both are no-ops unless the VM runs in debug
//! mode, as defined by the runtime's `InterpreterOptions::enable_debugging`.

use crate::{monomorphic_natives, NativeEntry};
use mono_move_core::{
    native::{
        native_invariant_violation, NativeContext, NativeContextFamily, NativeStatus, Vector,
    },
    VMResult,
};

/// `0x1::debug::native_print(x: String)`
///
/// Prints `x`, already formatted by the Move-side `debug::print`.
///
/// Charges no gas, like V1.
pub fn native_print<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    if !ctx.debugging_enabled() {
        return Ok(NativeStatus::Success);
    }
    // SAFETY: arg 0 is `String`, which has the same representation as
    // `vector<u8>`.
    let v: Vector<u8> = unsafe { ctx.arg(0)? };
    // SAFETY: the bytes are consumed immediately, before any allocation.
    let bytes = unsafe { v.as_bytes() };
    let s = std::str::from_utf8(bytes)
        .map_err(|_| native_invariant_violation("Every string must be UTF-8".to_string()))?;
    // TODO(cleanup): revisit printing to stdout, e.g. collect the output so
    // embedders and tests can inspect it.
    println!("[debug] {s}");
    Ok(NativeStatus::Success)
}

/// `0x1::debug::native_stack_trace(): String`
///
/// Returns the caller's call stack, or an empty string outside debug mode.
///
/// Charges no gas, like V1.
pub fn native_stack_trace<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let trace = if ctx.debugging_enabled() {
        ctx.debug_stack_trace()?
    } else {
        String::new()
    };
    let out = ctx.new_byte_vector(trace.as_bytes())?;
    // SAFETY: return 0 is `String`, which has the same representation as
    // `vector<u8>`.
    unsafe { ctx.set_return(0, out)? };
    Ok(NativeStatus::Success)
}

/// Natives for the `debug` module.
pub fn make_all_debug_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    monomorphic_natives![
        ("0x1::debug::native_print", native_print),
        ("0x1::debug::native_stack_trace", native_stack_trace),
    ]
}
