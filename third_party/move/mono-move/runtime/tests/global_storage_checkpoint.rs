// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Integration tests for `checkpoint()` and `rollback(n)`. These
//! tests drive the methods directly on the interpreter between
//! explicit MicroOp programs. The session model the Aptos VM
//! eventually invokes — prologue checkpoint, user checkpoint,
//! epilogue — is exercised piecewise here.

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
    error::{RuntimeError, RuntimeInvariantViolation},
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

const DST: FO = FO(0);
const ADDR: FO = FO(8);
const TMP: FO = FO(40);

fn make_program(code: Vec<MicroOp>, frame_layout: FrameLayoutInfo) -> Function {
    Function {
        name: GlobalArenaPtr::from_static("test"),
        code: Code::from_vec(code),
        param_sizes: vec![],
        param_sizes_sum: 0,
        param_and_local_sizes_sum: 56,
        extended_frame_size: 80,
        zero_frame: true,
        frame_layout,
        safe_point_layouts: SortedSafePointEntries::empty(),
    }
}

fn trivial_program() -> Function {
    make_program(vec![MicroOp::Return], FrameLayoutInfo::empty())
}

// ---------------------------------------------------------------------------
// Checkpoint / rollback API
// ---------------------------------------------------------------------------

#[test]
fn checkpoint_advances_epoch_and_records_stack() {
    let descriptors = ObjectDescriptorTable::new();
    let func = trivial_program();
    let mut exec_ctx = LocalRuntimeContext::with_max_budget(descriptors);
    let mut ctx = InterpreterContext::new(&mut exec_ctx, &func);

    assert_eq!(ctx.current_epoch(), 0);
    assert_eq!(ctx.checkpoint_depth(), 0);

    ctx.checkpoint();
    assert_eq!(ctx.current_epoch(), 1);
    assert_eq!(ctx.checkpoint_depth(), 1);

    ctx.checkpoint();
    assert_eq!(ctx.current_epoch(), 2);
    assert_eq!(ctx.checkpoint_depth(), 2);
}

#[test]
fn rollback_zero_is_noop() {
    let descriptors = ObjectDescriptorTable::new();
    let func = trivial_program();
    let mut exec_ctx = LocalRuntimeContext::with_max_budget(descriptors);
    let mut ctx = InterpreterContext::new(&mut exec_ctx, &func);

    ctx.checkpoint();
    ctx.rollback(0).unwrap();
    assert_eq!(ctx.current_epoch(), 1);
    assert_eq!(ctx.checkpoint_depth(), 1);
}

#[test]
fn rollback_more_than_stack_depth_aborts() {
    let descriptors = ObjectDescriptorTable::new();
    let func = trivial_program();
    let mut exec_ctx = LocalRuntimeContext::with_max_budget(descriptors);
    let mut ctx = InterpreterContext::new(&mut exec_ctx, &func);

    ctx.checkpoint();
    assert!(matches!(
        ctx.rollback(2),
        Err(RuntimeError::InvariantViolation(
            RuntimeInvariantViolation::RollbackUnderflow {
                requested: 2,
                available: 1,
            }
        ))
    ));
}

