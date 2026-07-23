// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Destack pipeline: converts Move bytecode into stackless execution IR.

mod analysis;
pub mod optimize;
mod slot_alloc;
mod ssa_conversion;
mod ssa_function;
mod test_utils;
mod translate;

use crate::{gas, stackless_exec_ir::ModuleIR};
use mono_move_core::{ExecutionErrorKind, Interner, IntoExecutionError, PreparedModule, VMResult};
use move_binary_format::CompiledModule;
use thiserror::Error;

// TODO(cleanup): figure out how to best surface verification errors — e.g.
// refactor the bytecode verifier to return typed errors instead of `VMError`.
#[derive(Debug, Error)]
#[error("bytecode verification failed: {0}")]
struct VerificationError(move_binary_format::errors::VMError);

impl IntoExecutionError for VerificationError {
    fn kind(&self) -> ExecutionErrorKind {
        // TODO(cleanup): map to `VerificationFailed` once it exists.
        ExecutionErrorKind::Placeholder
    }
}

/// Verify, convert, and optimize a compiled module into stackless execution IR.
pub fn destack(module: CompiledModule, interner: &impl Interner) -> VMResult<ModuleIR> {
    move_bytecode_verifier::verify_module(&module).map_err(VerificationError)?;

    let module = PreparedModule::build(module, interner)?;
    let mut module_ir = translate::translate_module(module, interner)?;
    optimize::optimize_module(&mut module_ir);

    // Gas instrumentation: emit a per-block cost formula for each function.
    // TODO(metering): could be hoisted before optimization, at the cost of
    // over-approximating (charging for instructions that optimization removes).
    gas::instrument(&mut module_ir, interner)?;

    // Debug-mode failsafe: verify xfer invariants hold after optimization.
    #[cfg(debug_assertions)]
    for func in module_ir.functions.iter().flatten() {
        analysis::assert_xfer_invariants_on_final_ir(func)?;
    }
    Ok(module_ir)
}
