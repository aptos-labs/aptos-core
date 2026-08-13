// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for the `function_info` module.

use crate::{monomorphic_natives, NativeEntry};
use mono_move_core::{
    native::{NativeContext, NativeContextFamily, NativeStatus, Ref, Vector},
    VMResult,
};
use move_core_types::identifier::Identifier;

/// `0x1::function_info::is_identifier(s: &vector<u8>): bool`
///
/// Returns true if `s` is valid UTF-8 spelling a Move identifier.
//
// TODO(metering): charge gas.
pub fn native_is_identifier<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `&vector<u8>`.
    let s: Ref<Vector<u8>> = unsafe { ctx.arg(0)? };
    let v = s.borrow();
    let valid = {
        // SAFETY: the bytes are consumed immediately before any allocation,
        // so GC cannot relocate them while the slice is held.
        let bytes = unsafe { v.as_bytes() };
        std::str::from_utf8(bytes).is_ok_and(Identifier::is_valid)
    };
    // SAFETY: return 0 is `bool`.
    unsafe { ctx.set_return(0, valid)? };
    Ok(NativeStatus::Success)
}

/// Natives for the `function_info` module.
pub fn make_all_function_info_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    monomorphic_natives![("0x1::function_info::is_identifier", native_is_identifier)]
}
