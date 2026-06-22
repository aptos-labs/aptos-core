// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Misc runtime types and layout constants.
//!
//! Object descriptors live in [`crate::heap::object_descriptor`].

pub use mono_move_core::{VEC_DATA_OFFSET, VEC_LENGTH_OFFSET};

// ---------------------------------------------------------------------------
// Step result
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum StepResult {
    /// There are more instructions to execute.
    Continue,
    /// The outermost function has returned — execution is complete.
    Done,
    /// Execution hit an `Abort` / `AbortMsg` micro-op. The code is the
    /// u64 abort code; the message is populated when `AbortMsg` ran.
    Aborted { code: u64, message: Option<String> },
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

pub(crate) const DEFAULT_STACK_SIZE: usize = 1024 * 1024; // 1 MiB

pub(crate) const DEFAULT_HEAP_SIZE: usize = 10 * 1024 * 1024; // 10 MiB

/// Maximum size of an `AbortMsg` message, in bytes.
/// TODO(cleanup): make this configurable in some VM config.
pub(crate) const ABORT_MESSAGE_SIZE_LIMIT: usize = 1024;

/// Byte offset of `saved_pc` within frame metadata.
pub(crate) const META_SAVED_PC_OFFSET: usize = 0;
/// Byte offset of `saved_fp` within frame metadata.
pub(crate) const META_SAVED_FP_OFFSET: usize = 8;
/// Byte offset of `saved_func_ptr` within frame metadata.
pub(crate) const META_SAVED_FUNC_PTR_OFFSET: usize = 16;

/// Marker written into the `descriptor_id` field of a forwarded object during GC.
pub(crate) const FORWARDED_MARKER: u32 = u32::MAX;
