// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! End-to-end tests example.
//!
//! TODO: ideally use Move harness tests via V1 and V2 VMs.

use mono_move_global_context::GlobalContext;
use mono_move_programs::{compile_move_source, testing};
use mono_move_runtime::InterpreterContext;
use move_core_types::identifier::Identifier;

/// Compile Move source, build an executable via GlobalContext, look up a
/// function by name, and run it in the micro-op interpreter.
fn build_and_run(source: &str, fun_name: &str, args: &[u64]) -> u64 {
    let module = compile_move_source(source);

    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();
    let executable = guard
        .executable_builder_for_module(&module)
        .build()
        .unwrap();

    let ident = Identifier::new(fun_name).unwrap();
    let name = guard.intern_identifier(&ident);
    let func = executable.get_function(name).unwrap();

    let mut interp = InterpreterContext::new(&[], &[], func);
    for (i, &arg) in args.iter().enumerate() {
        interp.set_root_arg((i * 8) as u32, &arg.to_le_bytes());
    }
    interp.run().unwrap();
    interp.root_result()
}

/// Run the same function via the Move VM and return the u64 result.
fn move_vm_run(source: &str, fun_name: &str, args: &[u64]) -> u64 {
    let module = compile_move_source(source);
    let vm_args: Vec<Vec<u8>> = args.iter().map(|&a| testing::arg_u64(a)).collect();
    let result = testing::run_move_function(&module, fun_name, vm_args);
    testing::return_u64(&result)
}

// ---------------------------------------------------------------------------
// Test cases
// ---------------------------------------------------------------------------

const ADD_SOURCE: &str = r#"
module 0x1::test {
    fun add(a: u64, b: u64): u64 {
        a + b
    }
}
"#;

#[test]
fn add() {
    let micro = build_and_run(ADD_SOURCE, "add", &[3, 5]);
    let vm = move_vm_run(ADD_SOURCE, "add", &[3, 5]);
    assert_eq!(micro, 8);
    assert_eq!(micro, vm);
}

const ADD_THREE_SOURCE: &str = r#"
module 0x1::test {
    fun add_three(a: u64, b: u64, c: u64): u64 {
        a + b + c
    }
}
"#;

#[test]
fn add_three() {
    for (a, b, c, expected) in [(1, 2, 3, 6), (0, 0, 0, 0), (10, 20, 30, 60)] {
        let micro = build_and_run(ADD_THREE_SOURCE, "add_three", &[a, b, c]);
        let vm = move_vm_run(ADD_THREE_SOURCE, "add_three", &[a, b, c]);
        assert_eq!(micro, expected, "add_three({a}, {b}, {c})");
        assert_eq!(micro, vm, "add_three({a}, {b}, {c}) vm mismatch");
    }
}

const FIB_SOURCE: &str = r#"
module 0x1::test {
    fun fib(n: u64): u64 {
        if (n <= 1) {
            n
        } else {
            fib(n - 1) + fib(n - 2)
        }
    }
}
"#;

#[test]
fn fib() {
    for (n, expected) in [(0, 0), (1, 1), (5, 5), (10, 55)] {
        let micro = build_and_run(FIB_SOURCE, "fib", &[n]);
        let vm = move_vm_run(FIB_SOURCE, "fib", &[n]);
        assert_eq!(micro, expected, "fib({n})");
        assert_eq!(micro, vm, "fib({n}) vm mismatch");
    }
}

const BRANCH_SOURCE: &str = r#"
module 0x1::test {
    fun clamp(n: u64): u64 {
        if (n <= 10) {
            n
        } else {
            10
        }
    }
}
"#;

#[test]
fn branch() {
    for (n, expected) in [(0, 0), (5, 5), (10, 10), (100, 10)] {
        let micro = build_and_run(BRANCH_SOURCE, "clamp", &[n]);
        let vm = move_vm_run(BRANCH_SOURCE, "clamp", &[n]);
        assert_eq!(micro, expected, "clamp({n})");
        assert_eq!(micro, vm, "clamp({n}) vm mismatch");
    }
}

const MULTI_FUNC_SOURCE: &str = r#"
module 0x1::test {
    fun double(x: u64): u64 {
        x + x
    }

    fun quad(x: u64): u64 {
        double(double(x))
    }
}
"#;

#[test]
fn multi_func() {
    for (x, expected) in [(0, 0), (1, 4), (3, 12), (10, 40)] {
        let micro = build_and_run(MULTI_FUNC_SOURCE, "quad", &[x]);
        let vm = move_vm_run(MULTI_FUNC_SOURCE, "quad", &[x]);
        assert_eq!(micro, expected, "quad({x})");
        assert_eq!(micro, vm, "quad({x}) vm mismatch");
    }
}
