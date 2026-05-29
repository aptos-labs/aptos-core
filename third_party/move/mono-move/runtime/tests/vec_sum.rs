// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use mono_move_alloc::GlobalArenaPtr;
use mono_move_core::{
    types::U64_TY, Code, CodeOffset as CO, FrameLayoutInfo, FrameOffset as FO, Function, MicroOp,
    SortedSafePointEntries,
};
use mono_move_runtime::{
    InterpreterContext, LocalRuntimeContext, ObjectDescriptor, ObjectDescriptorTable,
};

/// Data segment (48 bytes):
///   [fp + 0 ] : result (output) / scratch
///   [fp + 8 ] : vec_ptr (heap pointer to vector<u64>)
///   [fp + 16] : i (loop counter / len)
///   [fp + 24] : tmp (scratch)
///   [fp + 32] : vec_ref (16-byte fat pointer referencing vec_ptr)
fn make_vec_sum_program(n: u64) -> (Vec<Function>, ObjectDescriptorTable) {
    use MicroOp::*;

    let slot_result: u32 = 0;
    let slot_vec: u32 = 8;
    let slot_i: u32 = 16;
    let slot_tmp: u32 = 24;
    let slot_vec_ref: u32 = 32;

    let mut descriptors = ObjectDescriptorTable::new();
    let vec_ty = U64_TY;
    descriptors.push_for_type(vec_ty, ObjectDescriptor::new_vector(8, vec![]).unwrap());

    #[rustfmt::skip]
    let code = vec![
        VecNew { dst: FO(slot_vec) },
        SlotBorrow { dst: FO(slot_vec_ref), local: FO(slot_vec) },
        StoreImm8 { dst: FO(slot_i), imm: 0u64.to_le_bytes() },
        JumpGreaterEqualU64Imm { target: CO(9), src: FO(slot_i), imm: n },
        VecPushBack { vec_ref: FO(slot_vec_ref), elem: FO(slot_i), elem_size: 8, vec_ty },
        StoreImm8 { dst: FO(slot_tmp), imm: 1u64.to_le_bytes() },
        AddU64 { dst: FO(slot_i), lhs: FO(slot_i), rhs: FO(slot_tmp) },
        JumpGreaterEqualU64Imm { target: CO(9), src: FO(slot_i), imm: n },
        JumpNotZeroU64 { target: CO(4), src: FO(slot_i) },
        StoreImm8 { dst: FO(slot_result), imm: 0u64.to_le_bytes() },
        VecLen { dst: FO(slot_i), vec_ref: FO(slot_vec_ref) },
        JumpNotZeroU64 { target: CO(13), src: FO(slot_i) },
        Return,
        VecPopBack { dst: FO(slot_tmp), vec_ref: FO(slot_vec_ref), elem_size: 8 },
        AddU64 { dst: FO(slot_result), lhs: FO(slot_result), rhs: FO(slot_tmp) },
        VecLen { dst: FO(slot_i), vec_ref: FO(slot_vec_ref) },
        JumpNotZeroU64 { target: CO(13), src: FO(slot_i) },
        Return,
    ];
    let func = Function {
        name: GlobalArenaPtr::from_static("test"),
        code: Code::from_vec(code),
        param_sizes: vec![],
        param_sizes_sum: 0,
        param_and_local_sizes_sum: 48,
        extended_frame_size: 72,
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(vec![FO(slot_vec), FO(slot_vec_ref)]),
        safe_point_layouts: SortedSafePointEntries::empty(),
    };

    (vec![func], descriptors)
}

#[test]
fn vec_sum_100() {
    let n: u64 = 100;
    let (functions, descriptors) = make_vec_sum_program(n);
    let mut exec_ctx = LocalRuntimeContext::with_max_budget(descriptors);
    let mut ctx = InterpreterContext::new(&mut exec_ctx, &functions[0]);
    ctx.run().unwrap();
    assert_eq!(ctx.root_result(), n * (n - 1) / 2);
}

#[test]
fn vec_sum_with_gc_pressure() {
    let n: u64 = 200;
    let (functions, descriptors) = make_vec_sum_program(n);
    let mut exec_ctx = LocalRuntimeContext::with_max_budget(descriptors);
    let mut ctx = InterpreterContext::with_heap_size(&mut exec_ctx, &functions[0], 4 * 1024);
    ctx.run().unwrap();
    assert_eq!(ctx.root_result(), n * (n - 1) / 2);
    assert!(ctx.gc_count() > 0, "GC should have run at least once");
}
