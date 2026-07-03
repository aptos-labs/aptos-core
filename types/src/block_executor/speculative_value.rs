// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::write_set::{TransactionWrite, WriteOpKind};
use fail::fail_point;
use move_core_types::value::MoveTypeLayout;
use triomphe::Arc;

/// A value stored in the multi-version data-structure during speculative
/// execution. Captures exactly the value-level operations the versioned map
/// performs on a stored value, so the map itself stays agnostic to the
/// concrete value representation of the VM that produced it.
pub trait MVValue: Clone + Send + Sync {
    /// Whether two values are equal for push-validation and the incarnation-0
    /// pre-write check.
    fn eq_value(&self, other: &Self) -> bool;

    /// Whether the metadata of two values is equal. Used when only metadata is
    /// versioned (currently resource group metadata).
    fn eq_metadata(&self, other: &Self) -> bool;

    /// Serialized size of the value in bytes, or None for a deletion / absent value.
    fn bytes_len(&self) -> Option<usize>;

    /// Whether the value is a deletion.
    fn is_deletion(&self) -> bool;

    /// The kind of the write (creation / modification / deletion). Used by the
    /// unsync map to check group operations against the existing contents.
    fn write_op_kind(&self) -> WriteOpKind;
}

// TODO[agg_v2](cleanup): consider adding `DoesntExist` variant.
// Currently, "not existing value" is represented as Deletion.
#[derive(Debug, PartialEq, Eq)]
pub enum ValueWithLayout<V> {
    // When we read from storage, but don't have access to layout, we can only store the raw value.
    // This should never be returned to the user, before exchange is performed.
    RawFromStorage(Arc<V>),
    // We've used the optional layout, and applied exchange to the storage value.
    // The type layout is Some if there is a delayed field in the resource.
    // The type layout is None if there is no delayed field in the resource.
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

impl<V: TransactionWrite> ValueWithLayout<V> {
    pub fn write_op_kind(&self) -> WriteOpKind {
        match self {
            ValueWithLayout::RawFromStorage(value) => value.write_op_kind(),
            ValueWithLayout::Exchanged(value, _) => value.write_op_kind(),
        }
    }

    pub fn bytes_len(&self) -> Option<usize> {
        fail_point!("value_with_layout_bytes_len", |_| { Some(10) });
        match self {
            ValueWithLayout::RawFromStorage(value) | ValueWithLayout::Exchanged(value, _) => {
                value.bytes().map(|b| b.len())
            },
        }
    }

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

    /// Consumes the value and returns the underlying owned value, dropping the
    /// layout (if any). Avoids cloning when the value is uniquely referenced.
    pub fn into_value(self) -> V
    where
        V: Clone,
    {
        let value = match self {
            ValueWithLayout::RawFromStorage(value) | ValueWithLayout::Exchanged(value, _) => value,
        };
        Arc::try_unwrap(value).unwrap_or_else(|value| (*value).clone())
    }
}

impl<V: TransactionWrite + PartialEq + Send + Sync> MVValue for ValueWithLayout<V> {
    fn eq_value(&self, other: &Self) -> bool {
        // Both must be exchanged with no layout, and their values must be equal.
        // Layouts pass validation only if both are None; otherwise validation
        // pessimistically fails, avoiding potentially costly layout comparisons.
        use ValueWithLayout::*;
        matches!((self, other), (Exchanged(a, None), Exchanged(b, None)) if a == b)
    }

    fn eq_metadata(&self, other: &Self) -> bool {
        // Metadata comparison only passes against an exchanged previous value.
        matches!(self, ValueWithLayout::Exchanged(..))
            && self.extract_value().as_state_value_metadata()
                == other.extract_value().as_state_value_metadata()
    }

    fn bytes_len(&self) -> Option<usize> {
        fail_point!("value_with_layout_bytes_len", |_| { Some(10) });
        match self {
            ValueWithLayout::RawFromStorage(value) | ValueWithLayout::Exchanged(value, _) => {
                value.bytes().map(|b| b.len())
            },
        }
    }

    fn is_deletion(&self) -> bool {
        self.extract_value().is_deletion()
    }

    fn write_op_kind(&self) -> WriteOpKind {
        self.extract_value().write_op_kind()
    }
}
