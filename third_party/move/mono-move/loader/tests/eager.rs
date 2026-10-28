// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Integration tests for the Lazy loading + Eager lowering policy (EL).
//!
//! EL preloads MS(M), the union of every function in M's struct-layout
//! closure (excluding M itself). Lowering itself stays per-call in
//! `load_function`.

use mono_move_core::{native::NoNatives, types::EMPTY_TYPE_LIST, GasMeter};
use mono_move_global_context::{ExecutionGuard, GlobalContext};
use mono_move_loader::{Loader, LoadingPolicy, LoweringPolicy};
use mono_move_testsuite::InMemoryModuleProvider;
use move_core_types::{
    account_address::AccountAddress, ident_str, identifier::IdentStr, language_storage::ModuleId,
};

// Modeled on the EL example in `loader/DESIGN.md` §3.
//
// `a::mk` takes a `B` parameter and returns an `A`. The walker visits
// home-slot and return types, so it reaches A → B (via A's `x: B`
// field) → C (via B's `x: C` field). Module `d` is defined but no
// function in `a` references it, so it must not appear in MS(a).
const TEST_SOURCE: &str = r#"
module 0x1::c {
    struct C has drop { x: u64 }
}
module 0x1::b {
    use 0x1::c::C;
    struct B has drop { x: C, y: u64 }
}
module 0x1::a {
    use 0x1::b::B;
    struct A has drop { x: B, y: u64 }
    public fun mk(_b: B): A { abort 0 }
}
module 0x1::d {
    struct D has drop { x: bool }
}
"#;

/// Whether `0x1::<name>` has been charged for under `guard`.
fn is_charged(guard: &ExecutionGuard<'_>, name: &IdentStr) -> bool {
    let id = guard
        .intern_address_name(&AccountAddress::ONE, name)
        .into_global_arena_ptr();
    guard.is_charged(guard.module_idx(id).unwrap())
}

#[test]
fn load_eager_preloads_struct_closure() {
    let modules =
        mono_move_testsuite::compile_move_source(TEST_SOURCE).expect("compilation failed");
    let mut module_provider = InMemoryModuleProvider::new();
    module_provider.add_modules(&modules);

    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();
    let loader = Loader::new_with_policy(
        &guard,
        &module_provider,
        LoadingPolicy::Lazy(LoweringPolicy::Eager),
        &NoNatives,
    );

    let id_a = guard.intern_module_id(&ModuleId::new(
        AccountAddress::ONE,
        ident_str!("a").to_owned(),
    ));

    let mut gas = GasMeter::with_max_budget();
    let before = gas.balance();
    let exec = loader.load_module(&mut gas, id_a).unwrap();
    let charged = before - gas.balance();

    // a + b + c are charged for; d (unreached) is not.
    assert_eq!(guard.charged_modules().len(), 3, "expected {{a, b, c}}");
    assert!(is_charged(&guard, ident_str!("a")), "a must be charged for");
    assert!(is_charged(&guard, ident_str!("b")), "b must be charged for");
    assert!(is_charged(&guard, ident_str!("c")), "c must be charged for");
    assert!(
        !is_charged(&guard, ident_str!("d")),
        "d must NOT be charged for (unreached by a's functions)"
    );

    // a's stored MS holds {a, b, c}: a filled MS always includes self
    // (DESIGN.md §3).
    assert_eq!(
        exec.mandatory_dependencies().len(),
        3,
        "expected MS(a) to be {{a, b, c}}"
    );

    // Gas charged equals cost(a) + cost(b) + cost(c).
    let cost_of = |name: &IdentStr| {
        let id = guard
            .intern_address_name(&AccountAddress::ONE, name)
            .into_global_arena_ptr();
        loader.loaded_module(id).unwrap().cost()
    };
    assert_eq!(
        charged,
        exec.cost() + cost_of(ident_str!("b")) + cost_of(ident_str!("c")),
        "EL must charge bodies of a, b, c exactly once"
    );
}

// Module whose only function uses primitives. The lowering walker visits
// no struct fields, so without seeding self the MS would be empty.
const PRIMITIVE_ONLY_SOURCE: &str = r#"
module 0x1::p {
    public fun f(x: u64): u64 { x + 1 }
}
"#;

