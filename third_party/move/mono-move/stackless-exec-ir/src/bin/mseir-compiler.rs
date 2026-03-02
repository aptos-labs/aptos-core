// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! CLI tool that reads a compiled Move bytecode file (.mv) and produces
//! a stackless execution IR (.mseir) using either the v1 or v2 pipeline.

use anyhow::{Context, Result};
use clap::Parser;
use move_binary_format::{access::ModuleAccess, file_format::CompiledModule};
use move_vm_types::loaded_data::struct_name_indexing::StructNameIndex;
use stackless_exec_ir::{ir::Instr, run_pipeline, PipelineConfig, PipelineVersion};
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

    /// Pipeline version to use for conversion.
    #[clap(long, short, value_enum, default_value_t = Pipeline::V2)]
    pipeline: Pipeline,

    /// Skip bytecode verification (trust the input).
    #[clap(long)]
    no_verify: bool,

    /// Print per-function statistics comparing bytecode and IR.
    #[clap(long, short)]
    verbose: bool,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum Pipeline {
    V1,
    V2,
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

    let config = PipelineConfig {
        verify_bytecode: !args.no_verify,
        version: match args.pipeline {
            Pipeline::V1 => PipelineVersion::V1,
            Pipeline::V2 => PipelineVersion::V2,
        },
    };

    let module_ir = run_pipeline(module, &config, &struct_name_table)?;

    if args.verbose {
        print_stats(&module_ir);
    }

    let output = format!("{}", module_ir.display());

    let out_path = args.input.with_extension("mseir");
    std::fs::write(&out_path, &output)
        .with_context(|| format!("failed to write {}", out_path.display()))?;

    println!("{}", out_path.display());
    Ok(())
}

fn print_stats(module_ir: &stackless_exec_ir::ir::ModuleIR) {
    let module = &module_ir.module;

    let self_handle = module.module_handle_at(module.self_module_handle_idx);
    let addr = module.address_identifier_at(self_handle.address);
    let mod_name = module.identifier_at(self_handle.name);
    let mod_prefix = format!("0x{}::{}", addr.short_str_lossless(), mod_name);

    for func_ir in &module_ir.functions {
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

        // IR stats: count non-label instructions.
        let ir_instrs = func_ir
            .instrs
            .iter()
            .filter(|i| !matches!(i, Instr::Label(_)))
            .count();

        eprintln!(
            "{mod_prefix}::{func_name}  \
             bytecode: {bc_instrs} instrs, {bc_locals} locals  |  \
             IR: {ir_instrs} instrs, {} regs (= {} params + {} locals + {} temps)",
            func_ir.num_regs,
            func_ir.num_params,
            func_ir.num_locals,
            func_ir.num_regs.saturating_sub(func_ir.num_params + func_ir.num_locals),
        );
    }
}
