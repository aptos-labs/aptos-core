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
//!
//! ## Global Allocation Race Window
//!
//! When interning, allocation happens **outside the [`dashmap::DashMap`]
//! lock** to reduce contention. This creates a race window where multiple
//! threads may allocate the same interned data. This is intentional and safe:
//!
//!   - Only one pointer is stored in the interner's map.
//!   - Duplicate allocations leak but are bounded (interning converges).
//!   - Trade-off: minor memory waste for lower lock contention.

mod identifiers;

pub use identifiers::ExecutableId;
use std::hash::{Hash, Hasher};
mod interner;

use crate::{
    alloc::{GlobalArenaPtr, GlobalArenaShard},
    context::interner::DashMapInterner,
    maintenance_config::MaintenanceConfig,
    GlobalArenaPool,
};
use move_core_types::{
    account_address::AccountAddress, identifier::IdentStr, language_storage::ModuleId,
};
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::marker::PhantomData;

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
    identifiers: DashMapInterner<str>,
    executable_ids: DashMapInterner<ExecutableId>,
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
    #[allow(dead_code)]
    ctx: &'a Context,
    /// Arena dedicated for this execution guard with exclusive access.
    /// During execution, data can be allocated here without contention.
    #[allow(dead_code)]
    global_arena: GlobalArenaShard<'a>,

    /// Read guard preventing maintenance phase, but allowing concurrent
    /// execution phases. **Must** be dropped last.
    _guard: RwLockReadGuard<'a, ()>,
}

/// Scoped reference returned by public [`ExecutionGuard`] APIs. The lifetime
/// enforces compile-time guarantee that the execution guard is alive when
/// holding the reference. Hence, there is no way to invalidate the underlying
/// pointer because only [`MaintenanceGuard`] can deallocate, but it cannot be
/// acquired as the [`ExecutionGuard`] is held.
pub struct Ref<'a, T: ?Sized> {
    ptr: GlobalArenaPtr<T>,
    _guard: PhantomData<&'a ()>,
}

impl<'a, T: ?Sized> Ref<'a, T> {
    /// Casts this reference to a raw pointer.
    pub fn as_raw_ptr(&self) -> *const T {
        self.ptr.as_raw_ptr()
    }
}

impl<'a, T: ?Sized> Hash for Ref<'a, T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.ptr.hash(state)
    }
}

impl<'a, T: ?Sized> PartialEq for Ref<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        self.ptr == other.ptr
    }
}

impl<'a, T: ?Sized> Eq for Ref<'a, T> {}

impl<'a, T: ?Sized> Copy for Ref<'a, T> {}

impl<'a, T: ?Sized> Clone for Ref<'a, T> {
    fn clone(&self) -> Self {
        *self
    }
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
                identifiers: DashMapInterner::default(),
                executable_ids: DashMapInterner::default(),
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
            self.global_arena.reset_unchecked();
        }
    }
}

impl<'a> ExecutionGuard<'a> {
    /// Interns Move identifier as a string and returns a reference to it. The
    /// reference is valid for the lifetime of the [`ExecutionGuard`].
    pub fn intern_identifier<'b>(&'b self, identifier: &IdentStr) -> Ref<'b, str>
    where
        'a: 'b,
    {
        // TODO:
        //   Consider checking that the identifier size is within bounds. While
        //   CompiledModule / CompiledScript deserializer enforces 256 byte
        //   limit (in new config), when coming from deserialized TypeTag from
        //   transaction payload there is no bound. It is not a big problem,
        //   but just makes spam attacks easier to intern some dummy data in the
        //   pool. In general, for type tag interning we might want to enforce
        //   that the modules which are specified actually exist on-chain. In
        //   existing VM we already do that to get ability information, but not
        //   here (for now), so that we ensure that there is no spam that can
        //   get in. However, there still can be a problem with speculative
        //   module publishing: if we speculatively intern new names, but the
        //   publish actually fails, we end up with spam on-chain.
        //   Note: this DoS is only possible via `init_module`. If we remove it
        //   or ensure no speculative data even for names ever get on-chain, we
        //   limit interned set to the on-chain data, so for DoS one actually
        //   has to publish modules (expensive).
        let str = identifier.as_str();

        if let Some(ptr) = self.ctx.identifiers.get(str) {
            // SAFETY: We read the pointer from the interner's map, so it must
            // have been allocated and is still valid **provided** global arena
            // has not been flushed. The maintenance guard ensures all caches
            // are flushed. Hence, we can use the guard's lifetime for it.
            return Ref {
                ptr,
                _guard: PhantomData,
            };
        };

        let Ref { ptr, _guard } = self.alloc_str(str);
        Ref {
            // SAFETY: We have just allocated this pointer. Hence, it is safe
            // to dereference its contents when inserting into the interner.
            ptr: unsafe { self.ctx.identifiers.insert(ptr) },
            _guard,
        }
    }

    /// Interns [`ModuleId`] as [`ExecutableId`] and returns a reference to it.
    /// The reference is valid for the lifetime of the [`ExecutionGuard`].
    pub fn intern_module_id<'b>(&'b self, module_id: &ModuleId) -> Ref<'b, ExecutableId>
    where
        'a: 'b,
    {
        self.intern_address_name(&module_id.address, &module_id.name)
    }

    /// Interns [`AccountAddress`]-[`Identifier`] pair as [`ExecutableId`] and
    /// returns a reference to it. The reference is valid for the lifetime of
    /// the [`ExecutionGuard`].
    pub fn intern_address_name<'b>(
        &'b self,
        address: &AccountAddress,
        name: &IdentStr,
    ) -> Ref<'b, ExecutableId>
    where
        'a: 'b,
    {
        if let Some(ptr) = self.ctx.executable_ids.get(&(address, name)) {
            // SAFETY: We read the pointer from the interner's map, so it must
            // have been allocated and is still valid **provided** global arena
            // has not been flushed. The maintenance guard ensures all caches
            // are flushed. Hence, we can use the guard's lifetime for it.
            return Ref {
                ptr,
                _guard: PhantomData,
            };
        };

        let name = self.intern_identifier(name);
        let Ref { ptr, _guard } = self.alloc_executable_id(*address, name);
        Ref {
            // SAFETY: We have just allocated this pointer. Hence, it is safe
            // to dereference its contents when inserting into the interner.
            ptr: unsafe { self.ctx.executable_ids.insert(ptr) },
            _guard,
        }
    }
}

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
        // CRITICAL:
        //   - Enforce that the reset order is enforced for any new cache.
        let Context {
            identifiers,
            executable_ids,
        } = self.ctx;

        executable_ids.reset();
        identifiers.reset();
    }
}

impl<'a> ExecutionGuard<'a> {}
