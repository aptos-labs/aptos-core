// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::CompiledModule;
use move_core_types::language_storage::ModuleId;
use move_vm_runtime::ModuleStorage;
use std::{borrow::Borrow, sync::Arc};

pub trait CompiledModuleView {
    type Item: Borrow<CompiledModule>;

    // TODO: Consider using address and module name instead of module id.
    fn view_compiled_module(&self, id: &ModuleId) -> anyhow::Result<Option<Self::Item>>;
}

impl<M: ModuleStorage> CompiledModuleView for M {
    type Item = Arc<CompiledModule>;

    fn view_compiled_module(&self, id: &ModuleId) -> anyhow::Result<Option<Self::Item>> {
        Ok(if self.check_module_exists(id.address(), id.name())? {
            Some(self.fetch_deserialized_module(id.address(), id.name())?)
        } else {
            None
        })
    }
}
