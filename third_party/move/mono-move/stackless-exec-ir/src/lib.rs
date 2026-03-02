// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod convert_v1;
pub mod convert_v2;
pub mod display;
pub mod ir;
pub mod optimize_v1;
pub mod optimize_v2;
pub mod type_conversion;

use anyhow::{bail, Result};
use ir::ModuleIR;
use move_binary_format::CompiledModule;
use move_vm_types::loaded_data::struct_name_indexing::StructNameIndex;

/// Pipeline version selector.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PipelineVersion {
    V1,
    V2,
}

/// Configuration for the conversion + optimization pipeline.
pub struct PipelineConfig {
    /// Whether to run the bytecode verifier before conversion.
    /// Set to `false` when the module comes from a trusted source (e.g., the
    /// Move compiler, which verifies internally).
    pub verify_bytecode: bool,
    /// Which pipeline version to use.
    pub version: PipelineVersion,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            verify_bytecode: true,
            version: PipelineVersion::V1,
        }
    }
}

/// Run the full conversion + optimization pipeline on a compiled module.
pub fn run_pipeline(
    module: CompiledModule,
    config: &PipelineConfig,
    struct_name_table: &[StructNameIndex],
) -> Result<ModuleIR> {
    if config.verify_bytecode {
        if let Err(e) = move_bytecode_verifier::verify_module(&module) {
            bail!("bytecode verification failed: {:#}", e);
        }
    }

    let mut module_ir = match config.version {
        PipelineVersion::V1 => convert_v1::convert_module_v1(module, struct_name_table),
        PipelineVersion::V2 => convert_v2::convert_module_v2(module, struct_name_table),
    };

    match config.version {
        PipelineVersion::V1 => optimize_v1::optimize_module_v1(&mut module_ir),
        PipelineVersion::V2 => optimize_v2::optimize_module_v2(&mut module_ir),
    }

    Ok(module_ir)
}
