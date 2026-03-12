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
//!    provide read-only access to interners via [`RwLockReadGuard`] that may
//!    allocate new data but never deallocates, making arena allocations
//!    stable (no reset or drop possible). Pointers returned from the guard are
//!    valid for the guard's lifetime.
//!
//! 2. **Maintenance Phase**
//!    A single exclusive [`MaintenanceGuard`] guard exists with write access
//!    via [`RwLockWriteGuard`]. During this phase caches can be reset. Because
//!    no execution contexts can co-exist, there can be no dangling pointers,
//!    making deallocation safe.

use crate::maintenance_config::MaintenanceConfig;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

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
    /// Shared caches protected by read-write lock.
    ctx: RwLock<Context>,
    // TODO: global arena here.
    /// Configuration controlling maintenance behavior.
    maintenance_config: MaintenanceConfig,
}

/// Shared context containing interned data structures. Global arena where the
/// data is allocated is kept separately.
struct Context {
    // TODO: add caches here.
}

/// RAII guard for the maintenance phase providing exclusive write access.
///
/// Only one maintenance context can exist at a time, ensuring exclusive
/// access to the internal state for maintenance operations. The write lock
/// is held for the lifetime of this guard and automatically released when
/// dropped.
pub struct MaintenanceGuard<'a> {
    /// Write guard on shared data (disallows obtaining concurrent execution
    /// guard).
    #[allow(dead_code)]
    ctx_guard: RwLockWriteGuard<'a, Context>,

    // TODO: mut reference to global arena here.
    /// Configuration controlling maintenance behavior.
    #[allow(dead_code)]
    maintenance_config: &'a MaintenanceConfig,
}

/// RAII guard for the execution phase providing concurrent read access
/// to shared interned data and exclusive access to a dedicated arena.
/// Multiple execution contexts can exist simultaneously across threads.
pub struct ExecutionGuard<'a> {
    /// Read guard on shared interned data (prevents maintenance phase).
    #[allow(dead_code)]
    ctx_guard: RwLockReadGuard<'a, Context>,
    // TODO: reference to global arena (shard) here.
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
            ctx: RwLock::new(Context {}),
            maintenance_config,
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
        // TODO: acquire arena as well.
        Some(MaintenanceGuard {
            ctx_guard: self.ctx.try_write()?,
            maintenance_config: &self.maintenance_config,
        })
    }

    /// Transitions to execution mode by obtaining an [`ExecutionGuard`] guard
    /// and locking the arena for the given worker. Multiple execution contexts
    /// can be held concurrently across threads for different workers.
    ///
    /// Returns [`None`] if
    ///   - there is an ongoing maintenance phase.
    pub fn try_execution_context(&self, _worker_id: usize) -> Option<ExecutionGuard<'_>> {
        let ctx_guard = self.ctx.try_read()?;
        // TODO: acquire arena as well.
        Some(ExecutionGuard { ctx_guard })
    }
}

impl<'a> MaintenanceGuard<'a> {}

impl<'a> ExecutionGuard<'a> {}

//
// Only private APIs below.
// ------------------------

impl<'a> MaintenanceGuard<'a> {}

impl<'a> ExecutionGuard<'a> {}
