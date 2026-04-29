// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! High-level pipeline: destack → lower → gas instrument → frame layout.

use crate::{
    lower::{lower_function, try_build_context, LoweredFunction, LoweredModule},
    stackless_exec_ir::ModuleIR,
};
use anyhow::Result;
use mono_move_core::{MicroOpGasSchedule, FRAME_METADATA_SIZE};
use mono_move_gas::GasInstrumentor;

/// Lower an already-destacked [`ModuleIR`] into a [`LoweredModule`].
// TODO: extend with additional passes (e.g., monomorphization, GC safe-point layout).
pub fn lower_module(module_ir: &ModuleIR) -> Result<LoweredModule> {
    let mut functions = Vec::with_capacity(module_ir.functions.len());
    for func_ir in &module_ir.functions {
        let Some(func_ir) = func_ir else {
            continue;
        };
        let Some(ctx) = try_build_context(module_ir, func_ir)? else {
            continue;
        };
        let micro_ops = lower_function(func_ir, &ctx)?;
        let code = GasInstrumentor::new(MicroOpGasSchedule).run(micro_ops);

        let param_sizes: Vec<u32> = ctx.home_slots[..func_ir.num_params as usize]
            .iter()
            .map(|s| s.size)
            .collect();
        let param_sizes_sum = param_sizes.iter().map(|s| *s as usize).sum::<usize>();
        let param_and_local_sizes_sum = ctx.frame_data_size as usize;
        let extended_frame_size = ctx
            .call_sites
            .iter()
            .flat_map(|cs| cs.arg_write_slots.iter().chain(cs.ret_read_slots.iter()))
            .map(|s| (s.offset + s.size) as usize)
            .max()
            // Leaf function: no callee slots needed beyond metadata.
            .unwrap_or(param_and_local_sizes_sum + FRAME_METADATA_SIZE);

        functions.push(LoweredFunction {
            name_idx: func_ir.name_idx,
            handle_idx: func_ir.handle_idx,
            code,
            param_sizes,
            param_sizes_sum,
            param_and_local_sizes_sum,
            extended_frame_size,
        });
    }

    Ok(LoweredModule { functions })
}
