// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! High-level pipeline: destack → lower → gas instrument → frame layout.

use crate::{
    destack,
    lower::{lower_function, try_build_context, LoweredFunction, LoweredModule},
};
use anyhow::Result;
use mono_move_core::{types::InternedType, Interner, MicroOpGasSchedule, FRAME_METADATA_SIZE};
use mono_move_gas::GasInstrumentor;
use move_binary_format::CompiledModule;

/// Run the full specializer pipeline: destack → lower → gas instrument → frame layout.
// TODO: extend with additional passes (e.g., monomorphization, GC safe-point layout).
pub fn destack_and_lower_module(
    module: CompiledModule,
    interner: &impl Interner,
    struct_types: &[Option<InternedType>],
) -> Result<LoweredModule> {
    let module_ir = destack(module, interner, struct_types)?;

    let mut functions = Vec::with_capacity(module_ir.functions.len());
    for func_ir in &module_ir.functions {
        let Some(func_ir) = func_ir else {
            continue;
        };
        let Some(ctx) = try_build_context(&module_ir, func_ir)? else {
            continue;
        };
        let micro_ops = lower_function(func_ir, &ctx)?;
        let code = GasInstrumentor::new(MicroOpGasSchedule).run(micro_ops);

        // End offset of the last param slot, including any inter-slot
        // alignment padding.
        let args_size = ctx.home_slots[..func_ir.num_params as usize]
            .last()
            .map(|s| (s.offset + s.size) as usize)
            .unwrap_or(0);
        let args_and_locals_size = ctx.frame_data_size as usize;
        let extended_frame_size = ctx
            .call_sites
            .iter()
            .flat_map(|cs| cs.arg_write_slots.iter().chain(cs.ret_read_slots.iter()))
            .map(|s| (s.offset + s.size) as usize)
            .max()
            // Leaf function: no callee slots needed beyond metadata.
            .unwrap_or(args_and_locals_size + FRAME_METADATA_SIZE);

        functions.push(LoweredFunction {
            name_idx: func_ir.name_idx,
            handle_idx: func_ir.handle_idx,
            code,
            args_size,
            args_and_locals_size,
            extended_frame_size,
        });
    }

    Ok(LoweredModule { functions })
}
