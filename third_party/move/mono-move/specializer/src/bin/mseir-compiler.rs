// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! CLI tool that reads a compiled Move bytecode file (.mv) and produces
//! a stackless execution IR (.mseir).

use anyhow::{Context, Result};
use clap::Parser;
use move_binary_format::{access::ModuleAccess, file_format::CompiledModule};
use move_vm_types::loaded_data::struct_name_indexing::StructNameIndex;
use specializer::destack;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[clap(
    name = "mseir-compiler",
    about = "Compile Move bytecode (.mv) to stackless execution IR (.mseir)"
)]
struct Args {
    /// Path to the input .mv file.
    #[clap(value_name = "FILE")]
    input: PathBuf,

    /// Print per-function statistics comparing bytecode and IR.
    #[clap(long, short)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let bytes = std::fs::read(&args.input)
        .with_context(|| format!("failed to read {}", args.input.display()))?;

    let module = CompiledModule::deserialize(&bytes)
        .map_err(|e| anyhow::anyhow!("failed to deserialize module: {:?}", e))?;

    let struct_name_table: Vec<StructNameIndex> = (0..module.struct_handles.len())
        .map(|i| StructNameIndex::new(i as u32))
        .collect();

    let module_ir = destack(module, &struct_name_table)?;

    if args.verbose {
        print_stats(&module_ir);
    }

    let output = format!("{}", module_ir);

    let out_path = args.input.with_extension("mseir");
    std::fs::write(&out_path, &output)
        .with_context(|| format!("failed to write {}", out_path.display()))?;

    println!("{}", out_path.display());
    Ok(())
}

fn print_stats(module_ir: &specializer::stackless_exec_ir::ModuleIR) {
    let module = &module_ir.module;

    let self_handle = module.module_handle_at(module.self_module_handle_idx);
    let addr = module.address_identifier_at(self_handle.address);
    let mod_name = module.identifier_at(self_handle.name);
    let mod_prefix = format!("0x{}::{}", addr.short_str_lossless(), mod_name);

    for func_ir in module_ir.functions.iter().flatten() {
        let func_name = module.identifier_at(func_ir.name_idx);

        // Find the matching FunctionDefinition to get bytecode stats.
        let fdef = module
            .function_defs
            .iter()
            .find(|fd| fd.function == func_ir.handle_idx);

        let (bc_instrs, bc_locals) = match fdef.and_then(|fd| fd.code.as_ref()) {
            Some(code) => (code.code.len(), module.signature_at(code.locals).0.len()),
            None => (0, 0),
        };

        // IR stats: count instructions.
        let ir_instrs: usize = func_ir.blocks.iter().map(|b| b.instrs.len()).sum();

        let num_temps = func_ir
            .num_home_slots
            .saturating_sub(func_ir.num_params + func_ir.num_locals);
        let total_slots = func_ir.num_home_slots + func_ir.num_xfer_slots;

        eprintln!(
            "{mod_prefix}::{func_name}  \
             bytecode: {bc_instrs} instrs, {bc_locals} locals  |  \
             IR: {ir_instrs} instrs, {total_slots} slots \
             (= {} params + {} locals + {num_temps} temps + {} xfer)",
            func_ir.num_params, func_ir.num_locals, func_ir.num_xfer_slots,
        );
    }
}
