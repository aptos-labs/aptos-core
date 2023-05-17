// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::values::Value;
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{value::MoveTypeLayout, vm_status::StatusCode};

/// Any value stored in global storage must implement this trait.
pub trait Store {
    /// Consumes the stored type and returns its serialized version. If
    /// serialization fails an error is returned.
    fn into_bytes(self) -> PartialVMResult<Vec<u8>>;

    /// Consumes the stored type and returns its serialized version. If
    /// serialization fails an error is returned.
    fn as_bytes(&self) -> PartialVMResult<Vec<u8>>;

    /// Returns the number of bytes the stored type occupies in a serialized
    /// format.
    fn num_bytes(&self) -> usize;
}

impl<T: Clone + AsRef<T> + Store> Store for &T {
    fn into_bytes(self) -> PartialVMResult<Vec<u8>> {
        self.as_ref().clone().into_bytes()
    }

    fn as_bytes(&self) -> PartialVMResult<Vec<u8>> {
        (**self).as_bytes()
    }

    fn num_bytes(&self) -> usize {
        (**self).num_bytes()
    }
}

// Byte arrays can be stored in global storage.
impl Store for Vec<u8> {
    fn into_bytes(self) -> PartialVMResult<Vec<u8>> {
        Ok(self)
    }

    fn as_bytes(&self) -> PartialVMResult<Vec<u8>> {
        Ok(self.clone())
    }

    fn num_bytes(&self) -> usize {
        self.len()
    }
}

/// Wrapper around any Move resource which is stored to global storage.
#[derive(Debug)]
pub enum Resource {
    /// Resource serialized as bytes.
    Serialized(Vec<u8>),
    /// Non-serialized resource, with a type layout and its size in bytes.
    Cached(Value, MoveTypeLayout, usize),
}

impl Resource {
    pub fn from_value(value: Value, layout: MoveTypeLayout) -> PartialVMResult<Self> {
        // Using serialized size is preferred because it avoids allocations.
        let num_bytes = value.serialized_size(&layout).ok_or_else(|| {
            PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR).with_message(format!(
                "Error when calculating serialized size of {}.",
                value,
            ))
        })?;
        Ok(Self::Cached(value, layout, num_bytes))
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self::Serialized(bytes)
    }
}

// Resource can be stored to global storage.
impl Store for Resource {
    fn into_bytes(self) -> PartialVMResult<Vec<u8>> {
        match self {
            Self::Serialized(bytes) => Ok(bytes),
            Self::Cached(value, layout, _) => value.simple_serialize(&layout).ok_or_else(|| {
                PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)
                    .with_message(format!("Error when serializing {} into bytes.", value))
            }),
        }
    }

    fn as_bytes(&self) -> PartialVMResult<Vec<u8>> {
        match self {
            Self::Serialized(bytes) => Ok(bytes.clone()),
            Self::Cached(value, layout, _) => value.simple_serialize(layout).ok_or_else(|| {
                PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)
                    .with_message(format!("Error when serializing {} as bytes.", value))
            }),
        }
    }

    fn num_bytes(&self) -> usize {
        match self {
            Self::Serialized(bytes) => bytes.len(),
            Self::Cached(_, _, num_bytes) => *num_bytes,
        }
    }
}

/// Wrapper around any Move resource loaded from the global storage.
#[derive(Debug)]
pub enum ResourceRef {
    /// Resource serialized as bytes.
    Serialized(Vec<u8>),
    /// Non-serialized resource.
    Cached(Value),
}
