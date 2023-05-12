// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::values::Value;
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{value::MoveTypeLayout, vm_status::StatusCode};

/// Any value stored in global storage must implement this trait.
pub trait Store {
    /// Consumes the stored type and returns its serialized version.
    fn into_bytes(self) -> Option<Vec<u8>>;

    /// Returns the number of bytes the stored type occupies in serialized
    /// format.
    fn num_bytes(&self) -> usize;
}

impl Store for Vec<u8> {
    fn into_bytes(self) -> Option<Vec<u8>> {
        Some(self)
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
        let num_bytes = value
            .serialized_size(&layout)
            .ok_or_else(|| PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR))?;
        Ok(Self::Cached(value, layout, num_bytes))
    }

    pub fn from_bytes(bytes: Vec<u8>) -> PartialVMResult<Self> {
        Ok(Self::Serialized(bytes))
    }
}

impl Store for Resource {
    fn into_bytes(self) -> Option<Vec<u8>> {
        match self {
            Self::Serialized(bytes) => Some(bytes),
            Self::Cached(value, layout, _) => value.simple_serialize(&layout),
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
