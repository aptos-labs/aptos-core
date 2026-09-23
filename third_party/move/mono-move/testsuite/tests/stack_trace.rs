// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Stack traces list callers, most recent first, at their call instructions,
//! excluding the frame where execution stopped. Errors expose the trace on the
//! interpreter; aborts expose it on the completed call. Expected definition
//! indexes and offsets come from the compiled code so compiler layout changes
//! do not invalidate them.

use mono_move_core::{
    types::EMPTY_TYPE_LIST, CallFrame, GasMeter, NoResourceProvider, VMInternalError,
};
use mono_move_global_context::GlobalContext;
use mono_move_loader::{Loader, LoadingPolicy, LoweringPolicy};
use mono_move_runtime::{InterpreterContext, ProductionNativeRegistry, RuntimeStatus};
use mono_move_testsuite::{
    compile_move_script, compile_move_source, find_module, function_def_index,
    sole_bytecode_offset, InMemoryModuleProvider,
};
use move_binary_format::{
    access::ModuleAccess,
    file_format::{Bytecode, CompiledScript, FunctionDefinitionIndex},
    CompiledModule,
};
use move_core_types::{account_address::AccountAddress, ident_str, identifier::IdentStr};

/// Calls follow `outer` -> `middle` -> `inner`. The inner functions can fail by
/// overflow in `bump` or abort in `check`.
const SOURCE: &str = r#"
module 0x1::inner {
    public fun bump(value: u64): u64 { value + 1 }
    public fun check(fail: bool) { if (fail) abort 7 }
}
module 0x1::middle {
    public fun bump(value: u64): u64 { 0x1::inner::bump(value) }
    public fun check(fail: bool) { 0x1::inner::check(fail) }
}
module 0x1::outer {
    public fun bump(value: u64): u64 { 0x1::middle::bump(value) }
    public fun check(fail: bool) { 0x1::middle::check(fail) }
}
"#;

/// A test entry point: a function in `0x1::outer` or a script.
enum Entry<'a> {
    Outer(&'a IdentStr),
    Script(&'a CompiledScript),
}

/// Runs `entry` with the BCS-encoded `arg` and returns the compiled modules,
/// execution result, and caller stack trace.
fn run(
    entry: Entry<'_>,
    arg: &[u8],
) -> (
    Vec<CompiledModule>,
    Result<RuntimeStatus, VMInternalError>,
    Vec<CallFrame>,
) {
    let modules = compile_move_source(SOURCE).expect("source compiles");
    let mut module_provider = InMemoryModuleProvider::new();
    module_provider.add_modules(&modules);

    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx
        .try_execution_context(0)
        .expect("worker 0 execution context must be available");
    let natives = ProductionNativeRegistry::new();
    let loader = Loader::new_with_policy(
        &guard,
        &module_provider,
        LoadingPolicy::Lazy(LoweringPolicy::Lazy),
        &natives,
    );
    let mut interp = InterpreterContext::new(
        loader,
        GasMeter::with_max_budget(),
        &NoResourceProvider,
        &natives,
    );
    let entry_function = match entry {
        Entry::Outer(name) => interp.load_function(
            guard
                .intern_address_name(&AccountAddress::ONE, ident_str!("outer"))
                .into_global_arena_ptr(),
            guard.intern_identifier(name).into_global_arena_ptr(),
            EMPTY_TYPE_LIST,
        ),
        Entry::Script(script) => {
            let mut bytes = Vec::new();
            script.serialize(&mut bytes).expect("the script serializes");
            interp.load_script(&bytes, EMPTY_TYPE_LIST)
        },
    }
    .expect("the entry loads");

    let mut call = interp
        .build_call(entry_function)
        .expect("the root frame fits on the stack");
    call.arg_bcs(arg).expect("the argument places");
    // Success and abort return a completed call; errors leave their trace on
    // the interpreter.
    match call.run() {
        Ok(call) => {
            let trace = call.stack_trace().expect("the frame chain is intact");
            (modules, Ok(call.status().clone()), trace)
        },
        Err(err) => {
            let trace = interp.stack_trace().expect("the frame chain is intact");
            (modules, Err(err), trace)
        },
    }
}

/// The frame of `module_name::function_name` at its only call instruction.
fn call_frame(modules: &[CompiledModule], module_name: &str, function_name: &str) -> CallFrame {
    let module = find_module(modules, module_name);
    let function =
        function_def_index(module, function_name).expect("function is defined in the module");
    let body = module
        .function_def_at(function)
        .code
        .as_ref()
        .expect("function has a body");
    CallFrame {
        module: Some(module.self_id()),
        function,
        offset: sole_bytecode_offset(&body.code, is_call),
    }
}

fn is_call(instruction: &Bytecode) -> bool {
    matches!(instruction, Bytecode::Call(_))
}

/// A fault two calls deep names the two calling frames, innermost first.
#[test]
fn a_fault_carries_its_calling_frames() {
    let (modules, result, trace) = run(
        Entry::Outer(ident_str!("bump")),
        &bcs::to_bytes(&u64::MAX).expect("the argument serializes"),
    );
    assert!(result.is_err(), "the addition overflows");
    assert_eq!(trace, vec![
        call_frame(&modules, "middle", "bump"),
        call_frame(&modules, "outer", "bump"),
    ]);
}

/// An abort two calls deep exposes both callers on the completed call,
/// innermost first.
#[test]
fn an_abort_exposes_its_calling_frames() {
    let (modules, result, trace) = run(
        Entry::Outer(ident_str!("check")),
        &bcs::to_bytes(&true).expect("the argument serializes"),
    );
    assert!(matches!(result, Ok(RuntimeStatus::Aborted { code: 7, .. })));
    assert_eq!(trace, vec![
        call_frame(&modules, "middle", "check"),
        call_frame(&modules, "outer", "check"),
    ]);
}

/// A successful call ends in the root frame, which has no callers.
#[test]
fn a_success_has_no_calling_frames() {
    let (_modules, result, trace) = run(
        Entry::Outer(ident_str!("check")),
        &bcs::to_bytes(&false).expect("the argument serializes"),
    );
    assert!(matches!(result, Ok(RuntimeStatus::Success)));
    assert!(trace.is_empty());
}

/// A script frame has no module and uses definition index 0, matching V1.
#[test]
fn a_script_frame_names_no_module() {
    let script = compile_move_script(&format!(
        "{SOURCE}\nscript {{ fun main(value: u64) {{ 0x1::middle::bump(value); }} }}"
    ))
    .expect("the script compiles");
    let (modules, result, trace) = run(
        Entry::Script(&script),
        &bcs::to_bytes(&u64::MAX).expect("the argument serializes"),
    );
    assert!(result.is_err(), "the addition overflows");
    assert_eq!(trace, vec![
        call_frame(&modules, "middle", "bump"),
        CallFrame {
            module: None,
            function: FunctionDefinitionIndex(0),
            offset: sole_bytecode_offset(&script.code.code, is_call),
        },
    ]);
}
