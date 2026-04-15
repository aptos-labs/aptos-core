// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! High-level pipeline: destack → lower → gas instrument → frame layout.

use crate::{
    destack,
    lower::{build_func_id_map, lower_function, try_build_context, LoweredFunction, LoweredModule},
};
use anyhow::Result;
use mono_move_core::{MicroOpGasSchedule, FRAME_METADATA_SIZE};
use mono_move_gas::GasInstrumentor;
use move_binary_format::CompiledModule;
use move_vm_types::loaded_data::struct_name_indexing::StructNameIndex;

/// Run the full specializer pipeline: destack → lower → gas instrument → frame layout.
// TODO: extend with additional passes (e.g., monomorphization, GC safe-point layout).
pub fn destack_and_lower_module(module: CompiledModule) -> Result<LoweredModule> {
    // Identity mapping: valid when loading a single module in isolation.
    let struct_name_table: Vec<StructNameIndex> = (0..module.struct_handles.len())
        .map(|i| StructNameIndex::new(i as u32))
        .collect();
    let module_ir = destack(module, &struct_name_table)?;
    let func_id_map = build_func_id_map(&module_ir.module);

    let mut functions = Vec::with_capacity(module_ir.functions.len());
    for func_ir in &module_ir.functions {
        let Some(func_ir) = func_ir else {
            functions.push(None);
            continue;
        };
        let lowered = match try_build_context(&module_ir.module, func_ir, &func_id_map)? {
            Some(ctx) => {
                let micro_ops = lower_function(func_ir, &ctx)?;
                let code = GasInstrumentor::new(MicroOpGasSchedule).run(micro_ops);

                let args_size = ctx.home_slots[..func_ir.num_params as usize]
                    .iter()
                    .map(|s| s.size as usize)
                    .sum::<usize>();
                let args_and_locals_size = ctx.frame_data_size as usize;
                let extended_frame_size = ctx
                    .call_sites
                    .iter()
                    .flat_map(|cs| cs.arg_write_slots.iter().chain(cs.ret_read_slots.iter()))
                    .map(|s| (s.offset + s.size) as usize)
                    .max()
                    // Leaf function: no callee slots needed beyond metadata.
                    .unwrap_or(args_and_locals_size + FRAME_METADATA_SIZE);

                Some(LoweredFunction {
                    name_idx: func_ir.name_idx,
                    code,
                    args_size,
                    args_and_locals_size,
                    extended_frame_size,
                })
            },
            None => None,
        };
        functions.push(lowered);
    }

    Ok(LoweredModule { functions })
}
