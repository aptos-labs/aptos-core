// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Integration tests for type interning and metadata resolution.

use mono_move_core::Interner;
use mono_move_gas::NoOpGasMeter;
use mono_move_global_context::{view_type, GlobalContext};
use mono_move_loader::{Loader, LoadingPolicy, LoweringPolicy, ModuleReadSet};
use mono_move_testsuite::InMemoryModuleProvider;
use move_core_types::{account_address::AccountAddress, ident_str};

#[test]
fn test_basic_struct() {
    let modules = mono_move_testsuite::compile_move_source(
        r#"
module 0x1::a {
  struct A has drop { a: u8, b: bool }
  struct B has drop { a: u256, b: A, c: address }
  enum C has drop { V1 { a: A }, V2 { a: u8, b: B } }
  struct D has drop {
    a: bool,
    b: u8,
    c: u64,
    d: vector<u8>,
    e: u128,
    f: A,
    g: B,
    h: C,
  }

  public fun test(_x: D) {}
}
"#,
    )
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

    let mut read_set = ModuleReadSet::new();
    let mut gas_meter = NoOpGasMeter;

    let id = guard.intern_address_name(&AccountAddress::ONE, ident_str!("a"));
    let ir = loader
        .load_module(&mut read_set, &mut gas_meter, id)
        .unwrap()
        .ir();

    let idx = ir
        .module
        .interned_nominal_type_def_idx(guard.identifier_of(ident_str!("B")))
        .unwrap();
    let ty = ir.module.interned_nominal_def_type_at(idx);
    let layout = view_type(ty).layout().unwrap();
    assert_eq!(layout.size, 72);
    assert_eq!(layout.align, 8);
    let offsets = layout
        .field_layouts()
        .expect("Struct layout carries per-field offsets")
        .iter()
        .map(|f| f.offset)
        .collect::<Vec<_>>();
    assert_eq!(offsets, vec![0, 32, 40]);

    let idx = ir
        .module
        .interned_nominal_type_def_idx(guard.identifier_of(ident_str!("D")))
        .unwrap();
    let ty = ir.module.interned_nominal_def_type_at(idx);
    let layout = view_type(ty).layout().unwrap();
    assert_eq!(layout.size, 128);
    assert_eq!(layout.align, 8);
    let offsets = layout
        .field_layouts()
        .expect("Struct layout carries per-field offsets")
        .iter()
        .map(|f| f.offset)
        .collect::<Vec<_>>();
    assert_eq!(offsets, vec![0, 1, 8, 16, 24, 40, 48, 120]);

    let idx = ir
        .module
        .interned_nominal_type_def_idx(guard.identifier_of(ident_str!("C")))
        .unwrap();
    let ty = ir.module.interned_nominal_def_type_at(idx);
    let layout = view_type(ty).layout().unwrap();
    assert_eq!(layout.size, 8);
    assert_eq!(layout.align, 8);
    assert!(layout.field_layouts().is_none());
}
