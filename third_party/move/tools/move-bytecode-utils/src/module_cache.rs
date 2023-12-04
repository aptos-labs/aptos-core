// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use move_binary_format::CompiledModule;
use move_core_types::language_storage::ModuleId;
use std::{borrow::Borrow, fmt::Debug};

/// A persistent storage that can fetch the bytecode for a given module id
/// TODO: do we want to implement this in a way that allows clients to cache struct layouts?
pub trait GetModule {
    type Error: Debug;
    type Item: Borrow<CompiledModule>;

    fn get_module_by_id(&self, id: &ModuleId) -> Result<Option<Self::Item>, Self::Error>;
}
