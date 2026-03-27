// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use mono_move_alloc::GlobalArenaPtr;
use mono_move_core::{CodeOffset as CO, DescriptorId, FrameOffset as FO, Function, MicroOp};
use mono_move_runtime::{InterpreterContext, ObjectDescriptor};

/// Data segment (48 bytes):
///   [fp + 0 ] : result (output) / scratch
///   [fp + 8 ] : vec_ptr (heap pointer to vector<u64>)
///   [fp + 16] : i (loop counter / len)
///   [fp + 24] : tmp (scratch)
///   [fp + 32] : vec_ref (16-byte fat pointer referencing vec_ptr)
fn make_vec_sum_program(n: u64) -> (Vec<Function>, Vec<ObjectDescriptor>) {
    use MicroOp::*;

    let slot_result: u32 = 0;
    let slot_vec: u32 = 8;
    let slot_i: u32 = 16;
    let slot_tmp: u32 = 24;
    let slot_vec_ref: u32 = 32;

    #[rustfmt::skip]
    let code = vec![
        VecNew { dst: FO(slot_vec) },
        SlotBorrow { dst: FO(slot_vec_ref), local: FO(slot_vec) },
        StoreImm8 { dst: FO(slot_i), imm: 0 },
        JumpGreaterEqualU64Imm { target: CO(9), src: FO(slot_i), imm: n },
        VecPushBack { vec_ref: FO(slot_vec_ref), elem: FO(slot_i), elem_size: 8, descriptor_id: DescriptorId(0) },
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
    ];

    let func = Function {
        name: GlobalArenaPtr::from_static("test"),
        code,
        args_size: 0,
        args_and_locals_size: 48,
        extended_frame_size: 72,
        zero_frame: true,
        pointer_offsets: vec![FO(slot_vec), FO(slot_vec_ref)],
    };

    let descriptors = vec![ObjectDescriptor::Trivial];
    (vec![func], descriptors)
}

#[test]
fn vec_sum_100() {
    let n: u64 = 100;
    let (functions, descriptors) = make_vec_sum_program(n);
    let mut ctx = InterpreterContext::new(&functions, &descriptors, 0);
    ctx.run().unwrap();
    assert_eq!(ctx.root_result(), n * (n - 1) / 2);
}

#[test]
fn vec_sum_with_gc_pressure() {
    let n: u64 = 200;
    let (functions, descriptors) = make_vec_sum_program(n);
    let mut ctx = InterpreterContext::with_heap_size(&functions, &descriptors, 0, 4 * 1024);
    ctx.run().unwrap();
    assert_eq!(ctx.root_result(), n * (n - 1) / 2);
    assert!(ctx.gc_count() > 0, "GC should have run at least once");
}
