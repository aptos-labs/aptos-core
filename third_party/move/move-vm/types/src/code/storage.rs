// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::indices::ModuleIdx;
use bytes::Bytes;
use move_binary_format::errors::VMResult;

/// Storage that contains serialized modules. Clients can implement this trait for their own
/// backends, so that [ModuleStorage] can be built on top of it.
pub trait ModuleBytesStorage {
    /// Returns bytes of modules, if they exist, and [None] otherwise. The error is returned in
    /// case there are some storage-related issues.
    fn fetch_module_bytes(&self, idx: &ModuleIdx) -> VMResult<Option<Bytes>>;
}