#[test]
fn load_eager_primitive_only_module_includes_self() {
    let modules = mono_move_testsuite::compile_move_source(PRIMITIVE_ONLY_SOURCE)
        .expect("compilation failed");
    let mut module_provider = InMemoryModuleProvider::new();
    module_provider.add_modules(&modules);

    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();
    let loader = Loader::new_with_policy(
        &guard,
        &module_provider,
        LoadingPolicy::Lazy(LoweringPolicy::Eager),
        &NoNatives,
    );

    let id_p = guard.intern_module_id(&ModuleId::new(
        AccountAddress::ONE,
        ident_str!("p").to_owned(),
    ));

    let mut gas = GasMeter::with_max_budget();
    let exec = loader.load_module(&mut gas, id_p).unwrap();

    assert_eq!(guard.charged_modules().len(), 1);
    assert_eq!(
        exec.mandatory_dependencies().len(),
        1,
        "MS must always include self even without struct refs"
    );
}

#[test]
fn load_eager_cache_hit_reproduces_state() {
    let modules =
        mono_move_testsuite::compile_move_source(TEST_SOURCE).expect("compilation failed");
    let mut module_provider = InMemoryModuleProvider::new();
    module_provider.add_modules(&modules);

    let ctx = GlobalContext::with_num_execution_workers(1);

    // Each load runs under its own execution guard: the guard is what records
    // charged modules, so sharing one would make the second load a free hit.
    let load_once = || {
        let guard = ctx.try_execution_context(0).unwrap();
        let loader = Loader::new_with_policy(
            &guard,
            &module_provider,
            LoadingPolicy::Lazy(LoweringPolicy::Eager),
            &NoNatives,
        );
        let id_a = guard.intern_module_id(&ModuleId::new(
            AccountAddress::ONE,
            ident_str!("a").to_owned(),
        ));

        let mut gas = GasMeter::with_max_budget();
        let before = gas.balance();
        loader.load_module(&mut gas, id_a).unwrap();
        (before - gas.balance(), guard.charged_modules().len())
    };

    // Prime the cache, then hit it. The hit must recreate the same shape:
    // same total charged, same number of modules charged for.
    let (cost_first, charged_first) = load_once();
    let (cost_second, charged_second) = load_once();

    assert_eq!(cost_first, cost_second);
    assert_eq!(charged_first, charged_second);
    assert_eq!(charged_second, 3);
}

// EL has no per-transaction "already lowered" bit: every call into a module
// the transaction has charged for re-resolves MS(M) and charges for its
// members. MS is memoized on the module and the members are already charged,
// so the repeat must come out free. Anything else would make gas depend on how
// many times a transaction happens to call into the same module.
#[test]
fn load_function_on_charged_module_charges_nothing() {
    let modules =
        mono_move_testsuite::compile_move_source(TEST_SOURCE).expect("compilation failed");
    let mut module_provider = InMemoryModuleProvider::new();
    module_provider.add_modules(&modules);

    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();
    let loader = Loader::new_with_policy(
        &guard,
        &module_provider,
        LoadingPolicy::Lazy(LoweringPolicy::Eager),
        &NoNatives,
    );

    let id_a = guard
        .intern_address_name(&AccountAddress::ONE, ident_str!("a"))
        .into_global_arena_ptr();
    let name_mk = guard
        .intern_identifier(ident_str!("mk"))
        .into_global_arena_ptr();

    let mut gas = GasMeter::with_max_budget();
    let before_first = gas.balance();
    loader
        .load_function(&mut gas, id_a, name_mk, EMPTY_TYPE_LIST)
        .expect("a::mk must lower");
    assert!(before_first > gas.balance(), "the first call charges");

    let before_second = gas.balance();
    loader
        .load_function(&mut gas, id_a, name_mk, EMPTY_TYPE_LIST)
        .expect("a::mk must resolve again");
    assert_eq!(
        before_second,
        gas.balance(),
        "a second call into an already-charged module must charge nothing"
    );
}
