// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Native implementations for `aptos_experimental::native_position`.
//!
//! This crate owns:
//! - `NativePositionContext`: the session extension that buffers per-TX
//!   position writes (create / update / remove). Drained at session
//!   finalize into the `VMChangeSet` position bucket.
//! - [`NativePosition`]: the Rust enum mirroring `Position` in Move,
//!   plus its compact-binary codec.
//! - Native function implementations invoked by the Move module.
//!
//! Move-side reads from the in-memory position store are deferred to
//! milestone 2; the read-side plumbing (resolver trait, per-TX read
//! cache) is not present yet.
//!
//! The subsystem is documented in `PLAN_native_position.md`.

pub mod context;
pub mod natives;
pub mod position;

pub use context::{NativePositionContext, PositionTxCache};
pub use natives::all_natives;
pub use position::{NativePosition, PositionKey};
