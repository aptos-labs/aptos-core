// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Integration tests for type interning and metadata resolution.

use mono_move_global_context::{Executable, ExecutionGuard, GlobalContext};
use move_asm::assembler;
use move_core_types::ident_str;

/// Assembles a compiled module and adds it to the executable cache.
fn add_executable<'guard>(guard: &'guard ExecutionGuard<'_>, source: &str) -> &'guard Executable {
    let options = assembler::Options::default();
    let module = assembler::assemble(&options, source, std::iter::empty())
        .expect("Assembling should always succeed")
        .left()
        .expect("Only modules are expected");

    let executable = mono_move_orchestrator::build_executable(guard, &module)
        .expect("Building an executable should always succeed");

    let id = guard.intern_address_name(module.self_addr(), module.self_name());
    guard.insert_executable(id, executable)
}

#[test]
fn test_basic_struct() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();

    let executable = add_executable(
        &guard,
        r#"
module 0x1::a

struct A
    a: u8
    b: bool

struct B
    a: u256
    b: A
    c: address

enum C
    V1
        a: A
    V2
        a: u8
        b: B

struct D
    a: bool
    b: u8
    c: u64
    d: vector<u8>
    e: u128
    f: A
    g: B
    h: C
"#,
    );

    let name = guard.intern_identifier(ident_str!("B"));
    let layout = executable
        .get_struct(name.into_global_arena_ptr())
        .unwrap()
        .struct_layout()
        .unwrap();
    assert_eq!(layout.size, 96);
    assert_eq!(layout.align, 32);
    let offsets = layout
        .field_layouts()
        .iter()
        .map(|f| f.offset)
        .collect::<Vec<_>>();
    assert_eq!(offsets, vec![0, 32, 64]);

    let name = guard.intern_identifier(ident_str!("D"));
    let layout = executable
        .get_struct(name.into_global_arena_ptr())
        .unwrap()
        .struct_layout()
        .unwrap();
    assert_eq!(layout.size, 192);
    assert_eq!(layout.align, 32);
    let offsets = layout
        .field_layouts()
        .iter()
        .map(|f| f.offset)
        .collect::<Vec<_>>();
    assert_eq!(offsets, vec![0, 1, 8, 16, 32, 48, 64, 160]);

    let name = guard.intern_identifier(ident_str!("E"));
    assert!(executable
        .get_struct(name.into_global_arena_ptr())
        .is_none());
}
