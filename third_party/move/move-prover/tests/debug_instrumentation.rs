// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use codespan_reporting::term::termcolor::Buffer;
use move_compiler_v2::{run_move_compiler_for_analysis, Options};
use move_model::metadata::LanguageVersion;
use move_stackless_bytecode::{
    debug_instrumentation::DebugInstrumenter,
    function_target_pipeline::{FunctionTargetPipeline, FunctionTargetsHolder, FunctionVariant},
    stackless_bytecode::{Bytecode, Operation},
};
use std::collections::BTreeSet;

fn check_traces(source: &str, return_slots: &[&str], user_locals: &[&str]) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("trace.move");
    std::fs::write(&path, format!("module 0x42::Test {{ {source} }}")).unwrap();
    let mut errors = Buffer::no_color();
    let env = run_move_compiler_for_analysis(&mut errors, Options {
        sources: vec![path.to_string_lossy().into_owned()],
        language_version: Some(LanguageVersion::latest()),
        compile_verify_code: true,
        testing: true,
        ..Options::default()
    })
    .unwrap_or_else(|error| panic!("{error}: {}", String::from_utf8_lossy(errors.as_slice())));
    assert!(!env.has_errors());
    let module = env.get_modules().next().unwrap();
    let fun = module.get_functions().next().unwrap();
    let mut targets = FunctionTargetsHolder::default();
    targets.add_target(&fun);
    let target = targets.get_target(&fun, &FunctionVariant::Baseline);
    let original = target.get_bytecode().to_vec();
    let local_name = |idx| {
        fun.get_local_name(idx)
            .display(env.symbol_pool())
            .to_string()
    };
    let written_names: BTreeSet<_> = original
        .iter()
        .flat_map(|bc| bc.modifies(&target).0)
        .map(local_name)
        .collect();
    for name in return_slots.iter().chain(user_locals) {
        assert!(written_names.contains(*name), "fixture must write {name}");
    }

    let mut pipeline = FunctionTargetPipeline::default();
    pipeline.add_processor(DebugInstrumenter::new());
    pipeline.run(&env, &mut targets);
    let target = targets.get_target(&fun, &FunctionVariant::Baseline);
    let code = target.get_bytecode();
    let traced_names: BTreeSet<_> = code
        .iter()
        .filter_map(|bc| match bc {
            Bytecode::Call(_, _, Operation::TraceLocal(idx), _, _) => Some(local_name(*idx)),
            _ => None,
        })
        .collect();
    for name in return_slots {
        assert!(
            !traced_names.contains(*name),
            "synthetic local {name} was traced"
        );
    }
    for name in user_locals {
        assert!(
            traced_names.contains(*name),
            "user local {name} was not traced"
        );
    }
    for idx in 0..fun.get_parameter_count() {
        assert!(
            traced_names.contains(&local_name(idx)),
            "parameter was not traced"
        );
    }

    let mut returns = 0;
    for (offset, bc) in code.iter().enumerate() {
        if let Bytecode::Ret(_, values) = bc {
            returns += 1;
            for (idx, value) in values.iter().enumerate() {
                assert!(
                    matches!(
                        &code[offset - values.len() + idx],
                        Bytecode::Call(_, _, Operation::TraceReturn(slot), sources, None)
                            if *slot == idx && sources == &vec![*value]
                    ),
                    "every return value must retain its result trace"
                );
            }
        }
    }
    assert!(returns >= 2, "fixture must exercise both return branches");
    let executable: Vec<_> = code
        .iter()
        .filter(|bc| {
            !matches!(
                bc,
                Bytecode::Call(
                    _,
                    _,
                    Operation::TraceLocal(_) | Operation::TraceReturn(_),
                    _,
                    _
                )
            )
        })
        .cloned()
        .collect();
    // The shared bytecode builder removes jumps to the immediately following label.
    let original: Vec<_> = original
        .iter()
        .enumerate()
        .filter(|(offset, bc)| {
            !matches!(
                (bc, original.get(offset + 1)),
                (Bytecode::Jump(_, dest), Some(Bytecode::Label(_, next))) if dest == next
            )
        })
        .map(|(_, bc)| bc.clone())
        .collect();
    assert_eq!(
        executable, original,
        "tracing must preserve executable instructions"
    );
}

#[test]
fn single_return_slot_is_not_a_user_local() {
    check_traces(
        "fun f(b: bool): u64 { return (if (b) 4 else 5) }",
        &["return"],
        &[],
    );
}

#[test]
fn multiple_return_slots_are_not_user_locals() {
    check_traces(
        "fun f(b: bool): (u64, u64) { return (if (b) (4, 6) else (5, 7)) }",
        &["return[0]", "return[1]"],
        &[],
    );
}

#[test]
fn similarly_named_user_locals_keep_their_traces() {
    check_traces(
        "fun f(b: bool): u64 {
            let return_value = if (b) 4 else 5;
            return_value = return_value + 1;
            if (b) return_value else return_value + 1
        }",
        &[],
        &["return_value"],
    );
}
