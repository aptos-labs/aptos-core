// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Raw, typed read/write of values at a byte offset from a base pointer.

use crate::align::MAX_ALIGN;
use move_core_types::account_address::AccountAddress;

/// # Safety
/// `base.add(byte_offset)` must be valid and point to an initialized `u8`.
#[inline(always)]
pub unsafe fn read_u8(base: *const u8, byte_offset: impl Into<usize>) -> u8 {
    // SAFETY: caller must uphold the documented pointer requirements.
    unsafe { base.add(byte_offset.into()).read() }
}

/// # Safety
/// `base.add(byte_offset)` must be valid and writable for a `u8`.
#[inline(always)]
pub unsafe fn write_u8(base: *mut u8, byte_offset: impl Into<usize>, val: u8) {
    // SAFETY: caller must uphold the documented pointer requirements.
    unsafe { base.add(byte_offset.into()).write(val) }
}

/// Read a boolean slot. The slot invariant is that it holds exactly `0` or
/// `1`; the `debug_assert` catches any violation in debug builds.
///
/// # Safety
/// `base.add(byte_offset)` must be valid and point to an initialized boolean
/// byte.
#[inline(always)]
pub unsafe fn read_bool(base: *const u8, byte_offset: impl Into<usize>) -> bool {
    // SAFETY: caller must uphold the documented pointer requirements.
    let byte = unsafe { read_u8(base, byte_offset) };
    debug_assert!(byte <= 1, "boolean slot holds non-boolean byte {byte}");
    byte != 0
}

/// Write a boolean slot as the canonical `0`/`1` byte.
///
/// # Safety
/// `base.add(byte_offset)` must be valid and writable for a boolean byte.
#[inline(always)]
pub unsafe fn write_bool(base: *mut u8, byte_offset: impl Into<usize>, val: bool) {
    // SAFETY: caller must uphold the documented pointer requirements.
    unsafe { write_u8(base, byte_offset, val as u8) }
}

/// # Safety
/// `base.add(byte_offset)` must be valid, aligned, and point to an initialized `u64`.
#[inline(always)]
pub unsafe fn read_u64(base: *const u8, byte_offset: impl Into<usize>) -> u64 {
    // SAFETY: caller must uphold the documented pointer requirements.
    unsafe { (base.add(byte_offset.into()) as *const u64).read() }
}

/// # Safety
/// `base.add(byte_offset)` must be valid, aligned, and writable for a `u64`.
#[inline(always)]
pub unsafe fn write_u64(base: *mut u8, byte_offset: impl Into<usize>, val: u64) {
    // SAFETY: caller must uphold the documented pointer requirements.
    unsafe { (base.add(byte_offset.into()) as *mut u64).write(val) }
}

/// # Safety
/// `base.add(byte_offset)` must be valid, aligned, and point to an initialized pointer.
#[inline(always)]
pub unsafe fn read_ptr(base: *const u8, byte_offset: impl Into<usize>) -> *mut u8 {
    // SAFETY: caller must uphold the documented pointer requirements.
    unsafe { (base.add(byte_offset.into()) as *const *mut u8).read() }
}

/// # Safety
/// `base.add(byte_offset)` must be valid, aligned, and writable for a pointer.
#[inline(always)]
pub unsafe fn write_ptr(base: *mut u8, byte_offset: impl Into<usize>, ptr: *const u8) {
    // SAFETY: caller must uphold the documented pointer requirements.
    unsafe { (base.add(byte_offset.into()) as *mut *const u8).write(ptr) }
}

/// Byte offset of the scalar `offset` half within a 16-byte fat pointer; the
/// `base` half occupies the first 8 bytes.
const FAT_PTR_OFFSET_HALF: usize = 8;

/// Read a 16-byte fat pointer `(base, offset)` whose base half starts at
/// `byte_offset`.
///
/// # Safety
/// `base.add(byte_offset)` must be valid, aligned, and point to an initialized
/// 16-byte fat pointer.
#[inline(always)]
pub unsafe fn read_fat_ptr(base: *const u8, byte_offset: impl Into<usize>) -> (*mut u8, u64) {
    let byte_offset = byte_offset.into();
    // SAFETY: caller must uphold the documented pointer requirements.
    unsafe {
        (
            read_ptr(base, byte_offset),
            read_u64(base, byte_offset + FAT_PTR_OFFSET_HALF),
        )
    }
}

/// Write a 16-byte fat pointer `(ptr, offset)` whose base half starts at
/// `byte_offset`.
///
/// # Safety
/// `base.add(byte_offset)` must be valid, aligned, and writable for a 16-byte
/// fat pointer.
#[inline(always)]
pub unsafe fn write_fat_ptr(
    base: *mut u8,
    byte_offset: impl Into<usize>,
    ptr: *const u8,
    offset: u64,
) {
    let byte_offset = byte_offset.into();
    // SAFETY: caller must uphold the documented pointer requirements.
    unsafe {
        write_ptr(base, byte_offset, ptr);
        write_u64(base, byte_offset + FAT_PTR_OFFSET_HALF, offset);
    }
}

/// # Safety
/// `base.add(byte_offset)` must be valid, aligned, and point to an initialized `u32`.
#[inline(always)]
pub unsafe fn read_u32(base: *const u8, byte_offset: impl Into<usize>) -> u32 {
    // SAFETY: caller must uphold the documented pointer requirements.
    unsafe { (base.add(byte_offset.into()) as *const u32).read() }
}

/// # Safety
/// `base.add(byte_offset)` must be valid, aligned, and writable for a `u32`.
#[inline(always)]
pub unsafe fn write_u32(base: *mut u8, byte_offset: impl Into<usize>, val: u32) {
    // SAFETY: caller must uphold the documented pointer requirements.
    unsafe { (base.add(byte_offset.into()) as *mut u32).write(val) }
}

/// Read a 32-byte [`AccountAddress`] starting at `byte_offset`. Reads aligned
/// when its alignment is within [`MAX_ALIGN`], otherwise unaligned.
///
/// This check shall have no impact on performance as it is easily optimized
/// away by the compiler.
///
/// # Safety
/// `base.add(byte_offset)` must be valid and point to an initialized
/// [`AccountAddress`].
#[inline(always)]
pub unsafe fn read_account_address(
    base: *const u8,
    byte_offset: impl Into<usize>,
) -> AccountAddress {
    let ptr = unsafe { base.add(byte_offset.into()) as *const AccountAddress };
    // SAFETY: caller must uphold the documented pointer requirements.
    unsafe {
        if std::mem::align_of::<AccountAddress>() <= MAX_ALIGN {
            ptr.read()
        } else {
            ptr.read_unaligned()
        }
    }
}
