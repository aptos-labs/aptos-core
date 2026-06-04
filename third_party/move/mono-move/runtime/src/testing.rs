// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Test/bench helpers for building a [`ExecutionContext`] without the full
//! loader pipeline. Shared across the runtime, programs, and testsuite crates.
//!
//! These build a real [`Loader`] over a fresh global context with an empty
//! module cache; cross-module `load_function` therefore returns a clean miss.
//! Object descriptors are installed directly into the guard.

use crate::{types::DEFAULT_HEAP_SIZE, ExecutionContext};
use mono_move_core::{
    native::ProductionNativeRegistry, InMemoryModuleProvider, ObjectDescriptorTable,
    ResourceProvider, NO_RESOURCE_PROVIDER,
};
use mono_move_gas::{GasMeter, SimpleGasMeter};
use mono_move_global_context::{ExecutionGuard, GlobalContext};
use mono_move_loader::{Loader, LoadingPolicy, LoweringPolicy};

/// Builds an [`ExecutionContext`] over a fresh global context with an empty
/// module cache and no natives, with `descriptors` installed into the guard,
/// the given resource provider, and the given heap size.
///
/// The global context, guard, module provider, and native registry are leaked
/// so the returned context owns only `'static` borrows and can be held as a
/// plain local. Leaking is fine in short-lived test/bench processes; each call
/// gets an independent context, so concurrent tests never share state.
pub fn test_txn_ctx_with_resources_and_heap_size<'r, G: GasMeter>(
    descriptors: ObjectDescriptorTable,
    gas_meter: G,
    resource_provider: &'r dyn ResourceProvider,
    heap_size: usize,
) -> ExecutionContext<'r, 'static, G> {
    let gctx: &'static GlobalContext =
        Box::leak(Box::new(GlobalContext::with_num_execution_workers(1)));
    let guard: &'static ExecutionGuard<'static> = Box::leak(Box::new(
        gctx.try_execution_context(0).expect("execution guard"),
    ));
    guard.install_descriptors_for_test(descriptors);
    let module_provider: &'static InMemoryModuleProvider =
        Box::leak(Box::new(InMemoryModuleProvider::new()));
    let natives: &'static ProductionNativeRegistry<G> =
        Box::leak(Box::new(ProductionNativeRegistry::<G>::new()));
    let loader = Loader::new_with_policy(
        guard,
        module_provider,
        LoadingPolicy::Lazy(LoweringPolicy::Lazy),
        natives,
    );
    ExecutionContext::with_heap_size(loader, gas_meter, resource_provider, natives, heap_size)
}

/// [`test_txn_ctx_with_resources_and_heap_size`] with the default heap size.
pub fn test_txn_ctx_with_resources<'r, G: GasMeter>(
    descriptors: ObjectDescriptorTable,
    gas_meter: G,
    resource_provider: &'r dyn ResourceProvider,
) -> ExecutionContext<'r, 'static, G> {
    test_txn_ctx_with_resources_and_heap_size(
        descriptors,
        gas_meter,
        resource_provider,
        DEFAULT_HEAP_SIZE,
    )
}

/// [`test_txn_ctx_with_resources`] with no resources — the common case for
/// non-storage tests.
pub fn test_txn_ctx<G: GasMeter>(
    descriptors: ObjectDescriptorTable,
    gas_meter: G,
) -> ExecutionContext<'static, 'static, G> {
    test_txn_ctx_with_resources(descriptors, gas_meter, &NO_RESOURCE_PROVIDER)
}

/// [`test_txn_ctx`] with a `SimpleGasMeter(u64::MAX)`.
pub fn test_txn_ctx_max_budget(
    descriptors: ObjectDescriptorTable,
) -> ExecutionContext<'static, 'static, SimpleGasMeter> {
    test_txn_ctx(descriptors, SimpleGasMeter::new(u64::MAX))
}

/// [`test_txn_ctx_max_budget`] with a custom heap size, for GC-pressure tests.
pub fn test_txn_ctx_max_budget_with_heap_size(
    descriptors: ObjectDescriptorTable,
    heap_size: usize,
) -> ExecutionContext<'static, 'static, SimpleGasMeter> {
    test_txn_ctx_with_resources_and_heap_size(
        descriptors,
        SimpleGasMeter::new(u64::MAX),
        &NO_RESOURCE_PROVIDER,
        heap_size,
    )
}
