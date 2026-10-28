// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Integration tests for the Lazy loading policy.

use mono_move_core::{native::NoNatives, types::EMPTY_TYPE_LIST, ExecutionErrorKind, GasMeter};
use mono_move_global_context::GlobalContext;
use mono_move_loader::{Loader, LoadingPolicy, LoweringPolicy};
use mono_move_testsuite::InMemoryModuleProvider;
use move_core_types::{account_address::AccountAddress, ident_str, language_storage::ModuleId};

const TEST_SOURCE: &str = r#"
module 0x1::test {
    fun identity(x: u64): u64 { x }
}
"#;

#[test]
fn load_lazy_cache_miss_and_hit() {
    let modules =
        mono_move_testsuite::compile_move_source(TEST_SOURCE).expect("compilation failed");
    let mut module_provider = InMemoryModuleProvider::new();
    module_provider.add_modules(&modules);

    let ctx = GlobalContext::with_num_execution_workers(1);
    let id_module = ModuleId::new(AccountAddress::ONE, ident_str!("test").to_owned());

    // Each load runs under its own execution guard: the guard is what records
    // charged modules, so sharing one would make the second load a free hit.
    // The module cache lives on the context and survives both.
    let load_once = || {
        let guard = ctx.try_execution_context(0).unwrap();
        let loader = Loader::new_with_policy(
            &guard,
            &module_provider,
            LoadingPolicy::Lazy(LoweringPolicy::Lazy),
            &NoNatives,
        );
        let id = guard.intern_module_id(&id_module);

        let mut gas = GasMeter::with_max_budget();
        let before = gas.balance();
        let module = loader.load_module(&mut gas, id).unwrap();
        // Lazy policy: no dependency slots (self is handled separately).
        assert!(module.mandatory_dependencies().is_empty());
        assert_eq!(guard.charged_modules().len(), 1);
        (before - gas.balance(), module.cost())
    };

    // First call is a cache miss: fetches, deserializes, builds, installs.
    let (charged, cost) = load_once();
    assert!(cost > 0, "cost should reflect bytecode size");
    assert_eq!(charged, cost);

    // Second call is a cache hit: charges the same cost without fetching.
    let (charged_again, cost_again) = load_once();
    assert_eq!(cost_again, cost);
    assert_eq!(charged_again, cost);
}

const GENERIC_SOURCE: &str = r#"
module 0x1::generic {
    fun identity<T: drop>(x: T): T { x }
}
"#;

// Gas must not depend on long-living cache state: an instantiation-cache
// miss (which runs the lowering pipeline) and a hit must charge the same.
#[test]
fn load_function_gas_is_cache_state_independent() {
    use mono_move_core::{
        types::{BOOL_TY, U64_TY},
        Interner,
    };

    let modules =
        mono_move_testsuite::compile_move_source(GENERIC_SOURCE).expect("compilation failed");
    let mut module_provider = InMemoryModuleProvider::new();
    module_provider.add_modules(&modules);

    let ctx = GlobalContext::with_num_execution_workers(1);

    // Each call runs under its own execution guard, so none of them sees a
    // module the previous one already charged for.
    let charge_for = |ty_args: &[_]| {
        let guard = ctx.try_execution_context(0).unwrap();
        let loader = Loader::new_with_policy(
            &guard,
            &module_provider,
            LoadingPolicy::Lazy(LoweringPolicy::Lazy),
            &NoNatives,
        );
        let module_id = guard
            .intern_module_id(&ModuleId::new(
                AccountAddress::ONE,
                ident_str!("generic").to_owned(),
            ))
            .into_global_arena_ptr();
        let name = guard
            .intern_identifier(ident_str!("identity"))
            .into_global_arena_ptr();
        let ty_args = guard.type_list_of(ty_args);

        let mut gas = GasMeter::with_max_budget();
        let before = gas.balance();
        loader
            .load_function(&mut gas, module_id, name, ty_args)
            .expect("instantiation must lower");
        before - gas.balance()
    };

    // Cold (instantiation-cache miss, lowering runs) vs warm (hit):
    // identical charges, or replay would diverge across validators with
    // different cache states.
    let cold = charge_for(&[U64_TY]);
    let warm = charge_for(&[U64_TY]);
    assert_eq!(cold, warm, "cache state must not change gas charged");
    let other_cold = charge_for(&[BOOL_TY]);
    let other_warm = charge_for(&[BOOL_TY]);
    assert_eq!(
        other_cold, other_warm,
        "cache state must not change gas charged"
    );
}

