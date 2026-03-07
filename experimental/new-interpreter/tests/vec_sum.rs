// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_experimental_new_interpreter::{
    interpreter::InterpreterContext, Function, Instruction, ObjectDescriptor,
};
use std::collections::HashMap;

/// Data segment (32 bytes):
///   [fp + 0 ] : result (output) / scratch
///   [fp + 8 ] : vec_ptr (heap pointer to vector<u64>)
///   [fp + 16] : i (loop counter / len)
///   [fp + 24] : tmp (scratch)
fn make_vec_sum_program(n: u64) -> (Vec<Function>, Vec<ObjectDescriptor>) {
    use Instruction::*;

    let slot_result: u32 = 0;
    let slot_vec: u32 = 8;
    let slot_i: u32 = 16;
    let slot_tmp: u32 = 24;

    #[rustfmt::skip]
    let code = vec![
        VecNew { descriptor_id: 0, elem_size: 8, initial_capacity: 4, dst_fp_offset: slot_vec },
        StoreU64 { dst_fp_offset: slot_i, val: 0 },
        JumpIfGreaterEqualU64Const { src_fp_offset: slot_i, dst_pc: 8, val: n },
        VecPushBack { vec_fp_offset: slot_vec, elem_fp_offset: slot_i, elem_size: 8 },
        StoreU64 { dst_fp_offset: slot_tmp, val: 1 },
        AddU64 { src_fp_offset_1: slot_i, src_fp_offset_2: slot_tmp, dst_fp_offset: slot_i },
        JumpIfGreaterEqualU64Const { src_fp_offset: slot_i, dst_pc: 8, val: n },
        JumpIfNotZero { src_fp_offset: slot_i, dst_pc: 3 },
        StoreU64 { dst_fp_offset: slot_result, val: 0 },
        VecLen { vec_fp_offset: slot_vec, dst_fp_offset: slot_i },
        JumpIfNotZero { src_fp_offset: slot_i, dst_pc: 12 },
        Return,
        VecPopBack { vec_fp_offset: slot_vec, dst_fp_offset: slot_tmp, elem_size: 8 },
        AddU64 { src_fp_offset_1: slot_result, src_fp_offset_2: slot_tmp, dst_fp_offset: slot_result },
        VecLen { vec_fp_offset: slot_vec, dst_fp_offset: slot_i },
        JumpIfNotZero { src_fp_offset: slot_i, dst_pc: 12 },
        Return,
    ];

    let mut stack_maps = HashMap::new();
    stack_maps.insert(0, vec![]);
    stack_maps.insert(3, vec![slot_vec]);

    let func = Function {
        code,
        data_size: 32,
        extended_frame_size: 56,
        stack_maps,
    };

    let descriptors = vec![ObjectDescriptor::Trivial];
    (vec![func], descriptors)
}

#[test]
fn vec_sum_100() {
    let n: u64 = 100;
    let (functions, descriptors) = make_vec_sum_program(n);
    let mut ctx = InterpreterContext::new(&functions, &descriptors, 0, &[]);
    ctx.run().unwrap();
    assert_eq!(ctx.root_result(), n * (n - 1) / 2);
}

#[test]
fn vec_sum_with_gc_pressure() {
    let n: u64 = 200;
    let (functions, descriptors) = make_vec_sum_program(n);
    let mut ctx = InterpreterContext::with_heap_size(&functions, &descriptors, 0, &[], 4 * 1024);
    ctx.run().unwrap();
    assert_eq!(ctx.root_result(), n * (n - 1) / 2);
    assert!(ctx.gc_count() > 0, "GC should have run at least once");
}
