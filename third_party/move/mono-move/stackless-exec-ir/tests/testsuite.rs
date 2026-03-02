// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use codespan_reporting::term::termcolor::Buffer;
use legacy_move_compiler::shared::known_attributes::KnownAttribute;
use move_asm::assembler::{self, Options as AsmOptions};
use move_binary_format::CompiledModule;
use move_model::metadata::LanguageVersion;
use move_vm_types::loaded_data::struct_name_indexing::StructNameIndex;
use stackless_exec_ir::{run_pipeline, PipelineConfig, PipelineVersion};
use std::path::Path;

fn make_struct_name_table(module: &CompiledModule) -> Vec<StructNameIndex> {
    (0..module.struct_handles.len())
        .map(|i| StructNameIndex::new(i as u32))
        .collect()
}

const V1_EXT: &str = "v1.exp";
const V2_EXT: &str = "v2.exp";

datatest_stable::harness!(
    masm_runner, "tests/test_cases/masm", r".*\.masm$",
    move_runner, "tests/test_cases/move", r".*\.move$",
);

fn masm_runner(path: &Path) -> datatest_stable::Result<()> {
    let input = std::fs::read_to_string(path)?;
    let options = AsmOptions::default();
    let result = assembler::assemble(&options, &input, std::iter::empty())
        .map_err(|e| format!("{:?}", e))?;
    let module = result.left().ok_or("expected module, got script")?;
    let table = make_struct_name_table(&module);

    // V1
    let v1_config = PipelineConfig {
        version: PipelineVersion::V1,
        ..PipelineConfig::default()
    };
    let v1_ir = run_pipeline(module.clone(), &v1_config, &table).map_err(|e| format!("{:#}", e))?;
    let v1_output = format!("{}", v1_ir.display());
    let v1_path = path.with_extension(V1_EXT);
    move_prover_test_utils::baseline_test::verify_or_update_baseline(
        v1_path.as_path(),
        &v1_output,
    )?;

    // V2
    let v2_config = PipelineConfig {
        version: PipelineVersion::V2,
        ..PipelineConfig::default()
    };
    let v2_ir = run_pipeline(module, &v2_config, &table).map_err(|e| format!("{:#}", e))?;
    let v2_output = format!("{}", v2_ir.display());
    let v2_path = path.with_extension(V2_EXT);
    move_prover_test_utils::baseline_test::verify_or_update_baseline(
        v2_path.as_path(),
        &v2_output,
    )?;

    Ok(())
}

fn move_runner(path: &Path) -> datatest_stable::Result<()> {
    let options = move_compiler_v2::Options {
        sources: vec![path.display().to_string()],
        named_address_mapping: vec!["std=0x1".to_string()],
        known_attributes: KnownAttribute::get_all_attribute_names().clone(),
        language_version: Some(LanguageVersion::latest_stable()),
        ..Default::default()
    };

    let mut error_writer = Buffer::no_color();
    let result = {
        let mut emitter = options.error_emitter(&mut error_writer);
        move_compiler_v2::run_move_compiler(emitter.as_mut(), options)
    };
    let (_env, units) = result.map_err(|e| {
        format!(
            "compilation failed:\n{:#}\n{}",
            e,
            String::from_utf8_lossy(&error_writer.into_inner())
        )
    })?;

    // Collect modules
    let modules: Vec<CompiledModule> = units
        .into_iter()
        .filter_map(|unit| match unit {
            legacy_move_compiler::compiled_unit::CompiledUnitEnum::Module(m) => {
                Some(m.named_module.module)
            },
            _ => None,
        })
        .collect();

    // V1
    {
        let config = PipelineConfig {
            verify_bytecode: false,
            version: PipelineVersion::V1,
        };
        let mut output = String::new();
        for module in &modules {
            let table = make_struct_name_table(module);
            let masm_output =
                move_asm::disassembler::disassemble_module(String::new(), module, false)
                    .map_err(|e| format!("disassembly failed: {:#}", e))?;
            let module_ir =
                run_pipeline(module.clone(), &config, &table).map_err(|e| format!("{:#}", e))?;
            let ir_output = format!("{}", module_ir.display());
            output.push_str("=== masm ===\n");
            output.push_str(&masm_output);
            output.push_str("\n=== stackless-exec-ir ===\n");
            output.push_str(&ir_output);
        }
        let baseline_path = path.with_extension(V1_EXT);
        move_prover_test_utils::baseline_test::verify_or_update_baseline(
            baseline_path.as_path(),
            &output,
        )?;
    }

    // V2
    {
        let config = PipelineConfig {
            verify_bytecode: false,
            version: PipelineVersion::V2,
        };
        let mut output = String::new();
        for module in &modules {
            let table = make_struct_name_table(module);
            let masm_output =
                move_asm::disassembler::disassemble_module(String::new(), module, false)
                    .map_err(|e| format!("disassembly failed: {:#}", e))?;
            let module_ir =
                run_pipeline(module.clone(), &config, &table).map_err(|e| format!("{:#}", e))?;
            let ir_output = format!("{}", module_ir.display());
            output.push_str("=== masm ===\n");
            output.push_str(&masm_output);
            output.push_str("\n=== stackless-exec-ir ===\n");
            output.push_str(&ir_output);
        }
        let baseline_path = path.with_extension(V2_EXT);
        move_prover_test_utils::baseline_test::verify_or_update_baseline(
            baseline_path.as_path(),
            &output,
        )?;
    }

    Ok(())
}
