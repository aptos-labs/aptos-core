// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::CompiledModule;
use move_core_types::language_storage::ModuleId;
use std::borrow::Borrow;

pub trait CompiledModuleView {
    type Item: Borrow<CompiledModule>;

    // TODO: Consider using address and module name instead of module id.
    fn view_compiled_module(&self, id: &ModuleId) -> anyhow::Result<Option<Self::Item>>;
}
