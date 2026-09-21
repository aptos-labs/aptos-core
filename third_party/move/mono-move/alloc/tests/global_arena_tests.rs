// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Integration tests for [`GlobalArenaPool`] and [`GlobalArenaPtr`].

use mono_move_alloc::GlobalArenaPool;

/// Smallest and largest pooled sizes, 32 KiB and 4 MiB.
const MIN_POOLED: usize = 1 << 15;
const MAX_POOLED: usize = 1 << 22;

#[test]
fn test_alloc() {
    let pool = GlobalArenaPool::default();

    let arena = pool.lock_arena(0);
    let ptr = arena.alloc(42u64);
    assert_eq!(unsafe { *ptr.as_ref_unchecked() }, 42u64);
}

#[test]
fn test_alloc_str() {
    let pool = GlobalArenaPool::default();

    let arena = pool.lock_arena(0);
    let ptr = arena.alloc_str("hello");
    assert_eq!(unsafe { ptr.as_ref_unchecked() }, "hello");
}

#[test]
fn test_alloc_slice_copy() {
    let pool = GlobalArenaPool::default();

    let arena = pool.lock_arena(0);
    let ptr = arena.alloc_slice_copy(&[1u32, 2u32, 3u32]);
    assert_eq!(unsafe { ptr.as_ref_unchecked() }, &[1u32, 2u32, 3u32]);
}

#[test]
fn test_num_arenas() {
    let pool = GlobalArenaPool::with_num_arenas(4);
    assert_eq!(pool.num_arenas(), 4);
}

#[test]
fn test_default() {
    let pool = GlobalArenaPool::default();
    assert!(pool.num_arenas() >= 1);
}

/// Under Miri every take allocates, so no two of these tests' pointers ever
/// match. Reuse is what the assertions below are about, so they do not run.
fn reuse_is_observable() -> bool {
    !cfg!(miri)
}

#[test]
fn test_returned_region_comes_back() {
    if !reuse_is_observable() {
        return;
    }
    for size in [MIN_POOLED, MAX_POOLED] {
        let pool = GlobalArenaPool::default();
        let arena = pool.lock_arena(0);

        let region = arena.take_region(size);
        assert_eq!(region.len(), size);
        let ptr = region.as_ptr();
        arena.return_region(region);

        let taken = arena.take_region(size);
        assert_eq!(taken.as_ptr(), ptr);
        assert_eq!(taken.len(), size);
    }
}

#[test]
fn test_bucket_holds_more_than_one_region() {
    if !reuse_is_observable() {
        return;
    }
    let pool = GlobalArenaPool::default();
    let arena = pool.lock_arena(0);

    let first = arena.take_region(MIN_POOLED);
    let second = arena.take_region(MIN_POOLED);
    let mut parked = vec![first.as_ptr(), second.as_ptr()];
    assert_ne!(parked[0], parked[1]);
    arena.return_region(first);
    arena.return_region(second);

    let mut taken = vec![
        arena.take_region(MIN_POOLED).as_ptr(),
        arena.take_region(MIN_POOLED).as_ptr(),
    ];
    parked.sort();
    taken.sort();
    assert_eq!(parked, taken);
}

#[test]
fn test_buckets_do_not_mix_sizes() {
    if !reuse_is_observable() {
        return;
    }
    let pool = GlobalArenaPool::default();
    let arena = pool.lock_arena(0);

    let big = arena.take_region(MAX_POOLED);
    let ptr = big.as_ptr();
    arena.return_region(big);

    let small = arena.take_region(MIN_POOLED);
    assert_ne!(small.as_ptr(), ptr);
    assert_eq!(small.len(), MIN_POOLED);

    // The big region is still parked in its own bucket.
    assert_eq!(arena.take_region(MAX_POOLED).as_ptr(), ptr);
}

#[test]
fn test_unpooled_sizes_are_served_exactly() {
    let pool = GlobalArenaPool::default();
    let arena = pool.lock_arena(0);

    // Not a power of two, below the floor, and above the ceiling. Returning
    // one drops it, which must leave the pooled buckets alone.
    for size in [MIN_POOLED + 8, MIN_POOLED / 2, MAX_POOLED * 2] {
        let region = arena.take_region(size);
        assert_eq!(region.len(), size);
        arena.return_region(region);
    }

    let pooled = arena.take_region(MIN_POOLED);
    assert_eq!(pooled.len(), MIN_POOLED);
}

#[test]
fn test_lock_arena_try() {
    let pool = GlobalArenaPool::with_num_arenas(2);
    let arena = pool.try_lock_arena(0);
    assert!(arena.is_some());

    assert!(pool.try_lock_arena(0).is_none());
    assert!(pool.try_lock_arena(1).is_some());
}