#[test]
fn rollback_restores_pre_mutation_state() {
    let (descriptors, desc_id) = fresh_descriptors();
    let resources = InMemoryResources::new();
    resources.install_global(addr(1), resource_ty(), desc_id, &make_resource(0xAAAA));
    let mut exec_ctx = local_ctx_with(&resources, descriptors);

    let func = make_program(
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

    ctx.checkpoint();
    ctx.set_root_arg(8, &addr(1).into_bytes());
    ctx.run().unwrap();
    assert_eq!(ctx.journal_len(), 1, "BorrowGlobalMut must journal once");

    ctx.rollback(1).unwrap();
    assert_eq!(ctx.journal_len(), 0);
    assert_eq!(ctx.checkpoint_depth(), 0);
    assert_eq!(ctx.current_epoch(), 0);

    let exists_func = make_program(
        vec![
            MicroOp::Exists {
                addr: ADDR,
                ty: resource_ty(),
                dst: DST,
            },
            MicroOp::Return,
        ],
        FrameLayoutInfo::empty(),
    );
    ctx.invoke(&exists_func);
    ctx.set_root_arg(8, &addr(1).into_bytes());
    ctx.run().unwrap();
    assert_eq!(
        ctx.root_result(),
        1,
        "resource still present after rollback"
    );
}

#[test]
fn rollback_of_move_from_then_move_to_restores_to_deleted() {
    // Design doc headline case:
    //   1. start with resource present in storage.
    //   2. MoveFrom (write becomes Deleted).
    //   3. checkpoint().
    //   4. MoveTo with fresh value (write becomes LocalHeap).
    //   5. rollback(1) → must restore write=Deleted (not None).
    //   6. Exists must return false.
    let (descriptors, desc_id) = fresh_descriptors();
    let resources = InMemoryResources::new();
    resources.install_global(addr(2), resource_ty(), desc_id, &make_resource(0xBBBB));
    let mut exec_ctx = local_ctx_with(&resources, descriptors);

    let move_from_func = make_program(
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
    let move_to_func = make_program(
        vec![
            MicroOp::HeapNew {
                dst: TMP,
                descriptor_id: desc_id,
            },
            MicroOp::MoveTo {
                addr: ADDR,
                ty: resource_ty(),
                src: TMP,
            },
            MicroOp::Return,
        ],
        FrameLayoutInfo::new(vec![TMP]),
    );
    let exists_func = make_program(
        vec![
            MicroOp::Exists {
                addr: ADDR,
                ty: resource_ty(),
                dst: DST,
            },
            MicroOp::Return,
        ],
        FrameLayoutInfo::empty(),
    );

    let mut ctx = InterpreterContext::new(&mut exec_ctx, &move_from_func);

    ctx.set_root_arg(8, &addr(2).into_bytes());
    ctx.run().unwrap();
    assert_eq!(ctx.journal_len(), 1);

    ctx.checkpoint();
    assert_eq!(ctx.current_epoch(), 1);
    assert_eq!(ctx.journal_len(), 1);

    ctx.invoke(&move_to_func);
    ctx.set_root_arg(8, &addr(2).into_bytes());
    ctx.run().unwrap();
    assert_eq!(
        ctx.journal_len(),
        2,
        "MoveTo in a new epoch must add a journal entry"
    );

    ctx.rollback(1).unwrap();
    assert_eq!(ctx.journal_len(), 1);
    assert_eq!(ctx.current_epoch(), 0);

    ctx.invoke(&exists_func);
    ctx.set_root_arg(8, &addr(2).into_bytes());
    ctx.run().unwrap();
    assert_eq!(
        ctx.root_result(),
        0,
        "rollback of MoveTo after MoveFrom must restore Deleted, so Exists is false"
    );
}

#[test]
fn same_epoch_second_mutation_no_journal_growth() {
    let (descriptors, desc_id) = fresh_descriptors();
    let resources = InMemoryResources::new();
    resources.install_global(addr(3), resource_ty(), desc_id, &make_resource(0xCCCC));
    let mut exec_ctx = local_ctx_with(&resources, descriptors);

    let func = make_program(
        vec![
            MicroOp::BorrowGlobalMut {
                addr: ADDR,
                ty: resource_ty(),
                dst: TMP,
            },
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
    ctx.set_root_arg(8, &addr(3).into_bytes());
    ctx.run().unwrap();
    assert_eq!(
        ctx.journal_len(),
        1,
        "two same-epoch BorrowGlobalMut on same key must journal exactly once"
    );
}

#[test]
fn journal_grows_for_first_mutation_each_epoch() {
    let (descriptors, desc_id) = fresh_descriptors();
    let resources = InMemoryResources::new();
    resources.install_global(addr(4), resource_ty(), desc_id, &make_resource(0xDDDD));
    let mut exec_ctx = local_ctx_with(&resources, descriptors);

    let func = make_program(
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

    ctx.set_root_arg(8, &addr(4).into_bytes());
    ctx.run().unwrap();
    assert_eq!(ctx.journal_len(), 1);

    ctx.checkpoint();
    ctx.invoke(&func);
    ctx.set_root_arg(8, &addr(4).into_bytes());
    ctx.run().unwrap();
    assert_eq!(
        ctx.journal_len(),
        2,
        "first BorrowGlobalMut in a new epoch must add a journal entry"
    );
}

#[test]
fn rollback_n_collapses_multiple_checkpoints() {
    let (descriptors, desc_id) = fresh_descriptors();
    let resources = InMemoryResources::new();
    resources.install_global(addr(5), resource_ty(), desc_id, &make_resource(0xEEEE));
    let mut exec_ctx = local_ctx_with(&resources, descriptors);

    let func = make_program(
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

    ctx.checkpoint();
    ctx.set_root_arg(8, &addr(5).into_bytes());
    ctx.run().unwrap();
    ctx.checkpoint();
    ctx.invoke(&func);
    ctx.set_root_arg(8, &addr(5).into_bytes());
    ctx.run().unwrap();
    ctx.checkpoint();
    ctx.invoke(&func);
    ctx.set_root_arg(8, &addr(5).into_bytes());
    ctx.run().unwrap();

    assert_eq!(ctx.checkpoint_depth(), 3);
    assert_eq!(ctx.current_epoch(), 3);
    assert_eq!(ctx.journal_len(), 3);

    ctx.rollback(3).unwrap();
    assert_eq!(ctx.checkpoint_depth(), 0);
    assert_eq!(ctx.current_epoch(), 0);
    assert_eq!(ctx.journal_len(), 0);
}

#[test]
fn rollback_then_remutate_journals_again() {
    let (descriptors, desc_id) = fresh_descriptors();
    let resources = InMemoryResources::new();
    resources.install_global(addr(6), resource_ty(), desc_id, &make_resource(0xFFFF));
    let mut exec_ctx = local_ctx_with(&resources, descriptors);

    let func = make_program(
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

    ctx.checkpoint();
    ctx.set_root_arg(8, &addr(6).into_bytes());
    ctx.run().unwrap();
    assert_eq!(ctx.journal_len(), 1);

    ctx.rollback(1).unwrap();
    assert_eq!(ctx.journal_len(), 0);

    // After rollback the entry still exists in the read-write set
    // with write=None, so a fresh BorrowGlobalMut sees write.epoch()
    // == None and journals again.
    ctx.invoke(&func);
    ctx.set_root_arg(8, &addr(6).into_bytes());
    ctx.run().unwrap();
    assert_eq!(ctx.journal_len(), 1);
}
