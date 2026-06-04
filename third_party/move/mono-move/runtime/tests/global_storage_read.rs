// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Integration tests for the global-storage MicroOps (`Exists`,
//! `BorrowGlobal`, `BorrowGlobalMut`, `MoveFrom`). Each test builds a
//! one-function program that invokes exactly one global-storage
//! MicroOp, pre-populates an in-memory [`ResourceProvider`], wraps it
//! in [`LocalExecutionContext`], and inspects the root frame.

mod common;

use common::InMemoryResources;
use mono_move_alloc::GlobalArenaPtr;
use mono_move_core::{
    types::{InternedType, Type},
    Code, DescriptorId, FrameLayoutInfo, FrameOffset as FO, Function, MicroOp, ObjectDescriptor,
    ObjectDescriptorTable, SortedSafePointEntries,
};
use mono_move_gas::SimpleGasMeter;
use mono_move_runtime::{
    error::{GlobalStorageOp, RuntimeError},
    InterpreterContext, LocalRuntimeContext,
};
use move_core_types::account_address::AccountAddress;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

static RESOURCE_TY_NODE: Type = Type::U64;

fn resource_ty() -> InternedType {
    GlobalArenaPtr::from_static(&RESOURCE_TY_NODE)
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

/// Slot layout: result at 0 (8B), addr at 8 (32B). Locals total 40
/// bytes. Verifier requires `extended_frame_size >= 40 +
/// FRAME_METADATA_SIZE (24) = 64`.
fn make_program(code: Vec<MicroOp>) -> Function {
    Function {
        name: GlobalArenaPtr::from_static("test"),
        code: Code::from_vec(code),
        param_slots: vec![],
        param_region_size: 0,
        param_and_local_sizes_sum: 40,
        extended_frame_size: 64,
        zero_frame: false,
        frame_layout: FrameLayoutInfo::empty(),
        safe_point_layouts: SortedSafePointEntries::empty(),
    }
}

const DST: FO = FO(0);
const ADDR: FO = FO(8);

// ---------------------------------------------------------------------------
// Exists
// ---------------------------------------------------------------------------

#[test]
fn exists_returns_false_for_absent() {
    let (descriptors, _) = fresh_descriptors();
    let func = make_program(vec![
        MicroOp::Exists {
            addr: ADDR,
            ty: resource_ty(),
            dst: DST,
        },
        MicroOp::Return,
    ]);

    let mut exec_ctx = LocalRuntimeContext::with_max_budget(descriptors);
    let mut ctx = InterpreterContext::new(&mut exec_ctx, &func);
    ctx.set_root_arg(8, &addr(1).into_bytes());
    ctx.run().unwrap();

    assert_eq!(ctx.root_result(), 0, "missing resource exists as false");
}

#[test]
fn exists_returns_true_for_present() {
    let (descriptors, desc_id) = fresh_descriptors();
    let resources = InMemoryResources::new();
    resources.install_global(addr(1), resource_ty(), desc_id, &make_resource(42));
    let mut exec_ctx = local_ctx_with(&resources, descriptors);

    let func = make_program(vec![
        MicroOp::Exists {
            addr: ADDR,
            ty: resource_ty(),
            dst: DST,
        },
        MicroOp::Return,
    ]);

    let mut ctx = InterpreterContext::new(&mut exec_ctx, &func);
    ctx.set_root_arg(8, &addr(1).into_bytes());
    ctx.run().unwrap();

    assert_eq!(ctx.root_result(), 1, "present resource exists as true");
}

#[test]
fn exists_returns_false_after_move_from() {
    let (descriptors, desc_id) = fresh_descriptors();
    let resources = InMemoryResources::new();
    resources.install_global(addr(1), resource_ty(), desc_id, &make_resource(42));
    let mut exec_ctx = local_ctx_with(&resources, descriptors);

    let tmp: FO = FO(40);
    let func = Function {
        name: GlobalArenaPtr::from_static("test"),
        code: Code::from_vec(vec![
            MicroOp::MoveFrom {
                addr: ADDR,
                ty: resource_ty(),
                dst: tmp,
            },
            MicroOp::Exists {
                addr: ADDR,
                ty: resource_ty(),
                dst: DST,
            },
            MicroOp::Return,
        ]),
        param_slots: vec![],
        param_region_size: 0,
        param_and_local_sizes_sum: 48,
        extended_frame_size: 72,
        zero_frame: false,
        frame_layout: FrameLayoutInfo::empty(),
        safe_point_layouts: SortedSafePointEntries::empty(),
    };

    let mut ctx = InterpreterContext::new(&mut exec_ctx, &func);
    ctx.set_root_arg(8, &addr(1).into_bytes());
    ctx.run().unwrap();

    assert_eq!(
        ctx.root_result(),
        0,
        "post-MoveFrom Exists must observe Deleted"
    );
}

// ---------------------------------------------------------------------------
// BorrowGlobal
// ---------------------------------------------------------------------------

#[test]
fn borrow_global_returns_storage_pointer_zero_copy() {
    let (descriptors, desc_id) = fresh_descriptors();
    let resources = InMemoryResources::new();
    let expected_ptr =
        resources.install_global(addr(2), resource_ty(), desc_id, &make_resource(0xDEAD_BEEF));
    let mut exec_ctx = local_ctx_with(&resources, descriptors);

    let func = make_program(vec![
        MicroOp::BorrowGlobal {
            addr: ADDR,
            ty: resource_ty(),
            dst: DST,
        },
        MicroOp::Return,
    ]);

    let mut ctx = InterpreterContext::new(&mut exec_ctx, &func);
    ctx.set_root_arg(8, &addr(2).into_bytes());
    ctx.run().unwrap();

    let observed_ptr = ctx.root_heap_ptr(0);
    assert_eq!(
        observed_ptr as usize, expected_ptr as usize,
        "BorrowGlobal must hand the caller the storage backend's pointer verbatim (zero-copy)"
    );
}

#[test]
fn borrow_global_aborts_on_missing() {
    let (descriptors, _) = fresh_descriptors();
    let func = make_program(vec![
        MicroOp::BorrowGlobal {
            addr: ADDR,
            ty: resource_ty(),
            dst: DST,
        },
        MicroOp::Return,
    ]);

    let mut exec_ctx = LocalRuntimeContext::with_max_budget(descriptors);
    let mut ctx = InterpreterContext::new(&mut exec_ctx, &func);
    ctx.set_root_arg(8, &addr(9).into_bytes());
    assert!(matches!(
        ctx.run(),
        Err(RuntimeError::ResourceDoesNotExist {
            op: GlobalStorageOp::BorrowGlobal,
            ..
        })
    ));
}

// ---------------------------------------------------------------------------
// BorrowGlobalMut
// ---------------------------------------------------------------------------

#[test]
fn borrow_global_mut_aborts_on_missing() {
    let (descriptors, _) = fresh_descriptors();
    let func = make_program(vec![
        MicroOp::BorrowGlobalMut {
            addr: ADDR,
            ty: resource_ty(),
            dst: DST,
        },
        MicroOp::Return,
    ]);

    let mut exec_ctx = LocalRuntimeContext::with_max_budget(descriptors);
    let mut ctx = InterpreterContext::new(&mut exec_ctx, &func);
    ctx.set_root_arg(8, &addr(9).into_bytes());
    assert!(matches!(
        ctx.run(),
        Err(RuntimeError::ResourceDoesNotExist {
            op: GlobalStorageOp::BorrowGlobalMut,
            ..
        })
    ));
}

// ---------------------------------------------------------------------------
// MoveFrom
// ---------------------------------------------------------------------------

#[test]
fn move_from_aborts_on_missing() {
    let (descriptors, _) = fresh_descriptors();
    let func = make_program(vec![
        MicroOp::MoveFrom {
            addr: ADDR,
            ty: resource_ty(),
            dst: DST,
        },
        MicroOp::Return,
    ]);

    let mut exec_ctx = LocalRuntimeContext::with_max_budget(descriptors);
    let mut ctx = InterpreterContext::new(&mut exec_ctx, &func);
    ctx.set_root_arg(8, &addr(9).into_bytes());
    assert!(matches!(
        ctx.run(),
        Err(RuntimeError::ResourceDoesNotExist {
            op: GlobalStorageOp::MoveFrom,
            ..
        })
    ));
}

#[test]
fn move_from_marks_deleted_and_second_borrow_aborts() {
    let (descriptors, desc_id) = fresh_descriptors();
    let resources = InMemoryResources::new();
    resources.install_global(addr(3), resource_ty(), desc_id, &make_resource(42));
    let mut exec_ctx = local_ctx_with(&resources, descriptors);

    let tmp: FO = FO(40);
    let func = Function {
        name: GlobalArenaPtr::from_static("test"),
        code: Code::from_vec(vec![
            MicroOp::MoveFrom {
                addr: ADDR,
                ty: resource_ty(),
                dst: tmp,
            },
            MicroOp::BorrowGlobal {
                addr: ADDR,
                ty: resource_ty(),
                dst: DST,
            },
            MicroOp::Return,
        ]),
        param_slots: vec![],
        param_region_size: 0,
        param_and_local_sizes_sum: 48,
        extended_frame_size: 72,
        zero_frame: false,
        frame_layout: FrameLayoutInfo::empty(),
        safe_point_layouts: SortedSafePointEntries::empty(),
    };

    let mut ctx = InterpreterContext::new(&mut exec_ctx, &func);
    ctx.set_root_arg(8, &addr(3).into_bytes());
    assert!(matches!(
        ctx.run(),
        Err(RuntimeError::ResourceDoesNotExist {
            op: GlobalStorageOp::BorrowGlobal,
            ..
        })
    ));
}

// ---------------------------------------------------------------------------
// GC: ForceGC must not relocate `Read::ExternalHeap` pointers — the
// storage backend owns them and the data lives outside the local heap.
// ---------------------------------------------------------------------------

#[test]
fn force_gc_does_not_disturb_external_read() {
    let (descriptors, desc_id) = fresh_descriptors();
    let resources = InMemoryResources::new();
    let expected_ptr =
        resources.install_global(addr(4), resource_ty(), desc_id, &make_resource(0x1234));
    let mut exec_ctx = local_ctx_with(&resources, descriptors);

    let func = make_program(vec![
        MicroOp::BorrowGlobal {
            addr: ADDR,
            ty: resource_ty(),
            dst: DST,
        },
        MicroOp::ForceGC,
        MicroOp::BorrowGlobal {
            addr: ADDR,
            ty: resource_ty(),
            dst: DST,
        },
        MicroOp::Return,
    ]);

    let mut ctx = InterpreterContext::new(&mut exec_ctx, &func);
    ctx.set_root_arg(8, &addr(4).into_bytes());
    ctx.run().unwrap();

    assert_eq!(ctx.root_heap_ptr(0) as usize, expected_ptr as usize);
    assert!(ctx.gc_count() >= 1, "ForceGC must have run");
}
