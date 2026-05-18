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

use crate::stackless_exec_ir::ModuleIR;
use anyhow::{bail, Result};
use mono_move_core::{Interner, PreparedModule};
use move_binary_format::CompiledModule;

/// Verify, convert, and optimize a compiled module into stackless execution IR.
pub fn destack(module: CompiledModule, interner: &impl Interner) -> Result<ModuleIR> {
    if let Err(e) = move_bytecode_verifier::verify_module(&module) {
        bail!("bytecode verification failed: {:#}", e);
    }

    let module = PreparedModule::build(module, interner)?;
    let mut module_ir = translate::translate_module(module, interner)?;
    optimize::optimize_module(&mut module_ir);
    Ok(module_ir)
}
