// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Integration tests for [`GlobalArenaPool`] and [`GlobalArenaPtr`].

use mono_move_alloc::GlobalArenaPool;

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

#[test]
fn test_lock_arena_try() {
    let pool = GlobalArenaPool::with_num_arenas(2);
    let arena = pool.try_lock_arena(0);
    assert!(arena.is_some());

    assert!(pool.try_lock_arena(0).is_none());
    assert!(pool.try_lock_arena(1).is_some());
}
