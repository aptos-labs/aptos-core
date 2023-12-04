// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use move_binary_format::CompiledModule;
use move_core_types::language_storage::ModuleId;
use std::{borrow::Borrow, fmt::Debug};

/// Allows to view the bytecode for a given module id.
/// TODO: do we want to implement this in a way that allows clients to cache struct layouts?
pub trait ModuleViewer {
    type Error: Debug;
    type Item: Borrow<CompiledModule>;

    fn view_module(&self, id: &ModuleId) -> Result<Self::Item, Self::Error>;
}

pub trait CompiledModuleViewer: ModuleViewer<Error = anyhow::Error, Item = CompiledModule> {}

impl<V: ModuleViewer<Error = anyhow::Error, Item = CompiledModule>> CompiledModuleViewer for V {}
