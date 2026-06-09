// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! End-to-end snapshot tests for the specializer pipeline.
//!
//! Each `.move` input runs: compile → stackless IR → micro-op lowering.
//! Output is diffed against a `.exp` snapshot; `UPBL=1` refreshes snapshots
//! in place.
//!
//! These remaining snapshots cover functions the specializer cannot yet fully
//! lower (e.g. generic field access). As features land, such tests migrate to
//! the differential suite, which also compares execution against the legacy VM.

use mono_move_global_context::GlobalContext;
use mono_move_testsuite::{compile_move_path, print_sections::render_micro_ops};
use specializer::destack;
use std::path::Path;

const EXP_EXT: &str = "exp";

datatest_stable::harness!(move_runner, "tests/test_cases/snapshot/move", r".*\.move$",);

fn move_runner(path: &Path) -> datatest_stable::Result<()> {
    let modules = compile_move_path(path).map_err(|e| format!("{:#}", e))?;

    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();

    let mut output = String::new();
    for module in modules {
        let masm_output = move_asm::disassembler::disassemble_module(String::new(), &module)
            .map_err(|e| format!("disassembly failed: {:#}", e))?;
        let module_ir = destack(module, &guard).map_err(|e| format!("{:#}", e))?;
        output.push_str("=== masm ===\n");
        output.push_str(&masm_output);
        output.push_str("\n=== specializer ===\n");
        output.push_str(&module_ir.to_string());
        output.push_str("\n=== micro-ops ===\n");
        output.push_str(&render_micro_ops(&guard, &module_ir));
    }
    let baseline_path = path.with_extension(EXP_EXT);
    move_prover_test_utils::baseline_test::verify_or_update_baseline(
        baseline_path.as_path(),
        &output,
    )?;
    Ok(())
}
