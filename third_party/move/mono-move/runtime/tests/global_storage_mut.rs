// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Integration tests for the mutation MicroOps (`BorrowGlobalMut`,
//! `MoveTo`, `MoveFrom` with eager CoW), exercising deep copy and GC
//! tracing of local-heap writes.

mod common;

use common::InMemoryResources;
use mono_move_alloc::GlobalArenaPtr;
use mono_move_core::{
    types::{InternedType, Type},
    Code, DescriptorId, FrameLayoutInfo, FrameOffset as FO, Function, MicroOp, ObjectDescriptor,
    ObjectDescriptorTable, SortedSafePointEntries,
};
use mono_move_gas::SimpleGasMeter;
use mono_move_runtime::{error::RuntimeError, InterpreterContext, LocalRuntimeContext};
use move_core_types::account_address::AccountAddress;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

static RESOURCE_TY_NODE: Type = Type::U64;
static OTHER_RESOURCE_TY_NODE: Type = Type::U128;

fn resource_ty() -> InternedType {
    GlobalArenaPtr::from_static(&RESOURCE_TY_NODE)
}

fn other_resource_ty() -> InternedType {
    GlobalArenaPtr::from_static(&OTHER_RESOURCE_TY_NODE)
}

fn fresh_descriptors() -> (ObjectDescriptorTable, DescriptorId) {
    let mut table = ObjectDescriptorTable::new();
    let desc = table.push(ObjectDescriptor::new_struct(8, vec![]).unwrap());
    (table, desc)
}

fn addr(byte: u8) -> AccountAddress {
    let mut bytes = [0u8; AccountAddress::LENGTH];
    bytes[AccountAddress::LENGTH - 1] = byte;
    AccountAddress::new(bytes)
}

fn make_resource(value: u64) -> [u8; 8] {
    value.to_le_bytes()
}

fn local_ctx_with<'r>(
    resources: &'r InMemoryResources,
    descriptors: ObjectDescriptorTable,
) -> LocalRuntimeContext<'r, SimpleGasMeter> {
    LocalRuntimeContext::new(SimpleGasMeter::new(u64::MAX), resources, descriptors)
}

/// Frame layout for the mutation tests:
///   offset 0:  result slot (8 bytes)
///   offset 8:  addr slot (32 bytes)
///   offset 40: tmp slot (16 bytes, used as `dst` for borrow/move ops; a
///              mutable borrow writes a 16-byte fat-pointer reference)
/// Total locals = 56 bytes. With FRAME_METADATA_SIZE (24),
/// `extended_frame_size` must be ≥ 80.
fn make_program_with_tmp(code: Vec<MicroOp>, frame_layout: FrameLayoutInfo) -> Function {
    Function {
        name: GlobalArenaPtr::from_static("test"),
        module_id: crate::program_module_id!("test"),
        code: Code::from_vec(code),
        entry_gas: 0,
        param_slots: vec![],
        param_region_size: 0,
        param_and_local_sizes_sum: 56,
        extended_frame_size: 80,
        zero_frame: true,
        frame_layout,
        safe_point_layouts: SortedSafePointEntries::empty(),
    }
}

const DST: FO = FO(0);
const ADDR: FO = FO(8);
const TMP: FO = FO(40);
const SIGNER_REF: FO = FO(48);

// ---------------------------------------------------------------------------
// BorrowGlobalMut
// ---------------------------------------------------------------------------

#[test]
fn borrow_global_mut_deep_copies_external_resource() {
    let (descriptors, desc_id) = fresh_descriptors();
    let resources = InMemoryResources::new();
    let storage_ptr =
        resources.install_global(addr(1), resource_ty(), desc_id, &make_resource(0xAAAA));
    let mut exec_ctx = local_ctx_with(&resources, descriptors);

    let func = make_program_with_tmp(
        vec![
            MicroOp::BorrowGlobalMut {
                addr: ADDR,
                ty: resource_ty(),
                dst: TMP,
            },
            MicroOp::Return,
        ],
        FrameLayoutInfo::new(vec![TMP]),
    );

    let mut ctx = InterpreterContext::new(&mut exec_ctx, &func);
    ctx.set_root_arg(8, &addr(1).into_bytes());
    ctx.run().unwrap();

    let local_ptr = ctx.root_heap_ptr(40);
    assert_ne!(
        local_ptr as usize, storage_ptr as usize,
        "BorrowGlobalMut must hand a local-heap pointer, not the storage backend's pointer"
    );
    let local_bytes = unsafe { *(local_ptr as *const u64) };
    assert_eq!(local_bytes, 0xAAAA);
}

