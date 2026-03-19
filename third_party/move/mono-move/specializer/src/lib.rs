// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub mod analysis;
pub mod convert;
pub mod display;
pub mod instr_utils;
pub mod ir;
pub mod lower;
pub mod lowering_context;
pub mod micro_ops_display;
pub mod optimize;
pub mod pipeline;
pub mod regalloc;
pub mod type_conversion;

use anyhow::{bail, Result};
use ir::ModuleIR;
use move_binary_format::CompiledModule;
use move_vm_types::loaded_data::struct_name_indexing::StructNameIndex;

/// Configuration for the conversion + optimization pipeline.
pub struct PipelineConfig {
    /// Whether to run the bytecode verifier before conversion.
    /// Set to `false` when the module comes from a trusted source (e.g., the
    /// Move compiler, which verifies internally).
    pub verify_bytecode: bool,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            verify_bytecode: true,
        }
    }
}

/// Run the full conversion + optimization pipeline on a compiled module.
///
/// `struct_name_table` maps `StructHandleIndex` ordinals to globally unique
/// `StructNameIndex` values, used to convert bytecode-level struct references
/// into runtime `Type` representations.
pub fn run_pipeline(
    module: CompiledModule,
    config: &PipelineConfig,
    struct_name_table: &[StructNameIndex],
) -> Result<ModuleIR> {
    if config.verify_bytecode
        && let Err(e) = move_bytecode_verifier::verify_module(&module)
    {
        bail!("bytecode verification failed: {:#}", e);
    }

    pipeline::run_pipeline(module, struct_name_table)
}
