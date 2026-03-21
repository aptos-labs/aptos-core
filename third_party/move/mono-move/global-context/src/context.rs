// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Core types and implementation for the global execution context.
//!
//! # Safety Contract & Design Principles
//!
//! ## Two-Phase State Machine
//!
//! The global context operates in two exclusive phases:
//!
//! 1. **Execution Phase**
//!
//!    Multiple [`ExecutionGuard`] guards can be held concurrently. Guards
//!    provide read-only access to caches to obtain or allocate data, but never
//!    deallocate, making arena allocations stable (no reset or drop possible).
//!    Pointers returned from the guard are valid for the guard's lifetime.
//!
//! 2. **Maintenance Phase**
//!    A single exclusive [`MaintenanceGuard`] guard exists with write access
//!    via [`RwLockWriteGuard`]. During this phase caches can be reset. Because
//!    no execution contexts can co-exist, there can be no dangling pointers,
//!    making deallocation safe.
//!
//! ## Global Allocation Race Window
//!
//! When interning, allocation happens **outside the [`DashMap`] lock** to
//! reduce contention. This creates a race window where multiple threads may
//! allocate the same interned data. This is intentional and safe:
//!
//!   - Only one pointer is stored in the interner's map.
//!   - Duplicate allocations leak but are bounded (interning converges).
//!   - Trade-off: minor memory waste for lower lock contention.

use crate::{
    alloc::{GlobalArenaPtr, GlobalArenaShard},
    maintenance_config::MaintenanceConfig,
    GlobalArenaPool,
};
use dashmap::DashMap;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::{
    hash::{Hash, Hasher},
    marker::PhantomData,
    ptr,
    ptr::NonNull,
};

// Submodules: to split implementation into smaller pieces.
mod identifiers;
use identifiers::IdentifierInternerKey;
mod executable_ids;
pub use executable_ids::ExecutableId;
use executable_ids::ExecutableIdInternerKey;

/// Global execution context with a two-phase state machine.
///
/// # Phases
///
/// 1. **Execution Phase**: Multiple [`ExecutionGuard`] guards can be
///    obtained concurrently across threads. Each worker gets access to global
///    arena. This allows parallel execution where each thread can read from
///    the shared caches, allocate data, and safely use raw pointers (addresses
///    are guaranteed to be stable).
///
/// 2. **Maintenance Phase**: A single [`MaintenanceGuard`] guard provides
///    exclusive write access for maintenance operations (scheduled between
///    execution phases, e.g., between blocks of transactions) such as cache
///    cleanup or data deallocation.
pub struct GlobalContext {
    /// Shared caches storing interned data, executables.
    ctx: Context,
    /// Pool of arenas (assigned per execution worker). Each worker gets
    /// exclusive access to their arena to avoid contention.
    global_arena: GlobalArenaPool,
    /// Configuration controlling maintenance behavior.
    maintenance_config: MaintenanceConfig,
    /// Lock to switch between execution and maintenance modes:
    ///   - Read lock: execution phase.
    ///   - Write lock: maintenance phase.
    phase: RwLock<()>,
}

/// Shared context containing interned data structures. Global arena where the
/// data is allocated is kept separately.
struct Context {
    identifiers: DashMap<IdentifierInternerKey, GlobalArenaPtr<str>, ahash::RandomState>,
    executable_ids:
        DashMap<ExecutableIdInternerKey, GlobalArenaPtr<ExecutableId>, ahash::RandomState>,
}

/// RAII guard for the maintenance phase providing exclusive write access.
///
/// Only one maintenance context can exist at a time, ensuring exclusive
/// access to the internal state for maintenance operations. The write lock
/// is held for the lifetime of this guard and automatically released when
/// dropped.
pub struct MaintenanceGuard<'ctx> {
    /// Reference to the caches stored in context.
    ctx: &'ctx Context,
    /// Pool of all arenas managing global allocations.
    global_arena: &'ctx GlobalArenaPool,
    /// Configuration controlling maintenance behavior.
    #[allow(dead_code)]
    maintenance_config: &'ctx MaintenanceConfig,

    /// Write guard that disallows obtaining concurrent execution
    /// guard. **Must** be dropped last.
    _guard: RwLockWriteGuard<'ctx, ()>,
}

