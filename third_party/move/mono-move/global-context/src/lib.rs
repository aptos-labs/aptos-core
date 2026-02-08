// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Global execution context for MonoMove.
//!
//! This crate provides a two-phase state machine for managing the global state:
//! - **Execution phase**: Multiple [`ExecutionContext`] guards can be held concurrently
//!   across threads for parallel transaction execution.
//! - **Maintenance phase**: A single exclusive [`MaintenanceContext`] guard for inter-block
//!   maintenance operations.
//!
//! It also provides various concurrent interner implementations for benchmarking.

mod context;
pub mod interner_impls;

pub use context::{ExecutionContext, GlobalContext, MaintenanceContext};
