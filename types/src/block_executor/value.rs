// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Defines speculative value interfaces (writes produced by transactions)
//! used by Block-STM.

use crate::write_set::{TransactionWrite, WriteOpKind};
use fail::fail_point;
use move_core_types::value::MoveTypeLayout;
use triomphe::Arc;

/// A value stored in the multi-version data-structure during Block-STM
/// speculative execution. Captures exactly the value-level operations the
/// versioned map performs on a stored value.
pub trait SpeculativeValue: Clone + Send + Sync {
    /// Whether two values are equal by value.
    fn eq_value(&self, other: &Self) -> bool;

    /// Whether the metadata of two values is equal.
    ///
    /// Used for resource group metadata.
    fn eq_metadata(&self, other: &Self) -> bool;

    /// Serialized size of the value in bytes, or None for a deletion / absent value.
    fn bytes_len(&self) -> Option<usize>;

    /// Whether the value is a deletion.
    fn is_deletion(&self) -> bool {
        matches!(self.write_op_kind(), WriteOpKind::Deletion)
    }

    /// The kind of the write (creation, modification, or deletion).
    fn write_op_kind(&self) -> WriteOpKind;
}

/// Value representation used by Block-STM for legacy Move VM.
#[derive(Debug, PartialEq, Eq)]
pub enum ValueWithLayout<V> {
    /// The value was read from storage, there is no layout. Never returned to
    /// the user, before exchange is performed (see below).
    RawFromStorage(Arc<V>),
    /// Storage value that ran exchange or a write. The layout if set indicates
    /// there are delayed fields inside and [`None`] otherwise.
    Exchanged(Arc<V>, Option<Arc<MoveTypeLayout>>),
}

impl<T> Clone for ValueWithLayout<T> {
    fn clone(&self) -> Self {
        match self {
            ValueWithLayout::RawFromStorage(value) => {
                ValueWithLayout::RawFromStorage(value.clone())
            },
            ValueWithLayout::Exchanged(value, layout) => {
                ValueWithLayout::Exchanged(value.clone(), layout.clone())
            },
        }
    }
}

impl<V> ValueWithLayout<V> {
    pub fn extract_value_no_layout(&self) -> &V {
        match self {
            ValueWithLayout::RawFromStorage(value) => value.as_ref(),
            ValueWithLayout::Exchanged(value, None) => value.as_ref(),
            ValueWithLayout::Exchanged(_, Some(_)) => panic!("Unexpected layout"),
        }
    }

    /// Returns a reference to the underlying value, regardless of whether a layout is present.
    /// Unlike `extract_value_no_layout`, this method does not panic when a layout is present.
    pub fn extract_value(&self) -> &V {
        match self {
            ValueWithLayout::RawFromStorage(value) => value.as_ref(),
            ValueWithLayout::Exchanged(value, _) => value.as_ref(),
        }
    }

    /// Returns true if this value has a layout (i.e., contains delayed fields).
    pub fn has_layout(&self) -> bool {
        matches!(self, ValueWithLayout::Exchanged(_, Some(_)))
    }
}

impl<V> SpeculativeValue for ValueWithLayout<V>
where
    V: TransactionWrite + PartialEq + Send + Sync,
{
    fn eq_value(&self, other: &Self) -> bool {
        // Both must be exchanged with no layout, and their values must be equal.
        // Layouts pass validation only if both are None; otherwise validation
        // pessimistically fails, avoiding potentially costly layout comparisons.
        use ValueWithLayout::*;
        matches!((self, other), (Exchanged(a, None), Exchanged(b, None)) if a == b)
    }

    fn eq_metadata(&self, other: &Self) -> bool {
        // Metadata comparison only passes against exchanged values.
        use ValueWithLayout::*;
        matches!((self, other), (Exchanged(a, _), Exchanged(b, _)) if a.as_state_value_metadata() == b.as_state_value_metadata())
    }

    fn bytes_len(&self) -> Option<usize> {
        fail_point!("value_with_layout_bytes_len", |_| { Some(10) });
        match self {
            ValueWithLayout::RawFromStorage(value) | ValueWithLayout::Exchanged(value, _) => {
                value.bytes().map(|b| b.len())
            },
        }
    }

    fn write_op_kind(&self) -> WriteOpKind {
        self.extract_value().write_op_kind()
    }
}
