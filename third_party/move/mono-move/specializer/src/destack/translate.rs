// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Conversion pipeline: Bytecode → SSA → instruction fusion → slot allocation.

use super::ssa_conversion::SsaConverter;
use crate::stackless_exec_ir::{FunctionIR, ModuleIR};
use anyhow::Result;
use mono_move_core::{Interner, PreparedModule};
use move_binary_format::access::ModuleAccess;

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
pub fn translate_module(module: PreparedModule, interner: &impl Interner) -> Result<ModuleIR> {
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
            let param_types = module.interned_types_at(handle.parameters);
            let local_types = module.interned_types_at(code.locals);
            let num_params = param_types.len() as u16;
            let num_locals = local_types.len() as u16;
            let local_types = param_types
                .iter()
                .chain(local_types.iter())
                .copied()
                .collect::<Vec<_>>();

            // Pass: Bytecode -> Intra-Block SSA -> Fusion
            let converter = SsaConverter::new(local_types, interner);
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

    Ok(ModuleIR { module, functions })
}
