// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The speculative value the mono VM stores in the multi-version map: a raw
//! pointer into a frozen heap, kept alive by the `Arc` it carries.

use aptos_mvhashmap::types as mv_types;
use aptos_types::{block_executor::speculative_value::MVValue, write_set::WriteOpKind};
use mono_move_runtime::{Heap, Version};
use std::{ptr::NonNull, sync::Arc};

/// A mono value stored in the multi-version map: a transaction's write
/// (pointing into that transaction's heap), or a base value the resource
/// provider deserialized from storage and published at the storage version
/// (pointing into its own heap).
#[derive(Clone)]
pub enum MonoValue {
    /// A live value: a pointer into the frozen heap carried alongside it.
    /// The interpreter's deep-copy discipline guarantees the object graph
    /// under `ptr` lives entirely in that one heap, so the `Arc` is the only
    /// keep-alive needed; it drops when the map entry is removed and every
    /// reader has let go of its clone.
    Write {
        ptr: NonNull<u8>,
        /// Creation or modification, derived from the read the write shadows
        /// (base values are modifications by convention).
        kind: WriteOpKind,
        heap: Arc<Heap>,
    },
    /// The value was deleted (moved out), or does not exist in storage (for
    /// entries published at the storage version).
    Deletion,
}

// SAFETY: The pointer targets the carried heap, which is frozen: execution
// (or base-value deserialization) completed before the value was published,
// and nothing mutates or GCs the heap afterwards. Pointers into it are
// stable, read-only sharing across threads is sound, and drop only frees the
// buffer.
unsafe impl Send for MonoValue {}
unsafe impl Sync for MonoValue {}

impl MVValue for MonoValue {
    fn eq_value(&self, _other: &Self) -> bool {
        // Conservative: equality would need a deep value comparison, and
        // pointer identity never matches across incarnations (each execution
        // writes into a fresh heap). A false negative only forces
        // re-validation of readers.
        false
    }

    fn eq_metadata(&self, _other: &Self) -> bool {
        // Only reachable for resource group metadata, which the mono path
        // does not produce.
        false
    }

    fn bytes_len(&self) -> Option<usize> {
        // The serialized size is unknown until materialization.
        None
    }

    fn is_deletion(&self) -> bool {
        matches!(self, MonoValue::Deletion)
    }

    fn write_op_kind(&self) -> WriteOpKind {
        match self {
            MonoValue::Write { kind, .. } => kind.clone(),
            MonoValue::Deletion => WriteOpKind::Deletion,
        }
    }
}

/// Converts Block-STM's version into the mono mirror carried by
/// [`mono_move_runtime::StorageRead`] and validated against on re-reads.
pub fn to_mono_version(version: mv_types::Version) -> Version {
    match version {
        Ok((txn_idx, incarnation)) => Version::Write {
            txn_idx,
            incarnation,
        },
        Err(mv_types::StorageVersion) => Version::Storage,
    }
}
