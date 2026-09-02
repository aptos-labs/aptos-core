// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Integration tests for gas metering through the full pipeline.

use mono_move_core::{types::EMPTY_TYPE_LIST, GasExhaustedError, GasMeter};
use mono_move_global_context::GlobalContext;
use mono_move_loader::{Loader, LoadingPolicy, LoweringPolicy, ModuleReadSet};
use mono_move_runtime::{InterpreterContext, ProductionNativeRegistry};
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
    let mut interpreter = InterpreterContext::new(
        loader,
        GasMeter::new(10),
        &mono_move_core::NoResourceProvider,
        &natives,
    );
    // Load with the budget suspended; the run itself gets a tiny budget of 10.
    let fib = interpreter
        .unmetered(|interp| interp.load_function(id, fib_name, EMPTY_TYPE_LIST))
        .expect("load should succeed");
    mono_move_runtime::assert_verified(fib, &guard);
    let mut call = interpreter
        .build_call(fib)
        .expect("the root frame fits on the stack");
    call.arg(&10u64).expect("argument placement succeeds");
    let err = call.run().unwrap_err();
    assert!(err.downcast_ref::<GasExhaustedError>().is_some(),);
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
