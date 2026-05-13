// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! End-to-end test for cross-module dispatch through [`TransactionContext`].
//!
//! Compiles two Move modules from source, wires an [`InMemoryModuleProvider`]
//! to a [`Loader`], and runs the entry function in the interpreter. The
//! callee module is **not** preloaded — the test verifies that hitting
//! `CallIndirect` at runtime lazily loads it through the transaction
//! context.

use mono_move_core::ExecutionContext;
use mono_move_gas::SimpleGasMeter;
use mono_move_global_context::GlobalContext;
use mono_move_loader::{Loader, LoadingPolicy, LoweringPolicy, TransactionContext};
use mono_move_runtime::InterpreterContext;
use mono_move_testsuite::InMemoryModuleProvider;
use move_core_types::{account_address::AccountAddress, ident_str};

const SOURCE: &str = r#"
module 0x1::foo {
    public fun add_one(x: u64): u64 { x + 1 }
}
module 0x1::bar {
    public fun main(x: u64): u64 { 0x1::foo::add_one(x) }
}
"#;

#[test]
fn call_indirect_triggers_lazy_module_load() {
    // -- Compile sources and stage them in an in-memory provider ---------
    let modules = mono_move_testsuite::compile_move_source(SOURCE).expect("compilation failed");
    let mut module_provider = InMemoryModuleProvider::new();
    module_provider.add_modules(&modules);

    // -- Build the global context and lazy loader ------------------------
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();
    let loader = Loader::new_with_policy(
        &guard,
        &module_provider,
        LoadingPolicy::Lazy(LoweringPolicy::Lazy),
    );

    // -- Wrap into a TransactionContext ---------------------------
    let mut txn_ctx = TransactionContext::new(&guard, loader, SimpleGasMeter::new(u64::MAX));

    // -- Resolve bar::main through the txn_ctx ---------------------------
    // This lazily loads `bar` (via the loader) and returns a pointer to
    // `main`. `foo` is *not* loaded yet — its CallIndirect site inside
    // `bar::main` will trigger the lazy load when the interpreter executes.
    let bar_id = guard
        .intern_address_name(&AccountAddress::ONE, ident_str!("bar"))
        .into_global_arena_ptr();
    let main_name = guard
        .intern_identifier(ident_str!("main"))
        .into_global_arena_ptr();
    let main_ptr = txn_ctx
        .load_function(bar_id, main_name)
        .expect("bar::main should resolve");
    assert_eq!(txn_ctx.read_set().len(), 1, "only bar loaded so far");

    // -- Run the interpreter on bar::main --------------------------------
    // SAFETY: `main_ptr` came from the executable cache, which is kept
    // alive by `guard` for the duration of this test.
    let main_fn = unsafe { main_ptr.as_ref_unchecked() };
    let mut interp = InterpreterContext::new(&mut txn_ctx, &[], main_fn);
    interp.set_root_arg(0, &41u64.to_le_bytes());
    interp.run().expect("execution should succeed");

    assert_eq!(interp.root_result(), 42, "expected foo::add_one(41) = 42");

    // -- Both modules should now be in the read-set ----------------------
    drop(interp);
    assert_eq!(
        txn_ctx.read_set().len(),
        2,
        "foo should have been lazily loaded during execution"
    );
}
