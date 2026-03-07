// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Global execution context for MonoMove.
//!
//! This crate provides a two-phase state machine for managing the global state:
//! - **Execution phase**: Multiple [`ExecutionContext`] guards can be held concurrently
//!   across threads for parallel transaction execution.
//! - **Maintenance phase**: A single exclusive [`MaintenanceContext`] guard for inter-block
//!   maintenance operations.

mod alloc;
pub use alloc::{ArenaGuard, ArenaPool, GlobalArena};
pub mod configs;
mod context;
pub use context::{ExecutionContext, GlobalContext, MaintenanceContext};
pub(crate) mod counters;
mod executable;
pub use executable::{Executable, Function};
mod executable_cache;
pub use executable_cache::ExecutableCache;
mod interner;
mod types;
pub mod version;

pub use types::{ExecutableId, FunctionId, StructId, Type, TypeList};
