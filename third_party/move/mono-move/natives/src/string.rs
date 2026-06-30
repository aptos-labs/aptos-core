// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for the `string` module.

use crate::{monomorphic_natives, NativeEntry};
use mono_move_core::native::{
    NativeContext, NativeContextFamily, NativeStatus, Ref, VMInternalError, Vector,
};

/// Abort code raised by `internal_sub_string` when `j < i`.
const EINVALID_RANGE: u64 = 1;

/// Interprets `bytes` as UTF-8, raising an invariant violation if they are not.
/// The `vector<u8>` wrapped in a `String` is assumed to already be valid UTF-8,
/// so a failure here is a VM-internal violation, not a user abort.
fn from_utf8_checked(bytes: &[u8]) -> Result<&str, VMInternalError> {
    std::str::from_utf8(bytes)
        .map_err(|_| VMInternalError::invariant_violation("Every string must be UTF-8".to_string()))
}

/// `0x1::string::internal_check_utf8(v: &vector<u8>): bool`
///
/// Returns true if `v` is valid UTF-8.
//
// TODO(metering): charge gas.
pub fn native_check_utf8<C: NativeContext>(ctx: &C) -> Result<NativeStatus, VMInternalError> {
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

/// `0x1::string::internal_is_char_boundary(v: &vector<u8>, i: u64): bool`
///
/// Returns true if byte index `i` lies on a UTF-8 character boundary of `v`.
//
// TODO(metering): charge gas.
pub fn native_is_char_boundary<C: NativeContext>(ctx: &C) -> Result<NativeStatus, VMInternalError> {
    // SAFETY: arg 0 is `&vector<u8>`.
    let s: Ref<Vector<u8>> = unsafe { ctx.arg(0)? };
    // SAFETY: arg 1 is `u64`.
    let i = unsafe { ctx.arg::<u64>(1)? };
    let v = s.borrow();
    let ok = {
        // SAFETY: the bytes are consumed immediately before any allocation,
        // so GC cannot relocate them while the slice is held.
        let bytes = unsafe { v.as_bytes() };
        from_utf8_checked(bytes)?.is_char_boundary(i as usize)
    };
    // SAFETY: return 0 is `bool`.
    unsafe { ctx.set_return(0, ok)? };
    Ok(NativeStatus::Success)
}

/// `0x1::string::internal_sub_string(v: &vector<u8>, i: u64, j: u64): vector<u8>`
///
/// Returns the bytes of `v` in the half-open range `[i, j)`. Aborts with
/// `EINVALID_RANGE` if `j < i`. `i` and `j` must be char boundaries within
/// bounds, otherwise the slice below panics.
//
// TODO(metering): charge gas.
pub fn native_sub_string<C: NativeContext>(ctx: &C) -> Result<NativeStatus, VMInternalError> {
    // SAFETY: arg 0 is `&vector<u8>`.
    let s: Ref<Vector<u8>> = unsafe { ctx.arg(0)? };
    // SAFETY: arg 1 is `u64`.
    let i = unsafe { ctx.arg::<u64>(1)? } as usize;
    // SAFETY: arg 2 is `u64`.
    let j = unsafe { ctx.arg::<u64>(2)? } as usize;

    if j < i {
        return Ok(NativeStatus::Abort {
            code: EINVALID_RANGE,
            message: Some("sub_string range end is before its start".to_string()),
        });
    }

    let v = s.borrow();
    // Copy the substring off the VM heap before allocating: `new_byte_vector`
    // may trigger a GC that relocates the source bytes.
    let sub = {
        // SAFETY: the bytes are consumed immediately, into an owned `Vec`,
        // before any allocation.
        let bytes = unsafe { v.as_bytes() };
        from_utf8_checked(bytes)?[i..j].as_bytes().to_vec()
    };
    let out = ctx.new_byte_vector(&sub)?;
    // SAFETY: return 0 is `vector<u8>`.
    unsafe { ctx.set_return(0, out)? };
    Ok(NativeStatus::Success)
}

/// `0x1::string::internal_index_of(v: &vector<u8>, r: &vector<u8>): u64`
///
/// Returns the byte index of the first occurrence of `r` in `v`, or `v.len()`
/// if `r` does not occur.
//
// TODO(metering): charge gas.
pub fn native_index_of<C: NativeContext>(ctx: &C) -> Result<NativeStatus, VMInternalError> {
    // SAFETY: arg 0 is `&vector<u8>`.
    let s: Ref<Vector<u8>> = unsafe { ctx.arg(0)? };
    // SAFETY: arg 1 is `&vector<u8>`.
    let r: Ref<Vector<u8>> = unsafe { ctx.arg(1)? };
    let s_vec = s.borrow();
    let r_vec = r.borrow();
    let pos = {
        // SAFETY: both slices are consumed immediately by `find`, which does
        // not allocate, so GC cannot relocate them while they are held.
        let s_str = from_utf8_checked(unsafe { s_vec.as_bytes() })?;
        let r_str = from_utf8_checked(unsafe { r_vec.as_bytes() })?;
        s_str.find(r_str).unwrap_or(s_str.len())
    };
    // SAFETY: return 0 is `u64`.
    unsafe { ctx.set_return(0, pos as u64)? };
    Ok(NativeStatus::Success)
}

/// Natives for the `string` module.
pub fn make_all_string_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    monomorphic_natives![
        ("0x1::string::internal_check_utf8", native_check_utf8),
        (
            "0x1::string::internal_is_char_boundary",
            native_is_char_boundary
        ),
        ("0x1::string::internal_sub_string", native_sub_string),
        ("0x1::string::internal_index_of", native_index_of),
    ]
}
