// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use mono_move_runtime::{
    CodeOffset as CO, FrameOffset as FO, Function, InterpreterContext, MicroOp, ObjectDescriptor,
};

/// Data segment (32 bytes):
///   [fp + 0 ] : result (output) / scratch
///   [fp + 8 ] : vec_ptr (heap pointer to vector<u64>)
///   [fp + 16] : i (loop counter / len)
///   [fp + 24] : tmp (scratch)
fn make_vec_sum_program(n: u64) -> (Vec<Function>, Vec<ObjectDescriptor>) {
    use MicroOp::*;

    let slot_result: u32 = 0;
    let slot_vec: u32 = 8;
    let slot_i: u32 = 16;
    let slot_tmp: u32 = 24;

    #[rustfmt::skip]
    let code = vec![
        VecNew { dst: FO(slot_vec), descriptor_id: 0, elem_size: 8, initial_capacity: 4 },
        StoreImm8 { dst: FO(slot_i), imm: 0 },
        JumpGreaterEqualU64Imm { target: CO(8), src: FO(slot_i), imm: n },
        VecPushBack { heap_ptr: FO(slot_vec), elem: FO(slot_i), elem_size: 8 },
        StoreImm8 { dst: FO(slot_tmp), imm: 1 },
        AddU64 { dst: FO(slot_i), lhs: FO(slot_i), rhs: FO(slot_tmp) },
        JumpGreaterEqualU64Imm { target: CO(8), src: FO(slot_i), imm: n },
        JumpNotZeroU64 { target: CO(3), src: FO(slot_i) },
        StoreImm8 { dst: FO(slot_result), imm: 0 },
        VecLen { dst: FO(slot_i), heap_ptr: FO(slot_vec) },
        JumpNotZeroU64 { target: CO(12), src: FO(slot_i) },
        Return,
        VecPopBack { dst: FO(slot_tmp), heap_ptr: FO(slot_vec), elem_size: 8 },
        AddU64 { dst: FO(slot_result), lhs: FO(slot_result), rhs: FO(slot_tmp) },
        VecLen { dst: FO(slot_i), heap_ptr: FO(slot_vec) },
        JumpNotZeroU64 { target: CO(12), src: FO(slot_i) },
        Return,
    ];

    let func = Function {
        code,
        args_size: 0,
        data_size: 32,
        extended_frame_size: 56,
        zero_locals: true,
        pointer_slots: vec![FO(slot_vec)],
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
