// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Conversion pipeline: Bytecode → SSA → instruction fusion → slot allocation.

use crate::{
    ir::{FunctionIR, ModuleIR},
    ssa_conversion::SsaConverter,
    type_conversion::convert_sig_tokens,
};
use anyhow::Result;
use move_binary_format::{access::ModuleAccess, file_format::SignatureToken, CompiledModule};
use move_vm_types::loaded_data::struct_name_indexing::StructNameIndex;

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
    struct_name_table: &[StructNameIndex],
) -> Result<ModuleIR> {
    let functions = module
        .function_defs
        .iter()
        .filter_map(|fdef| {
            fdef.code.as_ref().map(|code| -> Result<FunctionIR> {
                let handle = module.function_handle_at(fdef.function);
                let name_idx = handle.name;
                let handle_idx = fdef.function;
                let param_sig_toks = &module.signature_at(handle.parameters).0;
                let local_sig_toks = &module.signature_at(code.locals).0;
                let num_params = param_sig_toks.len() as u16;
                let num_locals = local_sig_toks.len() as u16;
                let all_sig_toks: Vec<SignatureToken> = param_sig_toks
                    .iter()
                    .chain(local_sig_toks.iter())
                    .cloned()
                    .collect();
                // [TODO]: we currently convert signature tokens into the runtime type representation, but
                // this will change to use more efficient cached type representations.
                let local_types = convert_sig_tokens(&module, &all_sig_toks, struct_name_table);

                // Pass: Bytecode -> Intra-Block SSA
                let converter = SsaConverter::new(local_types, struct_name_table);
                let mut ssa = converter.convert_function(&module, &code.code)?;

                // Pass: Pre-allocation instruction fusion
                // [TODO]: right now, we have each different fusion operation to be a separate pass, each
                // computing basic block boundaries again in a linear pass. This is easier to reason about
                // but inefficient, and will be changed in to be more efficient in the future once we
                // land on a reasonably minimal set of fusion patterns to implement.
                ssa.fuse_field_access_instrs();
                ssa.fuse_immediate_binops();

                // Pass: Greedy Slot Allocation
                let alloc = crate::slot_alloc::allocate_slots(&ssa)?;

                Ok(FunctionIR {
                    name_idx,
                    handle_idx,
                    num_params,
                    num_locals,
                    num_home_slots: alloc.num_home_slots,
                    num_xfer_slots: alloc.num_xfer_slots,
                    instrs: alloc.instrs,
                    home_slot_types: alloc.home_slot_types,
                })
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(ModuleIR { module, functions })
}
