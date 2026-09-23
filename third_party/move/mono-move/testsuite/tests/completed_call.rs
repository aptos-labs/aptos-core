// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! A call's results are readable only when it succeeded: after an abort the
//! return slots hold no values, so [`CompletedCall`] refuses to serialize
//! them instead of reading whatever bytes the slots contain.
//!
//! [`CompletedCall`]: mono_move_runtime::CompletedCall

use mono_move_core::{types::EMPTY_TYPE_LIST, GasMeter, NoResourceProvider};
use mono_move_global_context::GlobalContext;
use mono_move_loader::{Loader, LoadingPolicy, LoweringPolicy};
use mono_move_runtime::{InterpreterContext, ProductionNativeRegistry, RuntimeStatus};
use mono_move_testsuite::InMemoryModuleProvider;
use move_core_types::{account_address::AccountAddress, ident_str};

/// Builds a pointer-bearing result, or aborts before writing the return slot,
/// which then still holds the `bool` argument's byte and zeroed padding rather
/// than a vector.
const SOURCE: &str = r#"
module 0x1::m {
    public fun build_or_abort(fail: bool): vector<u64> {
        if (fail) abort 7;
        vector[1, 2, 3]
    }
}
"#;

#[test]
fn an_aborted_call_refuses_to_serialize_results() {
    let modules = mono_move_testsuite::compile_move_source(SOURCE).expect("compilation failed");
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
    let module_id = guard
        .intern_address_name(&AccountAddress::ONE, ident_str!("m"))
        .into_global_arena_ptr();
    let function_name = guard
        .intern_identifier(ident_str!("build_or_abort"))
        .into_global_arena_ptr();
    let mut interp = InterpreterContext::new(
        loader,
        GasMeter::with_max_budget(),
        &NoResourceProvider,
        &natives,
    );
    let function = interp
        .load_function(module_id, function_name, EMPTY_TYPE_LIST)
        .expect("the function loads");

    let mut call = interp
        .build_call(function)
        .expect("the root frame fits on the stack");
    call.arg_bcs(&bcs::to_bytes(&true).unwrap())
        .expect("the flag places");
    let call = call.run().expect("an abort is a status, not an error");

    assert!(matches!(call.status(), RuntimeStatus::Aborted {
        code: 7,
        ..
    }));
    assert!(call.serialize_return_values().is_err());
}
