// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Destack pipeline: converts Move bytecode into stackless execution IR.

mod analysis;
mod instr_utils;
pub mod optimize;
mod slot_alloc;
mod ssa_conversion;
mod ssa_function;
mod translate;
mod type_conversion;

use crate::stackless_exec_ir::ModuleIR;
use anyhow::{bail, Result};
use move_binary_format::CompiledModule;
use move_vm_types::loaded_data::struct_name_indexing::StructNameIndex;

/// Verify, convert, and optimize a compiled module into stackless execution IR.
///
/// `struct_name_table` maps `StructHandleIndex` ordinals to globally unique
/// `StructNameIndex` values, used to convert bytecode-level struct references
/// into runtime `Type` representations.
pub fn destack(module: CompiledModule, struct_name_table: &[StructNameIndex]) -> Result<ModuleIR> {
    if let Err(e) = move_bytecode_verifier::verify_module(&module) {
        bail!("bytecode verification failed: {:#}", e);
    }

    let mut module_ir = translate::translate_module(module, struct_name_table)?;
    optimize::optimize_module(&mut module_ir);
    Ok(module_ir)
}
