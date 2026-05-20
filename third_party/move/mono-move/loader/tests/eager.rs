// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Integration tests for the Lazy loading + Eager lowering policy (EL).
//!
//! EL preloads MS(M), the union of every function in M's struct-layout
//! closure (excluding M itself). Lowering itself stays per-call in
//! `load_function`.

use mono_move_gas::{GasMeter, SimpleGasMeter};
use mono_move_global_context::GlobalContext;
use mono_move_loader::{Loader, LoadingPolicy, LoweringPolicy, ModuleReadSet};
use mono_move_testsuite::InMemoryModuleProvider;
use move_core_types::{account_address::AccountAddress, ident_str, language_storage::ModuleId};

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
    );

    let id_a = guard.intern_module_id(&ModuleId::new(
        AccountAddress::ONE,
        ident_str!("a").to_owned(),
    ));
    let id_b = guard.intern_module_id(&ModuleId::new(
        AccountAddress::ONE,
        ident_str!("b").to_owned(),
    ));
    let id_c = guard.intern_module_id(&ModuleId::new(
        AccountAddress::ONE,
        ident_str!("c").to_owned(),
    ));
    let id_d = guard.intern_module_id(&ModuleId::new(
        AccountAddress::ONE,
        ident_str!("d").to_owned(),
    ));

    let mut read_set = ModuleReadSet::new();
    let mut gas = SimpleGasMeter::new(u64::MAX);
    let before = gas.balance();
    let exec = loader.load_module(&mut read_set, &mut gas, id_a).unwrap();
    let charged = before - gas.balance();

    // a + b + c are in the read-set; d (unreached) is not.
    assert_eq!(read_set.len(), 3, "expected {{a, b, c}} in read-set");
    assert!(read_set.get(id_a).is_some(), "a must be in read-set");
    assert!(read_set.get(id_b).is_some(), "b must be in read-set");
    assert!(read_set.get(id_c).is_some(), "c must be in read-set");
    assert!(
        read_set.get(id_d).is_none(),
        "d must NOT be in read-set (unreached by a's functions)"
    );

    // a's stored MS holds {a, b, c}: filled MS entries include self
    // (ModuleMandatoryDependencies invariant 4, DESIGN.md §3).
    assert_eq!(
        exec.mandatory_dependencies().slots().len(),
        3,
        "expected MS(a) to be {{a, b, c}}"
    );

    // Gas charged equals cost(a) + cost(b) + cost(c).
    let cost_a = exec.cost();
    let cost_b = read_set.get(id_b).unwrap().cost_for_test();
    let cost_c = read_set.get(id_c).unwrap().cost_for_test();
    assert_eq!(
        charged,
        cost_a + cost_b + cost_c,
        "EL must charge bodies of a, b, c exactly once"
    );
}

// Module whose only function uses primitives. The lowering walker visits
// no struct fields, so without seeding self the MS would be empty and
// mark_ready_for_lowering would bail.
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
    );

    let id_p = guard.intern_module_id(&ModuleId::new(
        AccountAddress::ONE,
        ident_str!("p").to_owned(),
    ));

    let mut read_set = ModuleReadSet::new();
    let mut gas = SimpleGasMeter::new(u64::MAX);
    let exec = loader.load_module(&mut read_set, &mut gas, id_p).unwrap();

    assert_eq!(read_set.len(), 1);
    assert_eq!(
        exec.mandatory_dependencies().slots().len(),
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
    let guard = ctx.try_execution_context(0).unwrap();
    let loader = Loader::new_with_policy(
        &guard,
        &module_provider,
        LoadingPolicy::Lazy(LoweringPolicy::Eager),
    );

    let id_a = guard.intern_module_id(&ModuleId::new(
        AccountAddress::ONE,
        ident_str!("a").to_owned(),
    ));

    // Prime the cache.
    let mut rs1 = ModuleReadSet::new();
    let mut g1 = SimpleGasMeter::new(u64::MAX);
    let before1 = g1.balance();
    loader.load_module(&mut rs1, &mut g1, id_a).unwrap();
    let cost_first = before1 - g1.balance();

    // Cache hit on a fresh read-set must recreate the same shape:
    // same total charged, same number of read-set entries.
    let mut rs2 = ModuleReadSet::new();
    let mut g2 = SimpleGasMeter::new(u64::MAX);
    let before2 = g2.balance();
    loader.load_module(&mut rs2, &mut g2, id_a).unwrap();
    let cost_second = before2 - g2.balance();

    assert_eq!(cost_first, cost_second);
    assert_eq!(rs1.len(), rs2.len());
    assert_eq!(rs2.len(), 3);
}
