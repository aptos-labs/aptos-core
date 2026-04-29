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

use mono_move_global_context::GlobalContext;
use mono_move_testsuite::{
    assemble_masm_source, compile_move_path,
    print_sections::{format_micro_ops, resolve_struct_types},
};
use specializer::destack;
use std::path::Path;

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
    let struct_types = resolve_struct_types(&guard, &module).map_err(|e| format!("{:#}", e))?;

    let ir = destack(module, &guard, &struct_types).map_err(|e| format!("{:#}", e))?;
    let mut output = ir.to_string();
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
        let struct_types = resolve_struct_types(&guard, module).map_err(|e| format!("{:#}", e))?;
        let masm_output = move_asm::disassembler::disassemble_module(String::new(), module)
            .map_err(|e| format!("disassembly failed: {:#}", e))?;
        let module_ir =
            destack(module.clone(), &guard, &struct_types).map_err(|e| format!("{:#}", e))?;
        output.push_str("=== masm ===\n");
        output.push_str(&masm_output);
        output.push_str("\n=== specializer ===\n");
        output.push_str(&module_ir.to_string());
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
