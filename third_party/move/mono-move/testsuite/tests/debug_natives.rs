// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Debug natives in cases the differential tests cannot cover: with debug mode
//! off, and in a script.

use mono_move_core::{types::EMPTY_TYPE_LIST, GasMeter, NoResourceProvider};
use mono_move_global_context::GlobalContext;
use mono_move_loader::{Loader, LoadingPolicy, LoweringPolicy};
use mono_move_runtime::{InterpreterContext, InterpreterOptions, RuntimeStatus};
use mono_move_testsuite::{
    build_natives, compile_move_script, compile_move_source, compile_move_stdlib,
    InMemoryModuleProvider,
};
use move_core_types::{account_address::AccountAddress, ident_str, identifier::IdentStr};

const SOURCE: &str = r#"
module 0x1::debug {
    use std::string::String;

    native fun native_print(x: String);
    native fun native_stack_trace(): String;

    public fun print(x: String) {
        native_print(x)
    }

    public fun stack_trace(): String {
        native_stack_trace()
    }
}

module 0x1::main {
    use std::string;
    use 0x1::debug;

    public fun print_then_trace_length(): u64 {
        debug::print(string::utf8(b"hello"));
        string::length(&debug::stack_trace())
    }
}
"#;

/// A test entry point: a function in `0x1::main` or a script.
enum Entry<'a> {
    Main(&'a IdentStr),
    Script(&'a [u8]),
}

/// Runs `entry` with no arguments and returns its status, plus the u64 result
/// on success.
fn run(entry: Entry<'_>, enable_debugging: bool) -> (RuntimeStatus, Option<u64>) {
    let mut module_provider = InMemoryModuleProvider::new();
    module_provider.add_modules(&compile_move_stdlib().expect("the stdlib compiles"));
    module_provider.add_modules(&compile_move_source(SOURCE).expect("the source compiles"));

    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx
        .try_execution_context(0)
        .expect("worker 0 execution context must be available");
    let natives = build_natives();
    let loader = Loader::new_with_policy(
        &guard,
        &module_provider,
        LoadingPolicy::Lazy(LoweringPolicy::Lazy),
        natives,
    );
    let options = InterpreterOptions {
        enable_debugging,
        ..InterpreterOptions::default()
    };
    let mut interp = InterpreterContext::new_with_options(
        loader,
        GasMeter::with_max_budget(),
        &NoResourceProvider,
        natives,
        options,
    );
    let returns_u64 = matches!(entry, Entry::Main(_));
    let entry_function = match entry {
        Entry::Main(name) => interp.load_function(
            guard
                .intern_address_name(&AccountAddress::ONE, ident_str!("main"))
                .into_global_arena_ptr(),
            guard.intern_identifier(name).into_global_arena_ptr(),
            EMPTY_TYPE_LIST,
        ),
        Entry::Script(bytes) => interp.load_script(bytes, EMPTY_TYPE_LIST),
    }
    .expect("the entry loads");

    let call = interp
        .build_call(entry_function)
        .expect("the root frame fits on the stack")
        .run()
        .expect("the call runs");
    let status = call.status().clone();
    let result = (returns_u64 && matches!(status, RuntimeStatus::Success))
        .then(|| call.interpreter().root_result_u64_for_test());
    (status, result)
}

/// Outside debug mode, printing succeeds and the stack trace is empty.
#[test]
fn debug_natives_are_no_ops_outside_debug_mode() {
    let (status, result) = run(Entry::Main(ident_str!("print_then_trace_length")), false);
    assert!(matches!(status, RuntimeStatus::Success));
    assert_eq!(result, Some(0));
}

/// A script caller is named `script::main`, matching V1.
#[test]
fn a_stack_trace_names_a_script_caller() {
    let script = compile_move_script(&format!(
        r#"{SOURCE}
script {{
    use std::string;

    fun main() {{
        let trace = 0x1::debug::stack_trace();
        let name = string::utf8(b"script::main");
        assert!(string::index_of(&trace, &name) < string::length(&trace), 1);
    }}
}}"#
    ))
    .expect("the script compiles");
    let mut bytes = Vec::new();
    script.serialize(&mut bytes).expect("the script serializes");
    let (status, _) = run(Entry::Script(&bytes), true);
    assert!(
        matches!(status, RuntimeStatus::Success),
        "the trace names the script: {status:?}"
    );
}
