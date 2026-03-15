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
    maintenance_config::{MaintenanceConfig, TypeTreeSizeLimits},
    GlobalArenaPool,
};
use dashmap::DashMap;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

// Submodules: to split implementation into smaller pieces.
mod identifiers;
use identifiers::IdentifierInternerKey;
pub use identifiers::NameRef;
mod executable_ids;
pub use executable_ids::ExecutableIdRef;
use executable_ids::{ExecutableId, ExecutableIdInternerKey};
mod types;
use types::{RuntimeTypeInfo, TypeInternerKey, TypeListInternerKey};
pub use types::{TypeError, TypeListRef, TypeRef, TypeTreeSize};

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
    types: DashMap<TypeInternerKey, GlobalArenaPtr<RuntimeTypeInfo>, ahash::RandomState>,
    type_lists: DashMap<
        TypeListInternerKey,
        GlobalArenaPtr<[GlobalArenaPtr<RuntimeTypeInfo>]>,
        ahash::RandomState,
    >,
    type_tree_size_limits: TypeTreeSizeLimits,
}

/// RAII guard for the maintenance phase providing exclusive write access.
///
/// Only one maintenance context can exist at a time, ensuring exclusive
/// access to the internal state for maintenance operations. The write lock
/// is held for the lifetime of this guard and automatically released when
/// dropped.
pub struct MaintenanceGuard<'a> {
    /// Reference to the caches stored in context.
    ctx: &'a Context,
    /// Pool of all arenas managing global allocations.
    global_arena: &'a GlobalArenaPool,
    /// Configuration controlling maintenance behavior.
    #[allow(dead_code)]
    maintenance_config: &'a MaintenanceConfig,

    /// Write guard that disallows obtaining concurrent execution
    /// guard. **Must** be dropped last.
    _guard: RwLockWriteGuard<'a, ()>,
}

/// RAII guard for the execution phase providing concurrent read access
/// to shared interned data and exclusive access to a dedicated arena.
/// Multiple execution contexts can exist simultaneously across threads.
pub struct ExecutionGuard<'a> {
    /// Reference to the caches stored in context.
    ctx: &'a Context,
    /// Arena dedicated for this execution guard with exclusive access.
    /// During execution, data can be allocated here without contention.
    global_arena: GlobalArenaShard<'a>,

    /// Read guard preventing maintenance phase, but allowing concurrent
    /// execution phases. **Must** be dropped last.
    _guard: RwLockReadGuard<'a, ()>,
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
                types: DashMap::default(),
                type_lists: DashMap::default(),
                type_tree_size_limits: maintenance_config.type_tree_size_limits.clone(),
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

impl<'a> MaintenanceGuard<'a> {
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

    /// Returns the number of entries in interner's map for types.
    pub fn interned_types_count(&self) -> usize {
        self.ctx.types.len()
    }

    /// Returns the number of entries in interner's map for type lists.
    pub fn interned_type_lists_count(&self) -> usize {
        self.ctx.type_lists.len()
    }

    /// Resets all caches that store pointers to the arenas, and then resets
    /// the arenas as well.
    pub fn reset_arena_pool(&mut self) {
        // SAFETY: Arena is only reset **after**, so clearing all caches is
        // safe.
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

impl<'a> ExecutionGuard<'a> {}

//
// Only private APIs below.
// ------------------------

impl<'a> MaintenanceGuard<'a> {
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
            types,
            type_lists,
            type_tree_size_limits: _,
        } = self.ctx;

        type_lists.clear();
        types.clear();
        executable_ids.clear();
        identifiers.clear();
    }
}

impl<'a> ExecutionGuard<'a> {}
