// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Checks that rewrites keep generated tests verifiable and their VM behavior unchanged.

mod common;

use arbitrary::Unstructured;
use common::{assert_verifies, random_bytes, run_output, valid_program};
use masm_smith::{
    ir::{Operand, Step, Value},
    rewrite::{apply, compare_outputs, Agreement, Rewrite, DEAD_BRANCH_ABORT_BASE},
    MasmTest,
};
use std::collections::BTreeMap;

const PROGRAMS: u32 = 64;

/// Programs that run in the VM, for the tests comparing outputs.
const RUN_PROGRAMS: u32 = 24;

fn rewritten(program: &MasmTest, seed: u32, rewrite: Rewrite) -> Option<MasmTest> {
    let bytes = random_bytes(seed, 1024);
    apply(program, rewrite, &mut Unstructured::new(&bytes))
        .expect("rewrite draws from enough bytes")
}

#[test]
fn rewrites_keep_tests_verifiable() {
    let mut applied: BTreeMap<&str, u32> = BTreeMap::new();
    for (index, program) in (0..PROGRAMS).filter_map(valid_program).enumerate() {
        for rewrite in Rewrite::ALL {
            let seed = index as u32 * 16 + rewrite as u32;
            if let Some(variant) = rewritten(&program, seed, rewrite) {
                assert_verifies(&variant, &format!("{} on program {index}", rewrite.name()));
                *applied.entry(rewrite.name()).or_default() += 1;
            }
        }
    }
    eprintln!("applied: {applied:?}");
    for rewrite in Rewrite::ALL {
        let count = applied.get(rewrite.name()).copied().unwrap_or(0);
        assert!(
            count >= PROGRAMS / 16,
            "{} applied to only {count} programs",
            rewrite.name()
        );
    }
}

/// Every rewrite preserves statuses, abort codes, return values and writes; only a minority
/// of comparisons are inconclusive because one side exhausted gas or stack.
#[test]
fn rewrites_preserve_vm_output() {
    let (mut same, mut inconclusive) = (0u32, 0u32);
    for (index, program) in (0..RUN_PROGRAMS).filter_map(valid_program).enumerate() {
        let original = run_output(&program, &format!("program {index}"));
        for rewrite in Rewrite::ALL {
            let seed = index as u32 * 16 + rewrite as u32 + 7;
            let Some(variant) = rewritten(&program, seed, rewrite) else {
                continue;
            };
            let label = format!("{} on program {index}", rewrite.name());
            let output = run_output(&variant, &label);
            match compare_outputs(&original, &output) {
                Agreement::Same => same += 1,
                Agreement::Inconclusive { .. } => inconclusive += 1,
                disagreement => panic!(
                    "{label}: {disagreement:?}\n--original output--\n{original}\n--variant output--\n{output}\n--variant--\n{}",
                    variant.render()
                ),
            }
        }
    }
    eprintln!("{same} conclusive and {inconclusive} inconclusive comparisons");
    assert!(same > RUN_PROGRAMS, "only {same} conclusive comparisons");
    assert!(
        inconclusive * 4 < same,
        "{inconclusive} inconclusive against {same} conclusive comparisons"
    );
}

/// The block a dead branch guards against is reachable: inverting the guard makes runs that
/// pass through it abort with the marker code. So the oracle relies on the guard holding, not
/// on the block being unreachable for other reasons.
#[test]
fn dead_branches_abort_when_inverted() {
    let mut aborted = 0u32;
    for (index, program) in (0..RUN_PROGRAMS).filter_map(valid_program).enumerate() {
        let Some(mut variant) = rewritten(&program, index as u32 + 500, Rewrite::DeadBranch) else {
            continue;
        };
        let codes = invert_dead_branch_guards(&mut variant);
        let output = run_output(
            &variant,
            &format!("inverted dead branch on program {index}"),
        );
        if codes
            .iter()
            .any(|code| output.contains(&format!("sub_status: Some({code})")))
        {
            aborted += 1;
        }
    }
    eprintln!("{aborted} programs reached an inverted guard");
    assert!(aborted > 0, "no inverted guard was reached");
}

/// Flips the branch of every dead-branch guard and returns the guarded abort codes.
fn invert_dead_branch_guards(test: &mut MasmTest) -> Vec<u64> {
    let mut codes = Vec::new();
    for step in &mut test.steps {
        let functions = match step {
            Step::Publish(module) => module.functions.iter_mut().collect::<Vec<_>>(),
            Step::RunScript(script) => vec![&mut script.entry],
            Step::RunCall(_) => Vec::new(),
        };
        for function in functions {
            let dead_blocks: BTreeMap<_, u64> = function
                .body
                .iter()
                .filter_map(|block| match block.instrs.as_slice() {
                    [load, abort] if abort.opcode == "abort" => match load.operands.as_slice() {
                        [Operand::Imm(Value::U64(code))]
                            if code & !0xFFFF == DEAD_BRANCH_ABORT_BASE =>
                        {
                            Some((block.id, *code))
                        },
                        _ => None,
                    },
                    _ => None,
                })
                .collect();
            for instr in function.body.iter_mut().flat_map(|block| &mut block.instrs) {
                if let [Operand::Label(target)] = instr.operands.as_slice() {
                    if let Some(code) = dead_blocks.get(target) {
                        instr.opcode = match instr.opcode.as_str() {
                            "br_true" => "br_false",
                            _ => "br_true",
                        }
                        .to_string();
                        codes.push(*code);
                    }
                }
            }
        }
    }
    codes
}