const CROSS_MODULE_SOURCE: &str = r#"
module 0x1::b {
    struct S has drop { x: u64 }
    public fun g(): u64 { 42 }
}
module 0x1::a {
    use 0x1::b::S;
    public fun f(_s: S): u64 { 1 }
}
"#;

// `a::f` takes `b::S`, so lowering `f` side-loads `b`. The charge is the sum
// of the two module costs, with no module counted twice and none dropped.
// `GasMeter::charge` does not deduct when the balance is short, so any
// regrouping of this sum is observable as a different residual balance on
// out-of-gas.
#[test]
fn load_function_charges_type_closure_exactly_once() {
    let modules =
        mono_move_testsuite::compile_move_source(CROSS_MODULE_SOURCE).expect("compilation failed");
    let mut module_provider = InMemoryModuleProvider::new();
    module_provider.add_modules(&modules);

    let ctx = GlobalContext::with_num_execution_workers(1);

    // Returns what was charged and what the sum of the two module costs is,
    // under a guard of its own so neither module starts out charged for.
    let charge_for = || {
        let guard = ctx.try_execution_context(0).unwrap();
        let loader = Loader::new_with_policy(
            &guard,
            &module_provider,
            LoadingPolicy::Lazy(LoweringPolicy::Lazy),
            &NoNatives,
        );
        let id_a = guard
            .intern_module_id(&ModuleId::new(
                AccountAddress::ONE,
                ident_str!("a").to_owned(),
            ))
            .into_global_arena_ptr();
        let id_b = guard
            .intern_module_id(&ModuleId::new(
                AccountAddress::ONE,
                ident_str!("b").to_owned(),
            ))
            .into_global_arena_ptr();
        let name_f = guard
            .intern_identifier(ident_str!("f"))
            .into_global_arena_ptr();

        let mut gas = GasMeter::with_max_budget();
        let before = gas.balance();
        loader
            .load_function(&mut gas, id_a, name_f, EMPTY_TYPE_LIST)
            .expect("load_function(a::f) must lower");
        let cost_a = loader.loaded_module(id_a).expect("a must be loaded").cost();
        let cost_b = loader.loaded_module(id_b).expect("b must be loaded").cost();
        assert_eq!(guard.charged_modules().len(), 2, "expected {{a, b}} charged");
        (before - gas.balance(), cost_a + cost_b)
    };

    let (cold, cold_expected) = charge_for();
    assert_eq!(cold, cold_expected, "LL must charge cost(a) + cost(b)");

    let (warm, warm_expected) = charge_for();
    assert_eq!(warm, warm_expected, "LL must charge cost(a) + cost(b)");
    assert_eq!(cold, warm, "cache state must not change gas charged");
}

// A module whose load failed stays charged for, with no row in the module
// table. A later lowering walk that reaches it through a type must report the
// same linking error, not an invariant violation.
#[test]
fn lowering_after_failed_dependency_load_reports_linking_error() {
    let modules =
        mono_move_testsuite::compile_move_source(CROSS_MODULE_SOURCE).expect("compilation failed");
    let mut module_provider = InMemoryModuleProvider::new();
    // `b` is withheld from storage, so every attempt to load it fails.
    for module in &modules {
        if module.self_id().name() == ident_str!("a") {
            module_provider.add_module(module);
        }
    }

    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();
    let loader = Loader::new_with_policy(
        &guard,
        &module_provider,
        LoadingPolicy::Lazy(LoweringPolicy::Lazy),
        &NoNatives,
    );

    let key_a = guard.intern_module_id(&ModuleId::new(
        AccountAddress::ONE,
        ident_str!("a").to_owned(),
    ));
    let key_b = guard.intern_module_id(&ModuleId::new(
        AccountAddress::ONE,
        ident_str!("b").to_owned(),
    ));
    let id_a = key_a.into_global_arena_ptr();
    let name_f = guard
        .intern_identifier(ident_str!("f"))
        .into_global_arena_ptr();

    let mut gas = GasMeter::with_max_budget();

    let Err(err) = loader.load_module(&mut gas, key_b) else {
        panic!("b is not in storage, so loading it must fail");
    };
    assert_eq!(err.kind(), ExecutionErrorKind::LinkingError);

    // Lowering `a::f` walks `b::S` and reaches the failed load left behind.
    let Err(err) = loader.load_function(&mut gas, id_a, name_f, EMPTY_TYPE_LIST) else {
        panic!("b is still not in storage, so lowering `a::f` must fail");
    };
    assert_eq!(
        err.kind(),
        ExecutionErrorKind::LinkingError,
        "a failed load must not turn into an invariant violation"
    );
}
