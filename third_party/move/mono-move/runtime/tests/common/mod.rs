// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Test helpers shared by the runtime integration tests.

use mono_move_core::{
    native::{NativeExtensions, NoNatives},
    Function, GasMeter, NoModuleProvider, NoResourceProvider,
};
use mono_move_global_context::GlobalContext;
use mono_move_loader::{Loader, LoadingPolicy, LoweringPolicy, ModuleReadSet};
use mono_move_runtime::{InterpreterContext, ProductionNativeRegistry};

/// Runs `f` with a fresh [`InterpreterContext`] over an empty module provider
/// and no natives, with `entry` verified and installed. A fresh
/// [`GlobalContext`] is built per call so no cached module or interned state
/// leaks between tests.
// Not every test binary that includes `common` uses the helper.
#[allow(dead_code)]
pub fn with_test_interpreter<R>(
    entry: &Function,
    gas_budget: u64,
    extensions: NativeExtensions,
    f: impl FnOnce(&mut InterpreterContext<'_>) -> R,
) -> R {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx
        .try_execution_context(0)
        .expect("worker 0 arena is free in a fresh context");
    let natives = ProductionNativeRegistry::new();
    let loader = Loader::new_with_policy(
        &guard,
        &NoModuleProvider,
        LoadingPolicy::Lazy(LoweringPolicy::Lazy),
        &NoNatives,
    );
    let mut interp = InterpreterContext::new(
        loader,
        ModuleReadSet::new(),
        GasMeter::new(gas_budget),
        &NoResourceProvider,
        &natives,
        entry,
    )
    .with_extensions(extensions);
    f(&mut interp)
}

/// Builds an interned module id for hand-built test functions.
#[macro_export]
macro_rules! program_module_id {
    ($name:literal) => {{
        static MODULE_ID: ::mono_move_core::interner::ModuleId =
            ::mono_move_core::interner::ModuleId::new(
                ::move_core_types::account_address::AccountAddress::ONE,
                ::mono_move_alloc::GlobalArenaPtr::from_static($name),
            );
        ::mono_move_alloc::GlobalArenaPtr::from_static(&MODULE_ID)
    }};
}
