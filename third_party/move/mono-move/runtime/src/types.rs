// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Misc runtime types and layout constants.
//!
//! Object descriptors live in [`crate::heap::object_descriptor`].

pub use mono_move_core::{VEC_DATA_OFFSET, VEC_LENGTH_OFFSET};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Size of the interpreter's call stack. This bounds call depth, so it decides
/// which programs fail with a stack overflow. Changing it changes execution
/// results, and every node has to agree on the value.
pub(crate) const DEFAULT_STACK_SIZE: usize = 1024 * 1024; // 1 MiB

/// Size of a session's heap. This bounds how much a transaction can have live
/// at once, so it decides which programs fail with an out-of-memory error.
/// Changing it changes execution results, and every node has to agree on the
/// value.
pub(crate) const DEFAULT_HEAP_SIZE: usize = 4 * 1024 * 1024; // 4 MiB

/// Initial capacity, in elements, of a vector allocated lazily on its first
/// `push_back`.
pub(crate) const VEC_PUSHBACK_INIT_CAPACITY: u64 = 4;

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