#[test]
fn borrow_global_mut_same_epoch_no_extra_copy() {
    let (descriptors, desc_id) = fresh_descriptors();
    let resources = InMemoryResources::new();
    resources.install_global(addr(2), resource_ty(), desc_id, &make_resource(0xBBBB));
    let mut exec_ctx = local_ctx_with(&resources, descriptors);

    let result_a: FO = FO(40);
    let result_b: FO = FO(56);
    let func = Function {
        name: GlobalArenaPtr::from_static("test"),
        module_id: crate::program_module_id!("test"),
        code: Code::from_vec(vec![
            MicroOp::BorrowGlobalMut {
                addr: ADDR,
                ty: resource_ty(),
                dst: result_a,
            },
            MicroOp::BorrowGlobalMut {
                addr: ADDR,
                ty: resource_ty(),
                dst: result_b,
            },
            MicroOp::Return,
        ]),
        entry_gas: 0,
        param_slots: vec![],
        param_region_size: 0,
        param_and_local_sizes_sum: 72,
        extended_frame_size: 96,
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(vec![result_a, result_b]),
        safe_point_layouts: SortedSafePointEntries::empty(),
    };

    let mut ctx = InterpreterContext::new(&mut exec_ctx, &func);
    ctx.set_root_arg(8, &addr(2).into_bytes());
    ctx.run().unwrap();

    let first = ctx.root_heap_ptr(40);
    let second = ctx.root_heap_ptr(56);
    assert_eq!(
        first as usize, second as usize,
        "second same-epoch BorrowGlobalMut must reuse the first's local-heap pointer"
    );
}

// ---------------------------------------------------------------------------
// MoveTo
// ---------------------------------------------------------------------------

#[test]
fn move_to_publishes_resource_and_exists_is_true() {
    let (descriptors, desc_id) = fresh_descriptors();
    let mut exec_ctx = LocalRuntimeContext::with_max_budget(descriptors);

    let func = Function {
        name: GlobalArenaPtr::from_static("test"),
        module_id: crate::program_module_id!("test"),
        code: Code::from_vec(vec![
            MicroOp::HeapNew {
                dst: TMP,
                descriptor_id: desc_id,
            },
            MicroOp::SlotBorrow {
                dst: SIGNER_REF,
                local: ADDR,
            },
            MicroOp::MoveTo {
                signer_ref: SIGNER_REF,
                ty: resource_ty(),
                src: TMP,
            },
            MicroOp::Exists {
                addr: ADDR,
                ty: resource_ty(),
                dst: DST,
            },
            MicroOp::Return,
        ]),
        entry_gas: 0,
        param_slots: vec![],
        param_region_size: 0,
        param_and_local_sizes_sum: 64,
        extended_frame_size: 88,
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(vec![TMP]),
        safe_point_layouts: SortedSafePointEntries::empty(),
    };

    let mut ctx = InterpreterContext::new(&mut exec_ctx, &func);
    ctx.set_root_arg(8, &addr(3).into_bytes());
    ctx.run().unwrap();

    assert_eq!(ctx.root_result(), 1, "Exists must report true after MoveTo");
}

#[test]
fn move_to_aborts_when_already_present() {
    let (descriptors, desc_id) = fresh_descriptors();
    let resources = InMemoryResources::new();
    resources.install_global(addr(4), resource_ty(), desc_id, &make_resource(0));
    let mut exec_ctx = local_ctx_with(&resources, descriptors);

    let func = Function {
        name: GlobalArenaPtr::from_static("test"),
        module_id: crate::program_module_id!("test"),
        code: Code::from_vec(vec![
            MicroOp::HeapNew {
                dst: TMP,
                descriptor_id: desc_id,
            },
            MicroOp::SlotBorrow {
                dst: SIGNER_REF,
                local: ADDR,
            },
            MicroOp::MoveTo {
                signer_ref: SIGNER_REF,
                ty: resource_ty(),
                src: TMP,
            },
            MicroOp::Return,
        ]),
        entry_gas: 0,
        param_slots: vec![],
        param_region_size: 0,
        param_and_local_sizes_sum: 64,
        extended_frame_size: 88,
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(vec![TMP]),
        safe_point_layouts: SortedSafePointEntries::empty(),
    };

    let mut ctx = InterpreterContext::new(&mut exec_ctx, &func);
    ctx.set_root_arg(8, &addr(4).into_bytes());
    assert!(matches!(
        ctx.run(),
        Err(RuntimeError::ResourceAlreadyExists { .. })
    ));
}

// ---------------------------------------------------------------------------
// MoveFrom with CoW
// ---------------------------------------------------------------------------

