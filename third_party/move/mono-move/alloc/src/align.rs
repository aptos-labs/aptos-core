// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The VM-wide maximum alignment.

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
