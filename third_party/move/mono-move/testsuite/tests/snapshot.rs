// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! End-to-end snapshot tests for the specializer pipeline.
//!
//! Each `.masm` / `.move` input runs: assemble/compile → stackless IR →
//! micro-op lowering. Output is diffed against a `.exp` snapshot; `UPBL=1`
//! refreshes snapshots in place.
//!
//! Struct references render with real names via the orchestrator's
//! [`ExecutableBuilder`] rather than a placeholder table.

use mono_move_core::types::InternedType;
use mono_move_global_context::{ExecutionGuard, GlobalContext};
use mono_move_orchestrator::ExecutableBuilder;
use mono_move_testsuite::{assemble_masm_source, compile_move_path};
use move_binary_format::{access::ModuleAccess, CompiledModule};
use specializer::{
    destack,
    lower::{lower_function, try_build_context, MicroOpsFunctionDisplay},
    stackless_exec_ir::ModuleIR,
};
use std::path::Path;

fn format_micro_ops(module_ir: &ModuleIR) -> String {
    let module = &module_ir.module;
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
        match try_build_context(module_ir, func_ir) {
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

/// Resolve types via the orchestrator's builder and return the
/// struct_type_table that the specializer expects.
fn resolve_struct_types(
    guard: &ExecutionGuard<'_>,
    module: &CompiledModule,
) -> Result<Vec<Option<InternedType>>, String> {
    let mut builder = ExecutableBuilder::new(guard, module);
    builder.resolve_types().map_err(|e| format!("{:#}", e))?;
    Ok(builder.struct_type_table())
}

const EXP_EXT: &str = "exp";

datatest_stable::harness!(
    masm_runner,
    "tests/test_cases/snapshot/masm",
    r".*\.masm$",
    move_runner,
    "tests/test_cases/snapshot/move",
    r".*\.move$",
);

fn masm_runner(path: &Path) -> datatest_stable::Result<()> {
    let input = std::fs::read_to_string(path)?;
    let module = assemble_masm_source(&input).map_err(|e| format!("{:#}", e))?;

    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();
    let struct_types = resolve_struct_types(&guard, &module)?;

    let ir = destack(module, &guard, &struct_types).map_err(|e| format!("{:#}", e))?;
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

fn move_runner(path: &Path) -> datatest_stable::Result<()> {
    let modules = compile_move_path(path).map_err(|e| format!("{:#}", e))?;

    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();

    let mut output = String::new();
    for module in &modules {
        let struct_types = resolve_struct_types(&guard, module)?;
        let masm_output = move_asm::disassembler::disassemble_module(String::new(), module)
            .map_err(|e| format!("disassembly failed: {:#}", e))?;
        let module_ir =
            destack(module.clone(), &guard, &struct_types).map_err(|e| format!("{:#}", e))?;
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
