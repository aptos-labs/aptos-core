// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Core types and implementation for the global execution context.

use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Placeholder struct for global context data.
#[derive(Debug, Default)]
struct Context {
    // TODO: add data here
}

/// Global execution context with a two-phase state machine.
///
/// # Phases
///
/// 1. **Execution Phase**: Multiple [`ExecutionContext`] guards can be obtained
///    concurrently across threads. This allows parallel transaction execution
///    where each thread can read from shared caches and concurrently allocate
///    data.
///
/// 2. **Maintenance Phase**: A single [`MaintenanceContext`] guard provides
///    exclusive write access for inter-block maintenance operations such as
///    cache cleanup or data de-allocation.
pub struct GlobalContext {
    inner: RwLock<Context>,
}

impl GlobalContext {
    /// Creates a new global context with an empty internal state.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(Context {}),
        }
    }

    /// Transitions to execution mode by obtaining a read guard.
    ///
    /// Multiple execution contexts can be held concurrently across
    /// threads, enabling parallel transaction execution.
    ///
    /// Returns [None] is a [`MaintenanceContext`] is currently held.
    pub fn execution_context(&self) -> Option<ExecutionContext<'_>> {
        Some(ExecutionContext {
            guard: self.inner.try_read()?,
        })
    }

    /// Transitions to maintenance mode by obtaining a write guard.
    ///
    /// Only one maintenance context can be held at a time, providing
    /// exclusive access to the internal state for maintenance operations.
    ///
    /// Returns [None] is a [`ExecutionContext`] is currently held.
    pub fn maintenance_context(&self) -> Option<MaintenanceContext<'_>> {
        Some(MaintenanceContext {
            guard: self.inner.try_write()?,
        })
    }
}

/// RAII guard for the execution phase providing concurrent read access
/// or data allocation.
///
/// Multiple execution contexts can exist simultaneously across threads,
/// allowing parallel transaction execution. The read lock is held for
/// the lifetime of this guard and automatically released when dropped.
#[derive(Debug)]
pub struct ExecutionContext<'a> {
    #[allow(dead_code)]
    guard: RwLockReadGuard<'a, Context>,
}

/// RAII guard for the maintenance phase providing exclusive write access.
///
/// Only one maintenance context can exist at a time, ensuring exclusive
/// access to the internal state for maintenance operations. The write lock
/// is held for the lifetime of this guard and automatically released when dropped.
#[derive(Debug)]
pub struct MaintenanceContext<'a> {
    #[allow(dead_code)]
    guard: RwLockWriteGuard<'a, Context>,
}
