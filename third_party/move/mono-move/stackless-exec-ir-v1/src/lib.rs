// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub mod arg_regs;
pub mod convert_v1;
pub mod display;
pub mod ir;
pub mod lower;
pub mod lowering_context;
pub mod micro_ops_display;
pub mod optimize_v1;
pub mod type_conversion;

use anyhow::{bail, Result};
use ir::ModuleIR;
use move_binary_format::CompiledModule;
use move_vm_types::loaded_data::struct_name_indexing::StructNameIndex;

/// Run the V1 conversion + optimization pipeline on a compiled module.
///
/// `struct_name_table` maps `StructHandleIndex` ordinals to globally unique
/// `StructNameIndex` values, used to convert bytecode-level struct references
/// into runtime `Type` representations.
pub fn run_pipeline(
    module: CompiledModule,
    struct_name_table: &[StructNameIndex],
) -> Result<ModuleIR> {
    if let Err(e) = move_bytecode_verifier::verify_module(&module) {
        bail!("bytecode verification failed: {:#}", e);
    }

    let mut module_ir = convert_v1::convert_module_v1(module, struct_name_table);
    optimize_v1::optimize_module_v1(&mut module_ir);
    arg_regs::introduce_arg_registers_module(&mut module_ir);
    Ok(module_ir)
}
