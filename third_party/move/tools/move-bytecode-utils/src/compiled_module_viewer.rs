// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use move_binary_format::CompiledModule;
use move_core_types::language_storage::ModuleId;
use std::{borrow::Borrow, fmt::Debug};

pub trait CompiledModuleViewer {
    type Error: Debug;
    type Item: Borrow<CompiledModule>;

    fn view_compiled_module(&self, id: &ModuleId) -> Result<Option<Self::Item>, Self::Error>;
}
