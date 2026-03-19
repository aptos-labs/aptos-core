// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Pipeline orchestrator. Single entry point called from lib.rs.

use anyhow::Result;
use crate::ir::ModuleIR;
use move_binary_format::CompiledModule;
use move_vm_types::loaded_data::struct_name_indexing::StructNameIndex;

/// Convert + optimize a compiled module into stackless execution IR.
pub fn run_pipeline(
    module: CompiledModule,
    struct_name_table: &[StructNameIndex],
) -> Result<ModuleIR> {
    let mut module_ir = crate::convert::convert_module(module, struct_name_table)?;
    crate::optimize::optimize_module(&mut module_ir);
    Ok(module_ir)
}
