// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Integration tests for gas metering through the full pipeline.

use mono_move_core::{ExecutionContext, LocalExecutionContext};
use mono_move_gas::SimpleGasMeter;
use mono_move_global_context::{ExecutionGuard, GlobalContext};
use mono_move_loader::{Loader, LoadingPolicy, LoweringPolicy, TransactionContext};
use mono_move_runtime::{ExecutionError, InterpreterContext};
use mono_move_testsuite::InMemoryModuleProvider;
use move_core_types::{account_address::AccountAddress, ident_str};

/// Compiles a Move module and adds it to the cache.
fn add_executable(guard: &ExecutionGuard<'_>, source: &str) {
    let modules = mono_move_testsuite::compile_move_source(source).expect("compilation failed");
    for module in modules {
        let loaded = mono_move_orchestrator::build_executable(guard, module)
            .expect("Building a loaded module should always succeed");
        guard
            .insert_loaded_module(loaded)
            .expect("insert should succeed");
    }
}

#[test]
fn test_out_of_gas() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();

    add_executable(
        &guard,
        r#"
module 0x1::test {
    fun fib(n: u64): u64 {
        if (n <= 1) { n } else { fib(n - 1) + fib(n - 2) }
    }
}
"#,
    );

    let id = guard.intern_address_name(&AccountAddress::ONE, ident_str!("test"));
    let fib_name = guard.intern_identifier(ident_str!("fib"));
    let fib = guard
        .get_loaded_module(id)
        .and_then(|loaded| {
            loaded
                .executable()
                .get_function(fib_name.into_global_arena_ptr())
        })
        .expect("fib should exist");

    // SAFETY: `fib` is held alive by the executable cache via `guard`.
    let fib = unsafe { fib.as_ref_unchecked() };

    let mut exec_ctx = LocalExecutionContext::with_budget(10);
    let mut interpreter = InterpreterContext::new(&mut exec_ctx, &[], fib);
    interpreter.set_root_arg(0, &10u64.to_le_bytes());
    let err = interpreter.run().unwrap_err();
    assert!(matches!(err, ExecutionError::GasExhausted(_)));
}

/// `load_function` errors when the gas budget is too small to cover the
/// loader's load cost.
#[test]
fn test_out_of_gas_during_load() {
    let modules = mono_move_testsuite::compile_move_source(
        r#"module 0x1::test { public fun f(): u64 { 0 } }"#,
    )
    .expect("compilation failed");
    let mut module_provider = InMemoryModuleProvider::new();
    module_provider.add_modules(&modules);

    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();
    let loader = Loader::new_with_policy(
        &guard,
        &module_provider,
        LoadingPolicy::Lazy(LoweringPolicy::Lazy),
    );
    // 1 gas unit — far below the byte-length cost of any real module.
    let mut txn_ctx = TransactionContext::new(&guard, loader, SimpleGasMeter::new(1));

    let id = guard
        .intern_address_name(&AccountAddress::ONE, ident_str!("test"))
        .into_global_arena_ptr();
    let f_name = guard
        .intern_identifier(ident_str!("f"))
        .into_global_arena_ptr();

    let err = txn_ctx
        .load_function(id, f_name)
        .expect_err("loading should run out of gas");
    assert!(
        err.to_string().contains("out of gas"),
        "unexpected error: {err}"
    );
}
