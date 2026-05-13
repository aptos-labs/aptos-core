// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use mono_move_alloc::{ExecutableArena, ExecutableArenaPtr, GlobalArenaPtr};
use mono_move_core::{
    CodeOffset as CO, FrameLayoutInfo, FrameOffset as FO, Function, LocalExecutionContext, MicroOp,
    SortedSafePointEntries,
};
use mono_move_runtime::{InterpreterContext, ObjectDescriptor, ObjectDescriptorTable};

/// Data segment (48 bytes):
///   [fp + 0 ] : result (output) / scratch
///   [fp + 8 ] : vec_ptr (heap pointer to vector<u64>)
///   [fp + 16] : i (loop counter / len)
///   [fp + 24] : tmp (scratch)
///   [fp + 32] : vec_ref (16-byte fat pointer referencing vec_ptr)
fn make_vec_sum_program(
    arena: &ExecutableArena,
    n: u64,
) -> (Vec<ExecutableArenaPtr<Function>>, ObjectDescriptorTable) {
    use MicroOp::*;

    let slot_result: u32 = 0;
    let slot_vec: u32 = 8;
    let slot_i: u32 = 16;
    let slot_tmp: u32 = 24;
    let slot_vec_ref: u32 = 32;

    let mut descriptors = ObjectDescriptorTable::new();
    let desc_vec_u64 = descriptors.push(ObjectDescriptor::new_vector(8, vec![]).unwrap());

    #[rustfmt::skip]
    let code = arena.alloc_slice_fill_iter([
        VecNew { dst: FO(slot_vec) },
        SlotBorrow { dst: FO(slot_vec_ref), local: FO(slot_vec) },
        StoreImm8 { dst: FO(slot_i), imm: 0 },
        JumpGreaterEqualU64Imm { target: CO(9), src: FO(slot_i), imm: n },
        VecPushBack { vec_ref: FO(slot_vec_ref), elem: FO(slot_i), elem_size: 8, descriptor_id: desc_vec_u64 },
        StoreImm8 { dst: FO(slot_tmp), imm: 1 },
        AddU64 { dst: FO(slot_i), lhs: FO(slot_i), rhs: FO(slot_tmp) },
        JumpGreaterEqualU64Imm { target: CO(9), src: FO(slot_i), imm: n },
        JumpNotZeroU64 { target: CO(4), src: FO(slot_i) },
        StoreImm8 { dst: FO(slot_result), imm: 0 },
        VecLen { dst: FO(slot_i), vec_ref: FO(slot_vec_ref) },
        JumpNotZeroU64 { target: CO(13), src: FO(slot_i) },
        Return,
        VecPopBack { dst: FO(slot_tmp), vec_ref: FO(slot_vec_ref), elem_size: 8 },
        AddU64 { dst: FO(slot_result), lhs: FO(slot_result), rhs: FO(slot_tmp) },
        VecLen { dst: FO(slot_i), vec_ref: FO(slot_vec_ref) },
        JumpNotZeroU64 { target: CO(13), src: FO(slot_i) },
        Return,
    ]);
    let func = arena.alloc(Function {
        name: GlobalArenaPtr::from_static("test"),
        code,
        param_sizes: ExecutableArenaPtr::empty_slice(),
        param_sizes_sum: 0,
        param_and_local_sizes_sum: 48,
        extended_frame_size: 72,
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(arena, [FO(slot_vec), FO(slot_vec_ref)]),
        safe_point_layouts: SortedSafePointEntries::empty(),
    });

    (vec![func], descriptors)
}

#[test]
fn vec_sum_100() {
    let n: u64 = 100;
    let arena = ExecutableArena::new();
    let (functions, descriptors) = make_vec_sum_program(&arena, n);
    let mut exec_ctx = LocalExecutionContext::with_max_budget();
    let mut ctx = InterpreterContext::new(&mut exec_ctx, &descriptors, unsafe {
        functions[0].as_ref_unchecked()
    });
    ctx.run().unwrap();
    assert_eq!(ctx.root_result(), n * (n - 1) / 2);
}

#[test]
fn vec_sum_with_gc_pressure() {
    let n: u64 = 200;
    let arena = ExecutableArena::new();
    let (functions, descriptors) = make_vec_sum_program(&arena, n);
    let mut exec_ctx = LocalExecutionContext::with_max_budget();
    let mut ctx = InterpreterContext::with_heap_size(
        &mut exec_ctx,
        &descriptors,
        unsafe { functions[0].as_ref_unchecked() },
        4 * 1024,
    );
    ctx.run().unwrap();
    assert_eq!(ctx.root_result(), n * (n - 1) / 2);
    assert!(ctx.gc_count() > 0, "GC should have run at least once");
}