#[test]
fn move_from_deep_copies_external_resource() {
    let (descriptors, desc_id) = fresh_descriptors();
    let resources = InMemoryResources::new();
    let storage_ptr =
        resources.install_global(addr(5), resource_ty(), desc_id, &make_resource(0xCCCC));
    let mut exec_ctx = local_ctx_with(&resources, descriptors);

    let func = make_program_with_tmp(
        vec![
            MicroOp::MoveFrom {
                addr: ADDR,
                ty: resource_ty(),
                dst: TMP,
            },
            MicroOp::Return,
        ],
        FrameLayoutInfo::new(vec![TMP]),
    );

    let mut ctx = InterpreterContext::new(&mut exec_ctx, &func);
    ctx.set_root_arg(8, &addr(5).into_bytes());
    ctx.run().unwrap();

    let local_ptr = ctx.root_heap_ptr(40);
    assert_ne!(local_ptr as usize, storage_ptr as usize);
    let local_bytes = unsafe { *(local_ptr as *const u64) };
    assert_eq!(local_bytes, 0xCCCC);
}

// ---------------------------------------------------------------------------
// GC tracing of local-heap writes
// ---------------------------------------------------------------------------

#[test]
fn gc_traces_and_relocates_local_heap_writes() {
    let (descriptors, desc_id) = fresh_descriptors();
    let resources = InMemoryResources::new();
    resources.install_global(addr(6), resource_ty(), desc_id, &make_resource(0xDDDD));
    let mut exec_ctx = local_ctx_with(&resources, descriptors);

    let func = Function {
        name: GlobalArenaPtr::from_static("test"),
        module_id: crate::program_module_id!("test"),
        code: Code::from_vec(vec![
            MicroOp::BorrowGlobalMut {
                addr: ADDR,
                ty: resource_ty(),
                dst: TMP,
            },
            MicroOp::ForceGC,
            MicroOp::BorrowGlobal {
                addr: ADDR,
                ty: resource_ty(),
                dst: DST,
            },
            MicroOp::Return,
        ]),
        entry_gas: 0,
        param_slots: vec![],
        param_region_size: 0,
        param_and_local_sizes_sum: 56,
        extended_frame_size: 80,
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(vec![DST, TMP]),
        safe_point_layouts: SortedSafePointEntries::empty(),
    };

    let mut ctx = InterpreterContext::new(&mut exec_ctx, &func);
    ctx.set_root_arg(8, &addr(6).into_bytes());
    ctx.run().unwrap();

    let after_gc_ptr = ctx.root_heap_ptr(0);
    let tmp_ptr = ctx.root_heap_ptr(40);
    assert_eq!(
        after_gc_ptr as usize, tmp_ptr as usize,
        "BorrowGlobal after GC must return the same (relocated) local-heap pointer"
    );
    assert!(ctx.gc_count() >= 1, "ForceGC must have run");

    let bytes = unsafe { *(after_gc_ptr as *const u64) };
    assert_eq!(bytes, 0xDDDD);
}

// ---------------------------------------------------------------------------
// Distinct keys
// ---------------------------------------------------------------------------

#[test]
fn distinct_keys_track_independently() {
    let (descriptors, desc_id) = fresh_descriptors();
    let resources = InMemoryResources::new();
    resources.install_global(addr(7), resource_ty(), desc_id, &make_resource(0xEEE1));
    resources.install_global(
        addr(7),
        other_resource_ty(),
        desc_id,
        &make_resource(0xEEE2),
    );
    let mut exec_ctx = local_ctx_with(&resources, descriptors);

    let dst_a: FO = FO(40);
    let dst_b: FO = FO(56);
    let func = Function {
        name: GlobalArenaPtr::from_static("test"),
        module_id: crate::program_module_id!("test"),
        code: Code::from_vec(vec![
            MicroOp::BorrowGlobalMut {
                addr: ADDR,
                ty: resource_ty(),
                dst: dst_a,
            },
            MicroOp::BorrowGlobalMut {
                addr: ADDR,
                ty: other_resource_ty(),
                dst: dst_b,
            },
            MicroOp::Return,
        ]),
        entry_gas: 0,
        param_slots: vec![],
        param_region_size: 0,
        param_and_local_sizes_sum: 72,
        extended_frame_size: 96,
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(vec![dst_a, dst_b]),
        safe_point_layouts: SortedSafePointEntries::empty(),
    };

    let mut ctx = InterpreterContext::new(&mut exec_ctx, &func);
    ctx.set_root_arg(8, &addr(7).into_bytes());
    ctx.run().unwrap();

    let a_ptr = ctx.root_heap_ptr(40);
    let b_ptr = ctx.root_heap_ptr(56);
    assert_ne!(a_ptr as usize, b_ptr as usize);
    let a_val = unsafe { *(a_ptr as *const u64) };
    let b_val = unsafe { *(b_ptr as *const u64) };
    assert_eq!(a_val, 0xEEE1);
    assert_eq!(b_val, 0xEEE2);
}
