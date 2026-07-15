// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Integration tests for gas metering through the full pipeline.

use mono_move_core::{types::EMPTY_TYPE_LIST, GasMeter};
use mono_move_global_context::GlobalContext;
use mono_move_loader::{Loader, LoadingPolicy, LoweringPolicy, ModuleReadSet};
use mono_move_runtime::{InterpreterContext, ProductionNativeRegistry, RuntimeError};
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
    let natives = ProductionNativeRegistry::new();
    let loader = Loader::new_with_policy(
        &guard,
        &provider,
        LoadingPolicy::Lazy(LoweringPolicy::Lazy),
        &natives,
    );

    let id = guard
        .intern_address_name(&AccountAddress::ONE, ident_str!("test"))
        .into_global_arena_ptr();
    let fib_name = guard
        .intern_identifier(ident_str!("fib"))
        .into_global_arena_ptr();
    // Load with an effectively unbounded budget; the run itself gets a tiny
    // budget of 10.
    let mut read_set = ModuleReadSet::new();
    let mut load_gas = GasMeter::with_max_budget();
    let fib = loader
        .load_function(&mut read_set, &mut load_gas, id, fib_name, EMPTY_TYPE_LIST)
        .expect("load should succeed");

    // SAFETY: `fib` is held alive by the executable cache via `guard`.
    let fib = unsafe { fib.as_ref_unchecked() };

    let mut interpreter = InterpreterContext::new(
        loader,
        read_set,
        GasMeter::new(10),
        &mono_move_core::NoResourceProvider,
        &natives,
        fib,
    );
    interpreter.set_root_arg(0, &10u64.to_le_bytes());
    let err = interpreter.run().unwrap_err();
    assert!(matches!(err, RuntimeError::GasExhausted(_)));
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
    let natives = ProductionNativeRegistry::new();
    let loader = Loader::new_with_policy(
        &guard,
        &module_provider,
        LoadingPolicy::Lazy(LoweringPolicy::Lazy),
        &natives,
    );
    let id = guard
        .intern_address_name(&AccountAddress::ONE, ident_str!("test"))
        .into_global_arena_ptr();
    let f_name = guard
        .intern_identifier(ident_str!("f"))
        .into_global_arena_ptr();

    // 1 gas unit — far below the byte-length cost of any real module.
    let mut read_set = ModuleReadSet::new();
    let mut gas_meter = GasMeter::new(1);
    let Err(err) = loader.load_function(&mut read_set, &mut gas_meter, id, f_name, EMPTY_TYPE_LIST)
    else {
        panic!("loading failed");
    };
    assert!(
        err.to_string().contains("out of gas"),
        "unexpected error: {err}"
    );
}
