// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Integration tests for the Package loading policy.

use mono_move_gas::{GasMeter, SimpleGasMeter};
use mono_move_global_context::GlobalContext;
use mono_move_loader::{ExecutableReadSet, Loader, LoadingPolicy};
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
    let modules = mono_move_testsuite::compile_move_modules(TEST_SOURCE);
    let mut module_provider = InMemoryModuleProvider::new();
    module_provider.add_modules(&modules);
    module_provider.declare_package(AccountAddress::ONE, ident_str!("a").to_owned(), vec![
        ident_str!("b").to_owned(),
    ]);

    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();
    let loader = Loader::new_with_policy(&guard, &module_provider, LoadingPolicy::Package);

    let id_a_module = ModuleId::new(AccountAddress::ONE, ident_str!("a").to_owned());
    let id_a = guard.intern_module_id(&id_a_module);

    let mut read_set = ExecutableReadSet::new();
    let mut gas = SimpleGasMeter::new(u64::MAX);
    let exec = loader.load(&mut read_set, &mut gas, id_a).unwrap();

    // Both package members must be in the read-set.
    assert_eq!(read_set.len(), 2);

    // mandatory_dependencies covers every package member, including
    // self. For a 2-module package, that's both slots.
    assert_eq!(exec.mandatory_dependencies().slots().len(), 2);

    // The sibling must also be loadable from the read-set directly.
    let id_b = ModuleId::new(AccountAddress::ONE, ident_str!("b").to_owned());
    let key_b = guard.intern_module_id(&id_b);
    assert!(read_set.get(key_b).is_some());
}

#[test]
fn load_package_cache_hit_walks_dependencies() {
    let modules = mono_move_testsuite::compile_move_modules(TEST_SOURCE);
    let mut module_provider = InMemoryModuleProvider::new();
    module_provider.add_modules(&modules);
    module_provider.declare_package(AccountAddress::ONE, ident_str!("a").to_owned(), vec![
        ident_str!("b").to_owned(),
    ]);

    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();
    let loader = Loader::new_with_policy(&guard, &module_provider, LoadingPolicy::Package);

    let id_a_module = ModuleId::new(AccountAddress::ONE, ident_str!("a").to_owned());
    let id_a = guard.intern_module_id(&id_a_module);

    // Prime the cache with a full package load.
    let mut rs1 = ExecutableReadSet::new();
    let mut g1 = SimpleGasMeter::new(u64::MAX);
    loader.load(&mut rs1, &mut g1, id_a).unwrap();

    // Second call with a fresh read-set must hit the cache and charge both
    // members without fetching.
    let mut rs2 = ExecutableReadSet::new();
    let mut g2 = SimpleGasMeter::new(u64::MAX);
    let before = g2.balance();
    loader.load(&mut rs2, &mut g2, id_a).unwrap();
    let charged = before - g2.balance();
    assert!(charged > 0);
    assert_eq!(rs2.len(), 2);
}
