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
use mono_move_core::types::InternedType;
use mono_move_global_context::ExecutionGuard;
use move_binary_format::CompiledModule;

/// Verify, convert, and optimize a compiled module into stackless execution IR.
///
/// `struct_types` maps `StructHandleIndex` ordinals to pre-resolved interned
/// type pointers, used to convert bytecode-level struct references into
/// interned `Type` representations. `None` entries denote handles the
/// orchestrator could not resolve; touching one during conversion is an
/// error.
///
/// TODO: consider wrapping the `(CompiledModule, struct_types)` pair in a
/// `CompiledModuleWithContext` that owns the module and the resolved
/// per-handle interned types (and, later, the full signature pool interned
/// up front). That would let every downstream pass take one argument instead
/// of threading the slice through, and keep the invariant that `struct_types`
/// indexes the module's own handle table encoded in the type.
pub fn destack(
    module: CompiledModule,
    guard: &ExecutionGuard<'_>,
    struct_types: &[Option<InternedType>],
) -> Result<ModuleIR> {
    if let Err(e) = move_bytecode_verifier::verify_module(&module) {
        bail!("bytecode verification failed: {:#}", e);
    }

    let mut module_ir = translate::translate_module(module, guard, struct_types)?;
    optimize::optimize_module(&mut module_ir);
    Ok(module_ir)
}
