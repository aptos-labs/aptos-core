// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use codespan_reporting::term::termcolor::Buffer;
use legacy_move_compiler::shared::known_attributes::KnownAttribute;
use move_asm::assembler::{self, Options as AsmOptions};
use move_binary_format::{access::ModuleAccess, CompiledModule};
use move_model::metadata::LanguageVersion;
use move_vm_types::loaded_data::struct_name_indexing::StructNameIndex;
use specializer::{
    destack,
    lower::{build_func_id_map, lower_function, try_build_context, MicroOpsFunctionDisplay},
    stackless_exec_ir::ModuleIR,
};
use std::path::Path;

fn make_struct_name_table(module: &CompiledModule) -> Vec<StructNameIndex> {
    (0..module.struct_handles.len())
        .map(|i| StructNameIndex::new(i as u32))
        .collect()
}

fn format_micro_ops(module_ir: &ModuleIR) -> String {
    let module = &module_ir.module;
    let func_id_map = build_func_id_map(module);
    let self_handle = module.module_handle_at(module.self_module_handle_idx);
    let addr = module.address_identifier_at(self_handle.address);
    let mod_name = module.identifier_at(self_handle.name);

    let mut out = String::new();
    out.push_str(&format!(
        "=== Module 0x{}::{} ===\n",
        addr.short_str_lossless(),
        mod_name
    ));

    for func_ir in module_ir.functions.iter().flatten() {
        let func_name = module.identifier_at(func_ir.name_idx).to_string();
        match try_build_context(module, func_ir, &func_id_map) {
            Err(e) => {
                out.push_str(&format!(
                    "\nfun {}(): skipped (context: {})\n",
                    func_name, e
                ));
            },
            Ok(None) => {
                out.push_str(&format!(
                    "\nfun {}(): skipped (not all types are concrete)\n",
                    func_name
                ));
            },
            Ok(Some(ctx)) => match lower_function(func_ir, &ctx) {
                Ok(ops) => {
                    out.push('\n');
                    out.push_str(&format!("{}", MicroOpsFunctionDisplay {
                        func_name: &func_name,
                        ctx: &ctx,
                        ops: &ops,
                    }));
                },
                Err(e) => {
                    out.push_str(&format!(
                        "\nfun {}(): skipped (lowering: {})\n",
                        func_name, e
                    ));
                },
            },
        }
    }
    out
}

const EXP_EXT: &str = "exp";

datatest_stable::harness!(
    masm_runner,
    "tests/test_cases/masm",
    r".*\.masm$",
    move_runner,
    "tests/test_cases/move",
    r".*\.move$",
);

fn masm_runner(path: &Path) -> datatest_stable::Result<()> {
    let input = std::fs::read_to_string(path)?;
    let options = AsmOptions::default();
    let result = assembler::assemble(&options, &input, std::iter::empty())
        .map_err(|e| format!("{:?}", e))?;
    let module = result.left().ok_or("expected module, got script")?;
    let table = make_struct_name_table(&module);

    let ir = destack(module, &table).map_err(|e| format!("{:#}", e))?;
    let mut output = format!("{}", ir);
    output.push_str("\n=== micro-ops ===\n");
    output.push_str(&format_micro_ops(&ir));
    let baseline_path = path.with_extension(EXP_EXT);
    move_prover_test_utils::baseline_test::verify_or_update_baseline(
        baseline_path.as_path(),
        &output,
    )?;

    Ok(())
}

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
        let table = make_struct_name_table(module);
        let masm_output = move_asm::disassembler::disassemble_module(String::new(), module)
            .map_err(|e| format!("disassembly failed: {:#}", e))?;
        let module_ir = destack(module.clone(), &table).map_err(|e| format!("{:#}", e))?;
        let ir_output = format!("{}", module_ir);
        output.push_str("=== masm ===\n");
        output.push_str(&masm_output);
        output.push_str("\n=== specializer ===\n");
        output.push_str(&ir_output);
        output.push_str("\n=== micro-ops ===\n");
        output.push_str(&format_micro_ops(&module_ir));
    }
    let baseline_path = path.with_extension(EXP_EXT);
    move_prover_test_utils::baseline_test::verify_or_update_baseline(
        baseline_path.as_path(),
        &output,
    )?;

    Ok(())
}
