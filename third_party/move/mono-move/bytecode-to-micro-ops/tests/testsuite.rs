// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use bytecode_to_micro_ops::{display::LoweredFunctionDisplay, lower_module};
use codespan_reporting::term::termcolor::Buffer;
use legacy_move_compiler::shared::known_attributes::KnownAttribute;
use move_binary_format::CompiledModule;
use move_model::metadata::LanguageVersion;
use std::path::Path;

const EXP_EXT: &str = "exp";

datatest_stable::harness!(move_runner, "tests/test_cases/move", r".*\.move$",);

fn path_from_crate_root(path: &str) -> String {
    let mut buf = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    buf.push(path);
    buf.to_string_lossy().to_string()
}

fn move_runner(path: &Path) -> datatest_stable::Result<()> {
    let options = move_compiler_v2::Options {
        sources: vec![path.display().to_string()],
        dependencies: vec![path_from_crate_root("../../move-stdlib/sources")],
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

    let modules: Vec<CompiledModule> = units
        .into_iter()
        .filter_map(|unit| match unit {
            legacy_move_compiler::compiled_unit::CompiledUnitEnum::Module(m) => {
                Some(m.named_module.module)
            },
            _ => None,
        })
        .collect();

    let mut output = String::new();
    for module in &modules {
        // Disassemble bytecode for reference
        let masm_output = move_asm::disassembler::disassemble_module(String::new(), module)
            .map_err(|e| format!("disassembly failed: {:#}", e))?;
        output.push_str("=== masm ===\n");
        output.push_str(&masm_output);

        // Direct lowering
        output.push_str("\n=== direct micro-ops ===\n");
        let lowered_funcs = lower_module(module).map_err(|e| format!("{:#}", e))?;
        for func in &lowered_funcs {
            output.push('\n');
            output.push_str(&format!("{}", LoweredFunctionDisplay { func }));
        }
    }

    let baseline_path = path.with_extension(EXP_EXT);
    move_prover_test_utils::baseline_test::verify_or_update_baseline(
        baseline_path.as_path(),
        &output,
    )?;

    Ok(())
}
