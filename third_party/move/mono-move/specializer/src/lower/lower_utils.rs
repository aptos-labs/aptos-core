// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Small shared helpers when lowering.

/// Whether the byte ranges `[a, a + size_a)` and `[b, b + size_b)` overlap.
#[inline]
pub(super) fn ranges_overlap(a: u32, size_a: u32, b: u32, size_b: u32) -> bool {
    a < b.saturating_add(size_b) && b < a.saturating_add(size_a)
}
