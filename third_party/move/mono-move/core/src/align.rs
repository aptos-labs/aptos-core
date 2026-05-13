// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Alignment constants and helpers.

// ---------------------------------------------------------------------------
// MAX_ALIGN
// ---------------------------------------------------------------------------

/// Maximum alignment used by any value or VM-internal layout. Bounds the
/// alignment of region bases, the frame pointer, the bump pointer, and
/// the padding rounded into per-object `size` fields and frame segments.
///
/// Must be a power of two, at least 8 (the heap object header is 8 bytes,
/// and the 24-byte frame metadata block needs 8-byte granularity), and a
/// multiple of every alignment used by any value or VM-internal layout
/// (the third constraint is implied by the first two as long as every
/// such alignment is a power of two ≤ [`MAX_ALIGN`]).
pub const MAX_ALIGN: usize = 8;

const _: () = {
    assert!(MAX_ALIGN.is_power_of_two());
    assert!(MAX_ALIGN >= 8);
};

// ---------------------------------------------------------------------------
// Parametric rounding (usize)
// ---------------------------------------------------------------------------

/// Round `size` up to the next multiple of `align`. Wraps if
/// `size + (align - 1)` overflows `usize`; use [`checked_align_up`] when
/// the input may be attacker-controlled or otherwise unbounded.
///
/// **Pre-condition:** `align` is non-zero and is a power of two.
#[inline(always)]
pub const fn align_up(size: usize, align: usize) -> usize {
    debug_assert!(align > 0 && align.is_power_of_two());
    (size + (align - 1)) & !(align - 1)
}

/// Overflow-checked variant of [`align_up`]. Returns `None` if rounding
/// up would overflow `usize`.
///
/// **Pre-condition:** `align` is non-zero and is a power of two.
#[inline(always)]
pub const fn checked_align_up(size: usize, align: usize) -> Option<usize> {
    debug_assert!(align > 0 && align.is_power_of_two());
    match size.checked_add(align - 1) {
        Some(v) => Some(v & !(align - 1)),
        None => None,
    }
}

// ---------------------------------------------------------------------------
// Specializations to MAX_ALIGN
// ---------------------------------------------------------------------------

/// Round `size` up to the next multiple of [`MAX_ALIGN`]. Wraps if `size`
/// exceeds `usize::MAX - (MAX_ALIGN - 1)`; use [`checked_align_max`] when
/// the input may be attacker-controlled or otherwise unbounded.
#[inline(always)]
pub const fn align_max(size: usize) -> usize {
    align_up(size, MAX_ALIGN)
}

/// Overflow-checked variant of [`align_max`]. Returns `None` if rounding
/// up would overflow `usize`.
#[inline(always)]
pub const fn checked_align_max(size: usize) -> Option<usize> {
    checked_align_up(size, MAX_ALIGN)
}

// ---------------------------------------------------------------------------
// u32 variant (specializer layout)
// ---------------------------------------------------------------------------

/// `u32` variant of [`align_up`].
///
/// **Pre-condition:** `align` is non-zero and is a power of two.
#[inline(always)]
pub fn align_up_u32(offset: u32, align: u32) -> u32 {
    debug_assert!(align > 0 && align.is_power_of_two());
    (offset + align - 1) & !(align - 1)
}
