// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Integration tests for the Lazy loading policy.

use mono_move_gas::{GasMeter, SimpleGasMeter};
use mono_move_global_context::GlobalContext;
use mono_move_loader::{ExecutableReadSet, Loader, LoadingPolicy, LoweringPolicy};
use mono_move_testsuite::InMemoryModuleProvider;
use move_core_types::{account_address::AccountAddress, ident_str, language_storage::ModuleId};

const TEST_SOURCE: &str = r#"
module 0x1::test {
    fun identity(x: u64): u64 { x }
}
"#;

#[test]
fn load_lazy_cache_miss_and_hit() {
    let modules = mono_move_testsuite::compile_move_modules(TEST_SOURCE);
    let mut module_provider = InMemoryModuleProvider::new();
    module_provider.add_modules(&modules);

    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();
    let loader = Loader::new_with_policy(
        &guard,
        &module_provider,
        LoadingPolicy::Lazy(LoweringPolicy::Lazy),
    );

    let id_module = ModuleId::new(AccountAddress::ONE, ident_str!("test").to_owned());
    let id = guard.intern_module_id(&id_module);

    // First call is a cache miss: fetches, deserializes, builds, installs.
    let mut read_set = ExecutableReadSet::new();
    let mut gas = SimpleGasMeter::new(u64::MAX);
    let gas_before = gas.balance();
    let exec = loader.load(&mut read_set, &mut gas, id).unwrap();
    let first_cost = exec.cost();
    assert!(first_cost > 0, "cost should reflect bytecode size");
    assert_eq!(gas_before - gas.balance(), first_cost);
    assert_eq!(read_set.len(), 1);
    // Lazy policy: no dependency slots (self is handled separately).
    assert!(exec.mandatory_dependencies().slots().is_empty());

    // Second call on a fresh read-set is a cache hit: charges the same
    // cost, records without fetching.
    let mut read_set2 = ExecutableReadSet::new();
    let mut gas2 = SimpleGasMeter::new(u64::MAX);
    let gas_before2 = gas2.balance();
    let exec2 = loader.load(&mut read_set2, &mut gas2, id).unwrap();
    assert_eq!(exec2.cost(), first_cost);
    assert_eq!(gas_before2 - gas2.balance(), first_cost);
    assert_eq!(read_set2.len(), 1);
}
