// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Integration tests for [`MemoryRegion`].

use mono_move_alloc::MemoryRegion;

#[test]
fn contains_range_bounds() {
    let region = MemoryRegion::new_zeroed(64);
    let base = region.as_ptr();
    // Wrapping arithmetic allows test pointers outside the allocation.
    let at = |offset: usize| base.wrapping_add(offset);

    assert!(region.contains_range(base, 0));
    assert!(region.contains_range(base, 64));
    assert!(region.contains_range(at(8), 56));
    assert!(region.contains_range(at(64), 0));

    assert!(!region.contains_range(base, 65));
    assert!(!region.contains_range(at(8), 57));
    assert!(!region.contains_range(at(64), 1));
    assert!(!region.contains_range(at(65), 0));
    assert!(!region.contains_range(base.wrapping_sub(1), 1));
    assert!(!region.contains_range(at(8), usize::MAX));
}
