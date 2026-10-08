// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Integration tests for script loading.
//!
//! A script has no module ID to key the module table by, so it takes the
//! reserved index 0 and lives in a slot on the execution guard rather than in
//! the table.

use mono_move_core::{native::NoNatives, types::EMPTY_TYPE_LIST, GasMeter};
use mono_move_global_context::{GlobalContext, LoadedModule, SCRIPT_MODULE_IDX};
use mono_move_loader::{Loader, LoadingPolicy, LoweringPolicy};
use mono_move_testsuite::{compile_move_script, InMemoryModuleProvider};

const SCRIPT_SOURCE: &str = r#"
script {
    fun main() {}
}
"#;

const OTHER_SCRIPT_SOURCE: &str = r#"
script {
    fun main(_x: u64) {}
}
"#;

/// Serialized bytes of the single script `source` defines.
fn script_bytes(source: &str) -> Vec<u8> {
    let script = compile_move_script(source).expect("source compiles");
    let mut bytes = vec![];
    script.serialize(&mut bytes).expect("the script serializes");
    bytes
}

#[test]
fn script_takes_the_reserved_module_index() {
    let module_provider = InMemoryModuleProvider::new();
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();
    let loader = Loader::new_with_policy(
        &guard,
        &module_provider,
        LoadingPolicy::Lazy(LoweringPolicy::Lazy),
        &NoNatives,
    );

    assert!(
        guard.module_at(SCRIPT_MODULE_IDX).is_none(),
        "the script slot starts empty"
    );

    let mut gas = GasMeter::with_max_budget();
    loader
        .load_script(&mut gas, &script_bytes(SCRIPT_SOURCE), EMPTY_TYPE_LIST)
        .expect("the script loads");

    let script = guard
        .module_at(SCRIPT_MODULE_IDX)
        .expect("the loaded script answers to the reserved index");
    assert!(guard.is_charged(SCRIPT_MODULE_IDX));
    assert_eq!(guard.charged_modules(), vec![SCRIPT_MODULE_IDX]);

    // A transaction runs at most one script, but a caller may run several
    // under one guard. The reserved index must follow the script it is
    // running, not the one it ran first.
    loader
        .load_script(&mut gas, &script_bytes(OTHER_SCRIPT_SOURCE), EMPTY_TYPE_LIST)
        .expect("the second script loads");
    assert!(
        !std::ptr::eq(script, guard.module_at(SCRIPT_MODULE_IDX).unwrap()),
        "the reserved index must follow the second script"
    );
    assert_eq!(guard.charged_modules(), vec![SCRIPT_MODULE_IDX]);
}

#[test]
fn script_slot_does_not_outlive_its_guard() {
    let module_provider = InMemoryModuleProvider::new();
    let ctx = GlobalContext::with_num_execution_workers(1);

    // Loads `source` under a fresh guard and returns the identity of whatever
    // the reserved index resolves to afterwards.
    let load_under_fresh_guard = |source| {
        let guard = ctx.try_execution_context(0).unwrap();
        let loader = Loader::new_with_policy(
            &guard,
            &module_provider,
            LoadingPolicy::Lazy(LoweringPolicy::Lazy),
            &NoNatives,
        );
        let mut gas = GasMeter::with_max_budget();
        loader
            .load_script(&mut gas, &script_bytes(source), EMPTY_TYPE_LIST)
            .expect("the script loads");
        guard.module_at(SCRIPT_MODULE_IDX).unwrap() as *const LoadedModule
    };

    let first = load_under_fresh_guard(SCRIPT_SOURCE);
    let second = load_under_fresh_guard(OTHER_SCRIPT_SOURCE);
    assert!(
        !std::ptr::eq(first, second),
        "the reserved index must resolve to the script of the running execution"
    );

    // A guard that loads no script sees an empty slot, even though the index
    // itself is reserved for the lifetime of the context.
    let guard = ctx.try_execution_context(0).unwrap();
    assert!(guard.module_at(SCRIPT_MODULE_IDX).is_none());
    assert!(!guard.is_charged(SCRIPT_MODULE_IDX));
}
