// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use move_binary_format::CompiledModule;
use move_core_types::language_storage::ModuleId;
use std::borrow::Borrow;

pub trait CompiledModuleView {
    type Item: Borrow<CompiledModule>;

    // TODO: Consider using address and module name instead of module id.
    fn view_compiled_module(&self, id: &ModuleId) -> anyhow::Result<Option<Self::Item>>;
}
