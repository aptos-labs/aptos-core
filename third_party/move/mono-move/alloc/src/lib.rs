// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! This crate defines arena and pointer types implementations.
//!
//! # Global arena
//!
//! [`GlobalArenaPool`] is used for bump-allocating long-lived data (types,
//! identifiers, etc.) that outlives executables and is not subject to code
//! upgrade invalidation. Returns [`GlobalArenaPtr<T>`] which is a raw pointer
//! to arena's allocation.
//!
//! ## Safety model
//!
//! [`GlobalArenaPtr<T>`] exposes **only immutable access** to the allocated
//! data, and it is sound to share the pointers across threads. Dereferencing
//! the pointer is a separate concern captured in two **unsafe** contracts:
//!
//! - [`GlobalArenaPtr::as_ref_unchecked`] - caller must ensure that the arena
//!   that owns the data has not been reset or dropped, and that the pointer
//!   has not been invalidated.
//! - [`GlobalArenaPool::reset_all_arenas_unchecked`] - caller must ensure
//!   there are no live pointers derived from the allocations that is about to
//!   be reset and that it is called from single-threaded context (exclusive
//!   access required).
//!
//! # Executable arena
//!
//! [`ExecutableArena`] is used for bump-allocating data that is tied to a
//! particular executable version (e.g., function bytecode). Returns
//! [`ExecutableArenaPtr<T>`] which is a raw pointer to arena's allocation.
//!
//! ## Safety model
//!
//! When executable is dropped, the arena is also invalidated. The following
//! **unsafe** contracts must be enforced.
//!
//! - [`ExecutableArenaPtr::as_ref_unchecked`] - caller must ensure the owning
//!   executable (and therefore its arena) is still alive.
//! - **Drop contract**: the [`ExecutableArena`] is owned by the executable, so
//!   the pointers to arena and the arena itself are dropped together in the
//!   right order.
//!
//! # Explicit memory management
//!
//! [`LeakedBoxPtr<T>`] is used for data that cannot be bulk-allocated and
//! requires explicit memory management. For example, not all executables in
//! the cache need to be freed at the same time, but only stale old versions.
//!
//! ## Safety model
//!
//! - [`LeakedBoxPtr::as_ref_unchecked`] - caller must ensure the pointer has
//!   not yet been freed.
//! - [`LeakedBoxPtr::free_unchecked`] - caller must ensure no other references
//!   to the pointee exist, the pointer is freed at most once exclusively.
//!
//! # Interaction with block execution
//!
//! The safety model is enforced by making execution a two-phased state machine
//! where there is an **execution phase** and **maintenance phase**. During
//! execution phase, it is guaranteed that:
//!
//!   1. Global arena is not reset.
//!   2. Executables and their arenas are not dropped.
//!   3. Leaked pointers are not freed.
//!   4. No pointers outlive execution phase.
//!
//! During maintenance phase there is exclusive access to all arenas and data.
//! Maintenance phase can reset or drop arenas, and free leaked pointers.

mod executable_arena;
mod global_arena;
mod leaked;

pub use executable_arena::{ExecutableArena, ExecutableArenaPtr};
pub use global_arena::{GlobalArenaPool, GlobalArenaPtr, GlobalArenaShard};
pub use leaked::LeakedBoxPtr;