/// RAII guard for the execution phase providing concurrent read access
/// to shared interned data and exclusive access to a dedicated arena.
/// Multiple execution contexts can exist simultaneously across threads.
pub struct ExecutionGuard<'ctx> {
    /// Reference to the caches stored in context.
    ctx: &'ctx Context,
    /// Arena dedicated for this execution guard with exclusive access.
    /// During execution, data can be allocated here without contention.
    global_arena: GlobalArenaShard<'ctx>,

    /// Read guard preventing maintenance phase, but allowing concurrent
    /// execution phases. **Must** be dropped last.
    _guard: RwLockReadGuard<'ctx, ()>,
}

/// A scoped reference to data obtained from [`ExecutionGuard`] and is guaranteed
/// to be alive until the guard is dropped.
///
/// # Safety model
///
/// The reference lifetime is tied to the lifetime of the [`ExecutionGuard`].
/// It is guaranteed that the data it points to is kept alive as long as the
/// guard is alive.
///
/// The pointer stored behind the reference is guaranteed to be valid and
/// safe to dereference.
#[repr(transparent)]
pub struct ArenaRef<'guard, T: ?Sized> {
    ptr: NonNull<T>,
    _guard: PhantomData<&'guard ()>,
}

impl GlobalContext {
    /// Creates a new global context with the specified number of workers that
    /// can acquire [`ExecutionGuard`] and default maintenance config.
    ///
    /// # Panics
    ///
    /// Panics if the number of workers is 0, greater than 128 or is not a
    /// power of two.
    pub fn with_num_execution_workers(num_workers: usize) -> Self {
        Self::with_num_execution_workers_and_config(num_workers, MaintenanceConfig::default())
    }

    /// Creates a new global context with the specified number of execution
    /// workers that can acquire [`ExecutionGuard`] and the maintenance config.
    ///
    /// # Panics
    ///
    /// Panics if the number of workers is 0, greater than 128 or is not a
    /// power of two.
    pub fn with_num_execution_workers_and_config(
        num_workers: usize,
        maintenance_config: MaintenanceConfig,
    ) -> Self {
        assert!(
            num_workers > 0 && num_workers <= 128,
            "Number of workers must be between 1 and 128, got {num_workers}"
        );
        assert!(
            num_workers.is_power_of_two(),
            "Number of workers must be a power of two, got {num_workers}"
        );

        Self {
            ctx: Context {
                identifiers: DashMap::default(),
                executable_ids: DashMap::default(),
            },
            global_arena: GlobalArenaPool::with_num_arenas(num_workers),
            maintenance_config,
            phase: RwLock::new(()),
        }
    }

    /// Transitions to maintenance mode by obtaining a [`MaintenanceGuard`]
    /// guard. Only one maintenance context can be held at a time, providing
    /// exclusive access to the internal state for maintenance operations. No
    /// execution context can be held concurrently.
    ///
    /// Returns [`None`] if [`ExecutionGuard`] is currently held or there is
    /// an ongoing maintenance.
    #[must_use]
    pub fn try_maintenance_context(&self) -> Option<MaintenanceGuard<'_>> {
        let _guard = self.phase.try_write()?;

        Some(MaintenanceGuard {
            ctx: &self.ctx,
            global_arena: &self.global_arena,
            maintenance_config: &self.maintenance_config,
            _guard,
        })
    }

    /// Transitions to execution mode by obtaining an [`ExecutionGuard`] guard
    /// and locking the arena for the given worker. Multiple execution contexts
    /// can be held concurrently across threads for different workers.
    ///
    /// Returns [`None`] if
    ///   - there is an ongoing maintenance phase,
    ///   - the arena for this worker has already been locked.
    ///
    /// # Panics
    ///
    /// Panics if the worker ID is out of bounds when trying to get an arena
    /// from the pool.
    #[must_use]
    pub fn try_execution_context(&self, worker_id: usize) -> Option<ExecutionGuard<'_>> {
        let _guard = self.phase.try_read()?;

        Some(ExecutionGuard {
            ctx: &self.ctx,
            global_arena: self.global_arena.try_lock_arena(worker_id)?,
            _guard,
        })
    }
}

