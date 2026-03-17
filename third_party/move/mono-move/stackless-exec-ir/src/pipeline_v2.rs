// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! V2 pipeline orchestrator. Single entry point called from lib.rs.

use anyhow::Result;
use crate::ir::ModuleIR;
use move_binary_format::CompiledModule;
use move_vm_types::loaded_data::struct_name_indexing::StructNameIndex;

/// Convert + optimize using the V2 pipeline.
pub fn run_v2_pipeline(
    module: CompiledModule,
    struct_name_table: &[StructNameIndex],
) -> Result<ModuleIR> {
    let mut module_ir = crate::convert_v2::convert_module_v2(module, struct_name_table)?;
    crate::optimize_v2::optimize_module_v2(&mut module_ir);
    Ok(module_ir)
}
