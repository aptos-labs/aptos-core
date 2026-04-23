// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Conversion pipeline: Bytecode → SSA → instruction fusion → slot allocation.

use super::{
    ssa_conversion::SsaConverter,
    type_conversion::{convert_sig_token, convert_sig_tokens},
};
use crate::stackless_exec_ir::{FuncSignature, FunctionIR, ModuleIR};
use anyhow::Result;
use mono_move_core::types::InternedType;
use mono_move_global_context::ExecutionGuard;
use move_binary_format::{access::ModuleAccess, CompiledModule};

/// Convert an entire compiled module to stackless IR.
///
/// The caller is responsible for running the bytecode verifier beforehand
/// if the module comes from an untrusted source. The conversion relies on
/// the following verifier-guaranteed invariants:
///
/// - **Stack balance**: every pop has a matching push; the stack is empty at
///   basic-block boundaries and `Ret` drains exactly the declared return values.
/// - **Type consistency**: operand types on the stack match what each instruction
///   expects (e.g. `ReadRef` sees a reference, arithmetic operands are the same
///   integer type, `FreezeRef` sees a `&mut`).
/// - **Index bounds**: all pool indices (`StructDefinitionIndex`,
///   `FieldHandleIndex`, `FunctionHandleIndex`, `ConstantPoolIndex`,
///   `SignatureIndex`, variant indices, etc.) are within their respective tables.
/// - **Struct/variant field shape**: `Pack`/`Unpack` target structs with
///   `Declared` fields; variant instructions target `DeclaredVariants` with
///   valid variant and field indices.
/// - **Branch target validity**: every branch offset maps to a valid bytecode
///   position inside the function.
/// - **Local initialization**: locals are assigned via `StLoc` before any
///   `CopyLoc`/`MoveLoc`; `MoveLoc` is not used on an already-moved local.
/// - **Function signature correctness**: the number of arguments on the stack
///   matches the callee's declared parameter count, and return-type signatures
///   are well-formed.
/// - **Type parameter bounds**: `TypeParameter(idx)` indices fall within the
///   type-parameter list of the enclosing generic context.
/// - **Reference safety**: the borrow checker guarantees that freed slots
///   truly hold dead values, so type-keyed slot recycling is sound.
pub fn translate_module(
    module: CompiledModule,
    guard: &ExecutionGuard<'_>,
    struct_types: &[Option<InternedType>],
) -> Result<ModuleIR> {
    let functions = module
        .function_defs
        .iter()
        .map(|fdef| {
            let Some(code) = fdef.code.as_ref() else {
                return Ok(None);
            };
            let handle = module.function_handle_at(fdef.function);
            let name_idx = handle.name;
            let handle_idx = fdef.function;
            let param_sig_toks = &module.signature_at(handle.parameters).0;
            let local_sig_toks = &module.signature_at(code.locals).0;
            let num_params = param_sig_toks.len() as u16;
            let num_locals = local_sig_toks.len() as u16;
            let local_types: Vec<InternedType> = param_sig_toks
                .iter()
                .chain(local_sig_toks.iter())
                .map(|tok| convert_sig_token(tok, guard, struct_types))
                .collect::<Result<_>>()?;

            // Pass: Bytecode -> Intra-Block SSA -> Fusion
            let converter = SsaConverter::new(local_types, guard, struct_types);
            let ssa = converter
                .convert_function(&module, &code.code)?
                .with_fusion_passes();

            // Pass: Greedy Slot Allocation (consumes SSA, remaps in-place)
            let alloc = super::slot_alloc::allocate_slots(ssa)?;

            Ok(Some(FunctionIR {
                name_idx,
                handle_idx,
                num_params,
                num_locals,
                num_home_slots: alloc.num_home_slots,
                num_xfer_slots: alloc.num_xfer_slots,
                blocks: alloc.blocks,
                home_slot_types: alloc.home_slot_types,
            }))
        })
        .collect::<Result<Vec<_>>>()?;

    // Module-level signature caches. The lowering pass reads these directly
    // instead of re-walking signature tokens per call site.
    let handle_signatures = collect_handle_signatures(&module, guard, struct_types)?;
    let instantiation_signatures = collect_instantiation_signatures(&module, guard, struct_types)?;

    Ok(ModuleIR {
        module,
        functions,
        handle_signatures,
        instantiation_signatures,
    })
}

/// Pre-computes `FuncSignature` for every function handle in the module.
///
/// TODO: convert the module's signature pool once and look up signatures by
/// index here instead of re-walking the same `SignatureToken` slices per
/// handle/instantiation. Handles that share a signature index currently hit the
/// interner repeatedly, which is both wasted work; a one-pass pool conversion
/// collapses this to a single interning per unique signature.
fn collect_handle_signatures(
    module: &CompiledModule,
    guard: &ExecutionGuard<'_>,
    struct_types: &[Option<InternedType>],
) -> Result<Vec<FuncSignature>> {
    module
        .function_handles
        .iter()
        .map(|handle| {
            let param_types = convert_sig_tokens(
                &module.signature_at(handle.parameters).0,
                guard,
                struct_types,
            )?;
            let ret_types =
                convert_sig_tokens(&module.signature_at(handle.return_).0, guard, struct_types)?;
            Ok(FuncSignature {
                param_types,
                ret_types,
            })
        })
        .collect()
}

/// Pre-computes `FuncSignature` for every function instantiation in the module.
/// Today this mirrors the base handle's signature; future work (generic
/// instantiation substitution) will replace each entry with the concrete
/// substituted signature.
fn collect_instantiation_signatures(
    module: &CompiledModule,
    guard: &ExecutionGuard<'_>,
    struct_types: &[Option<InternedType>],
) -> Result<Vec<FuncSignature>> {
    module
        .function_instantiations
        .iter()
        .map(|inst| {
            let handle = module.function_handle_at(inst.handle);
            let param_types = convert_sig_tokens(
                &module.signature_at(handle.parameters).0,
                guard,
                struct_types,
            )?;
            let ret_types =
                convert_sig_tokens(&module.signature_at(handle.return_).0, guard, struct_types)?;
            Ok(FuncSignature {
                param_types,
                ret_types,
            })
        })
        .collect()
}
