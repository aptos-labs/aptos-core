// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Integration tests for gas metering through the full pipeline.

use mono_move_core::NoopTransactionContext;
use mono_move_gas::SimpleGasMeter;
use mono_move_global_context::{ExecutionGuard, GlobalContext};
use mono_move_runtime::{ExecutionError, InterpreterContext};
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

    let txn_ctx = NoopTransactionContext;
    let gas_meter = SimpleGasMeter::new(10);
    let mut interpreter = InterpreterContext::new(&txn_ctx, &[], gas_meter, fib);
    interpreter.set_root_arg(0, &10u64.to_le_bytes());
    let err = interpreter.run().unwrap_err();
    assert!(matches!(err, ExecutionError::GasExhausted(_)));
}
