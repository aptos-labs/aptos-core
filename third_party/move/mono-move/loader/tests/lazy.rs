// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Integration tests for the Lazy loading policy.

use mono_move_core::{native::NoNatives, types::EMPTY_TYPE_LIST, ExecutionErrorKind, GasMeter};
use mono_move_global_context::GlobalContext;
use mono_move_loader::{Loader, LoadingPolicy, LoweringPolicy, ModuleReadSet};
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
    let guard = ctx.try_execution_context(0).unwrap();
    let loader = Loader::new_with_policy(
        &guard,
        &module_provider,
        LoadingPolicy::Lazy(LoweringPolicy::Lazy),
        &NoNatives,
    );

    let id_module = ModuleId::new(AccountAddress::ONE, ident_str!("test").to_owned());
    let id = guard.intern_module_id(&id_module);

    // First call is a cache miss: fetches, deserializes, builds, installs.
    let mut read_set = ModuleReadSet::new();
    let mut gas = GasMeter::with_max_budget();
    let gas_before = gas.balance();
    let exec = loader.load_module(&mut read_set, &mut gas, id).unwrap();
    let first_cost = exec.cost();
    assert!(first_cost > 0, "cost should reflect bytecode size");
    assert_eq!(gas_before - gas.balance(), first_cost);
    assert_eq!(read_set.len(), 1);
    // Lazy policy: no dependency slots (self is handled separately).
    assert!(exec.mandatory_dependencies().is_empty());

    // Second call on a fresh read-set is a cache hit: charges the same
    // cost, records without fetching.
    let mut read_set2 = ModuleReadSet::new();
    let mut gas2 = GasMeter::with_max_budget();
    let gas_before2 = gas2.balance();
    let exec2 = loader.load_module(&mut read_set2, &mut gas2, id).unwrap();
    assert_eq!(exec2.cost(), first_cost);
    assert_eq!(gas_before2 - gas2.balance(), first_cost);
    assert_eq!(read_set2.len(), 1);
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
    let at_u64 = guard.type_list_of(&[U64_TY]);
    let at_bool = guard.type_list_of(&[BOOL_TY]);

    let charge_for = |ty_args| {
        let mut read_set = ModuleReadSet::new();
        let mut gas = GasMeter::with_max_budget();
        let before = gas.balance();
        loader
            .load_function(&mut read_set, &mut gas, module_id, name, ty_args)
            .expect("instantiation must lower");
        before - gas.balance()
    };

    // Cold (instantiation-cache miss, lowering runs) vs warm (hit):
    // identical charges, or replay would diverge across validators with
    // different cache states.
    let cold = charge_for(at_u64);
    let warm = charge_for(at_u64);
    assert_eq!(cold, warm, "cache state must not change gas charged");
    let other_cold = charge_for(at_bool);
    let other_warm = charge_for(at_bool);
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

    // Returns what was charged and what the sum of the two module costs is.
    let charge_for = || {
        let mut read_set = ModuleReadSet::new();
        let mut gas = GasMeter::with_max_budget();
        let before = gas.balance();
        loader
            .load_function(&mut read_set, &mut gas, id_a, name_f, EMPTY_TYPE_LIST)
            .expect("load_function(a::f) must lower");
        let cost_a = read_set
            .get(key_a)
            .expect("a must be in read-set")
            .cost_for_test();
        let cost_b = read_set
            .get(key_b)
            .expect("b must be in read-set")
            .cost_for_test();
        assert_eq!(read_set.len(), 2, "expected {{a, b}} in read-set");
        (before - gas.balance(), cost_a + cost_b)
    };

    let (cold, cold_expected) = charge_for();
    assert_eq!(cold, cold_expected, "LL must charge cost(a) + cost(b)");

    let (warm, warm_expected) = charge_for();
    assert_eq!(warm, warm_expected, "LL must charge cost(a) + cost(b)");
    assert_eq!(cold, warm, "cache state must not change gas charged");
}

// A module whose load failed stays in the read-set as a pending entry. A later
// lowering walk that reaches it through a type must report the same linking
// error, not an invariant violation.
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

    let mut read_set = ModuleReadSet::new();
    let mut gas = GasMeter::with_max_budget();

    let Err(err) = loader.load_module(&mut read_set, &mut gas, key_b) else {
        panic!("b is not in storage, so loading it must fail");
    };
    assert_eq!(err.kind(), ExecutionErrorKind::LinkingError);

    // Lowering `a::f` walks `b::S` and reaches the pending entry left behind.
    let Err(err) = loader.load_function(&mut read_set, &mut gas, id_a, name_f, EMPTY_TYPE_LIST)
    else {
        panic!("b is still not in storage, so lowering `a::f` must fail");
    };
    assert_eq!(
        err.kind(),
        ExecutionErrorKind::LinkingError,
        "a failed load must not turn into an invariant violation"
    );
}
