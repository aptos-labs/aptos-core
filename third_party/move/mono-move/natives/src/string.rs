// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for the `string` module.

use crate::{monomorphic_natives, NativeEntry};
use mono_move_core::{
    native::{NativeContext, NativeContextFamily, NativeStatus, Ref, Vector},
    VMResult,
};

/// `0x1::string::internal_check_utf8(v: &vector<u8>): bool`
///
/// Returns true if `v` is valid UTF-8.
//
// TODO(metering): charge gas.
pub fn native_check_utf8<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `&vector<u8>`.
    let s: Ref<Vector<u8>> = unsafe { ctx.arg(0)? };
    let v = s.borrow();
    let valid = {
        // SAFETY: the bytes are consumed immediately before any allocation,
        // so GC cannot relocate them while the slice is held.
        let bytes = unsafe { v.as_bytes() };
        std::str::from_utf8(bytes).is_ok()
    };
    // SAFETY: return 0 is `bool`.
    unsafe { ctx.set_return(0, valid)? };
    Ok(NativeStatus::Success)
}

/// Natives for the `string` module.
pub fn make_all_string_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    monomorphic_natives![("0x1::string::internal_check_utf8", native_check_utf8)]
}
