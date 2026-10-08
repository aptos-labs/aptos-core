// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Integration tests for the Package loading policy.

use mono_move_core::{native::NoNatives, types::EMPTY_TYPE_LIST, GasMeter};
use mono_move_global_context::GlobalContext;
use mono_move_loader::{Loader, LoadingPolicy};
use mono_move_testsuite::InMemoryModuleProvider;
use move_core_types::{account_address::AccountAddress, ident_str, language_storage::ModuleId};

const TEST_SOURCE: &str = r#"
module 0x1::a {
    public fun a_fn(): u64 { 1 }
}
module 0x1::b {
    public fun b_fn(): u64 { 2 }
}
"#;

#[test]
fn load_package_cache_miss_loads_all_members() {
    let modules =
        mono_move_testsuite::compile_move_source(TEST_SOURCE).expect("compilation failed");
    let mut module_provider = InMemoryModuleProvider::new();
    module_provider.add_modules(&modules);
    module_provider.declare_package(AccountAddress::ONE, ident_str!("a").to_owned(), vec![
        ident_str!("b").to_owned(),
    ]);

    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();
    let loader =
        Loader::new_with_policy(&guard, &module_provider, LoadingPolicy::Package, &NoNatives);

    let id_a_module = ModuleId::new(AccountAddress::ONE, ident_str!("a").to_owned());
    let id_a = guard.intern_module_id(&id_a_module);

    let mut gas = GasMeter::with_max_budget();
    let exec = loader.load_module(&mut gas, id_a).unwrap();

    // Both package members must have been charged for.
    assert_eq!(guard.charged_modules().len(), 2);

    // mandatory_dependencies covers every package member, including
    // self. For a 2-module package, that's both slots.
    assert_eq!(exec.mandatory_dependencies().len(), 2);

    // The sibling must also be resolvable through the loader directly.
    let id_b = guard
        .intern_address_name(&AccountAddress::ONE, ident_str!("b"))
        .into_global_arena_ptr();
    assert!(loader.loaded_module(id_b).is_ok());
}

const CROSS_PACKAGE_SOURCE: &str = r#"
module 0x1::b {
    struct S has drop { x: u64 }
    public fun g(): u64 { 42 }
}
module 0x1::a {
    use 0x1::b::S;
    public fun f(_s: S): u64 { 1 }
}
"#;

#[test]
fn package_policy_reuses_side_loaded_module_on_function_call() {
    let modules =
        mono_move_testsuite::compile_move_source(CROSS_PACKAGE_SOURCE).expect("compilation failed");
    let mut module_provider = InMemoryModuleProvider::new();
    module_provider.add_modules(&modules);
    module_provider.declare_package(AccountAddress::ONE, ident_str!("a").to_owned(), vec![]);
    module_provider.declare_package(AccountAddress::ONE, ident_str!("b").to_owned(), vec![]);

    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();
    let loader =
        Loader::new_with_policy(&guard, &module_provider, LoadingPolicy::Package, &NoNatives);

    let id_a = guard
        .intern_address_name(&AccountAddress::ONE, ident_str!("a"))
        .into_global_arena_ptr();
    let id_b = guard
        .intern_address_name(&AccountAddress::ONE, ident_str!("b"))
        .into_global_arena_ptr();
    let name_f = guard
        .intern_identifier(ident_str!("f"))
        .into_global_arena_ptr();
    let name_g = guard
        .intern_identifier(ident_str!("g"))
        .into_global_arena_ptr();

    let mut gas = GasMeter::with_max_budget();

    // 1. `a::f` takes `b::S` by value, so lowering it walks `S` and side-loads
    //    `b`. `S` is a concrete inline struct, so the specializer derives its
    //    GC layout and `a::f` lowers successfully. Only `b`'s layout was
    //    needed, so its package was never pulled in.
    loader
        .load_function(&mut gas, id_a, name_f, EMPTY_TYPE_LIST)
        .expect("load_function(a::f) must lower now that inline structs are supported");
    assert_eq!(
        guard.charged_modules().len(),
        2,
        "expected `a` and the side-loaded `b` to be charged for"
    );

    // 2. `b::g` is nominal-free, so dispatching to it succeeds. The package
    //    policy must reuse the already-charged `b` rather than re-loading or
    //    bailing, which is observable as a charge of zero.
    let before = gas.balance();
    loader
        .load_function(&mut gas, id_b, name_g, EMPTY_TYPE_LIST)
        .expect("load_function(b::g) must reuse b, not bail");
    assert_eq!(
        before,
        gas.balance(),
        "a module charged for earlier in the transaction must not be charged again"
    );
}

#[test]
fn load_package_cache_hit_walks_dependencies() {
    let modules =
        mono_move_testsuite::compile_move_source(TEST_SOURCE).expect("compilation failed");
    let mut module_provider = InMemoryModuleProvider::new();
    module_provider.add_modules(&modules);
    module_provider.declare_package(AccountAddress::ONE, ident_str!("a").to_owned(), vec![
        ident_str!("b").to_owned(),
    ]);

    let ctx = GlobalContext::with_num_execution_workers(1);

    // Each load runs under its own execution guard: the guard is what records
    // charged modules, so sharing one would make the second load a free hit.
    let load_once = || {
        let guard = ctx.try_execution_context(0).unwrap();
        let loader =
            Loader::new_with_policy(&guard, &module_provider, LoadingPolicy::Package, &NoNatives);
        let id_a = guard.intern_module_id(&ModuleId::new(
            AccountAddress::ONE,
            ident_str!("a").to_owned(),
        ));

        let mut gas = GasMeter::with_max_budget();
        let before = gas.balance();
        loader.load_module(&mut gas, id_a).unwrap();
        (before - gas.balance(), guard.charged_modules().len())
    };

    // Prime the cache with a full package load, then hit it: the hit must
    // charge both members without fetching.
    let (charged_first, members_first) = load_once();
    let (charged_second, members_second) = load_once();

    assert!(charged_second > 0);
    assert_eq!(charged_first, charged_second);
    assert_eq!(members_first, members_second);
    assert_eq!(members_second, 2);
}
