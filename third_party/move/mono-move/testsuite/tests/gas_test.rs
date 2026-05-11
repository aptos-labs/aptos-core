// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Integration tests for gas metering through the full pipeline.

use mono_move_core::{types::EMPTY_TYPE_LIST, ExecutionContext, LocalExecutionContext};
use mono_move_gas::SimpleGasMeter;
use mono_move_global_context::GlobalContext;
use mono_move_loader::{Loader, LoadingPolicy, LoweringPolicy, ModuleReadSet, TransactionContext};
use mono_move_runtime::{ExecutionError, InterpreterContext};
use mono_move_testsuite::InMemoryModuleProvider;
use move_core_types::{account_address::AccountAddress, ident_str};

#[test]
fn test_out_of_gas() {
    let modules = mono_move_testsuite::compile_move_source(
        r#"
module 0x1::test {
    fun fib(n: u64): u64 {
        if (n <= 1) { n } else { fib(n - 1) + fib(n - 2) }
    }
}
"#,
    )
    .expect("compilation failed");
    let mut provider = InMemoryModuleProvider::new();
    provider.add_modules(&modules);

    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();
    let loader =
        Loader::new_with_policy(&guard, &provider, LoadingPolicy::Lazy(LoweringPolicy::Lazy));

    let id = guard.intern_address_name(&AccountAddress::ONE, ident_str!("test"));
    let fib_name = guard
        .intern_identifier(ident_str!("fib"))
        .into_global_arena_ptr();
    let mut read_set = ModuleReadSet::new();
    let mut load_gas = SimpleGasMeter::new(u64::MAX);
    let fib = loader
        .load_function(
            &mut read_set,
            &mut load_gas,
            id.into_global_arena_ptr(),
            fib_name,
            EMPTY_TYPE_LIST,
        )
        .expect("load should succeed");

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
    let mut txn_ctx = TransactionContext::new(loader, SimpleGasMeter::new(1));

    let id = guard
        .intern_address_name(&AccountAddress::ONE, ident_str!("test"))
        .into_global_arena_ptr();
    let f_name = guard
        .intern_identifier(ident_str!("f"))
        .into_global_arena_ptr();

    let Err(err) = txn_ctx.load_function(id, f_name, EMPTY_TYPE_LIST) else {
        panic!("loading failed");
    };
    assert!(
        err.to_string().contains("out of gas"),
        "unexpected error: {err}"
    );
}
