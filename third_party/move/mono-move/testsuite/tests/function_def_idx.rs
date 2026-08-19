// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Function definition indexes end to end: stamped on lowered [`Function`]s,
//! resolvable by name on a loaded module (natives included), and recoverable
//! from the natives registry via `name_by_idx`.

use mono_move_core::{
    native::NativeIdx, types::EMPTY_TYPE_LIST, FunctionDefinitionIndex, GasMeter,
};
use mono_move_global_context::GlobalContext;
use mono_move_loader::{Loader, LoadingPolicy, LoweringPolicy, ModuleReadSet};
use mono_move_runtime::ProductionNativeRegistry;
use mono_move_testsuite::{engine::build_natives, InMemoryModuleProvider};
use move_binary_format::access::ModuleAccess;
use move_core_types::{account_address::AccountAddress, ident_str, identifier::IdentStr};

/// A native before and after `main` so the Move-body def index is neither 0
/// nor the last — a zero-default or off-by-one stamp fails the assertions.
const SOURCE: &str = r#"
module 0x1::defs {
    native public fun first_native(): u64;
    public fun main(x: u64): u64 { x + 1 }
    native public fun second_native(): u64;
}
"#;

#[test]
fn def_idx_stamped_and_resolvable_by_name() {
    let modules = mono_move_testsuite::compile_move_source(SOURCE).expect("compilation failed");
    let module = &modules[0];
    let def_position = |name: &str| {
        let position = module
            .function_defs()
            .iter()
            .position(|fdef| {
                module
                    .identifier_at(module.function_handle_at(fdef.function).name)
                    .as_str()
                    == name
            })
            .expect("function not found in compiled module");
        FunctionDefinitionIndex(position as u16)
    };

    let mut module_provider = InMemoryModuleProvider::new();
    module_provider.add_modules(&modules);

    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx
        .try_execution_context(0)
        .expect("worker 0 execution context must be available");
    let natives = ProductionNativeRegistry::new();
    let loader = Loader::new_with_policy(
        &guard,
        &module_provider,
        LoadingPolicy::Lazy(LoweringPolicy::Lazy),
        &natives,
    );

    let module_id = guard.intern_address_name(&AccountAddress::ONE, ident_str!("defs"));
    let main_name = guard
        .intern_identifier(ident_str!("main"))
        .into_global_arena_ptr();
    let mut read_set = ModuleReadSet::new();
    let mut gas_meter = GasMeter::with_max_budget();
    let main_ptr = loader
        .load_function(
            &mut read_set,
            &mut gas_meter,
            module_id.into_global_arena_ptr(),
            main_name,
            EMPTY_TYPE_LIST,
        )
        .expect("loading main must succeed");

    // The lowered function carries its definition index.
    // SAFETY: the guard is alive, so the function is not reclaimed.
    let main_fn = unsafe { main_ptr.as_ref_unchecked() };
    assert_eq!(main_fn.def_idx, def_position("main"));

    // Name → def-index lookup on the loaded module covers natives too.
    let loaded = guard.get_module(module_id).expect("module must be cached");
    for name in ["first_native", "main", "second_native"] {
        let interned = guard
            .intern_identifier(IdentStr::new(name).expect("test names are valid identifiers"))
            .into_global_arena_ptr();
        assert_eq!(loaded.function_def_idx(interned), Some(def_position(name)));
    }
    let absent = guard
        .intern_identifier(ident_str!("absent"))
        .into_global_arena_ptr();
    assert_eq!(loaded.function_def_idx(absent), None);
}

#[test]
fn registry_name_by_idx_round_trips() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx
        .try_execution_context(0)
        .expect("worker 0 execution context must be available");
    let natives = build_natives(&guard);
    assert!(!natives.is_empty());
    for raw_idx in 0..natives.len() as u32 {
        let idx = NativeIdx(raw_idx);
        let name = natives.name_by_idx(idx).expect("index in range");
        assert!(natives.module_by_idx(idx) == Some(name.module()));
    }
    assert!(natives
        .name_by_idx(NativeIdx(natives.len() as u32))
        .is_none());
}