impl<'ctx> MaintenanceGuard<'ctx> {
    /// Returns the total number of bytes used across all arenas in the global
    /// arena pool.
    pub fn global_arena_allocated_bytes_sum(&self) -> usize {
        (0..self.global_arena.num_arenas())
            .map(|idx| self.global_arena.allocated_bytes(idx))
            .sum()
    }

    /// Returns the number of entries in interner's map for identifiers.
    pub fn interned_identifiers_count(&self) -> usize {
        self.ctx.identifiers.len()
    }

    /// Returns the number of entries in interner's map for executable IDs.
    pub fn interned_executable_ids_count(&self) -> usize {
        self.ctx.executable_ids.len()
    }

    /// Resets all caches that store pointers to the arenas, and then resets
    /// the arenas as well.
    pub fn reset_arena_pool(&mut self) {
        // SAFETY: Arena is only reset **after** caches are cleared.
        unsafe {
            self.reset_all_caches();
        }

        // SAFETY: We are in maintenance phase, so there are no concurrent
        // execution contexts and therefore no live pointers to arena other
        // than ones that were stored in caches. All caches were cleared (see
        // above), and so there are no live pointers making reset safe.
        unsafe {
            self.global_arena.reset_all_arenas_unchecked();
        }
    }
}

impl<'ctx> ExecutionGuard<'ctx> {}

//
// Only private APIs below.
// ------------------------

impl<'ctx> MaintenanceGuard<'ctx> {
    /// Clears all caches stored in [`Context`]. Triggered when the global
    /// arena requires a full reset (and thus, any cache that stores pointers
    /// to that arena must be invalidated).
    ///
    /// # Safety
    ///
    /// Should be called **before** arena backing allocations is reset or
    /// dropped.
    unsafe fn reset_all_caches(&mut self) {
        // Exhaustive destructuring so that there is a compile-time error if a
        // new field is added without being explicitly handled here.
        //
        // CRITICAL: caches can store pointers to arenas which can be reset, it
        // is important to ensure these caches are cleared before that.
        let Context {
            identifiers,
            executable_ids,
        } = self.ctx;

        identifiers.clear();
        executable_ids.clear();
    }
}

impl<'ctx> ExecutionGuard<'ctx> {
    /// Returns a reference scoped to the lifetime of the guard.
    ///
    /// # Safety
    ///
    /// The caller must ensure the pointer points to stable data and will not
    /// be deallocated during guard's lifetime.
    unsafe fn arena_ref<'guard, T: ?Sized>(
        &'guard self,
        ptr: GlobalArenaPtr<T>,
    ) -> ArenaRef<'guard, T>
    where
        'ctx: 'guard,
    {
        ArenaRef {
            ptr: ptr.into_inner(),
            _guard: Default::default(),
        }
    }
}

impl<'guard, T: ?Sized> ArenaRef<'guard, T> {
    /// Returns the raw address of the allocation of the pointer. For testing
    /// purposes only.
    pub fn raw_address_for_testing(&self) -> usize {
        self.ptr.as_ptr().addr()
    }
}

// Arena reference uses pointer hash. Because of interning, pointer hash
// equality implies structural hash equality (ignoring hash collisions).
impl<'guard, T: ?Sized> Hash for ArenaRef<'guard, T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.ptr.hash(state)
    }
}

// Arena reference uses pointer equality. Because of interning, pointer
// equality implies structural equality.
impl<'guard, T: ?Sized> PartialEq for ArenaRef<'guard, T> {
    fn eq(&self, other: &Self) -> bool {
        ptr::eq(self.ptr.as_ptr(), other.ptr.as_ptr())
    }
}

impl<'guard, T: ?Sized> Eq for ArenaRef<'guard, T> {}

// Arena reference can be duplicated with bitwise copy.
impl<'guard, T: ?Sized> Copy for ArenaRef<'guard, T> {}

impl<'guard, T: ?Sized> Clone for ArenaRef<'guard, T> {
    fn clone(&self) -> Self {
        *self
    }
}
