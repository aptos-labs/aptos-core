// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_experimental_new_interpreter::{
    interpreter::InterpreterContext, Function, Instruction, ObjectDescriptor, FRAME_METADATA_SIZE,
};
use std::collections::HashMap;

/// Frame layout (48 bytes):
///   [fp +  0] : n (input) / result (output)
///   [fp +  8] : temp – holds fib(n-1)
///   [fp + 16] : frame metadata start (written by CallFunc, 24 bytes)
///   [fp + 40] : callee's slot 0 (u64)
fn make_fun_fib(fib_func_id: usize) -> Function {
    use Instruction::*;

    let metadata_offset: u32 = 16;
    let callee_slot0: u32 = metadata_offset + FRAME_METADATA_SIZE as u32;

    #[rustfmt::skip]
    let code = vec![
        JumpIfNotZero { src_fp_offset: 0, dst_pc: 3 },
        StoreU64 { dst_fp_offset: 0, val: 0 },
        Return,
        JumpIfGreaterEqualU64Const { src_fp_offset: 0, dst_pc: 6, val: 2 },
        StoreU64 { dst_fp_offset: 0, val: 1 },
        Return,
        SubU64Const { src_fp_offset: 0, val: 1, dst_fp_offset: callee_slot0 },
        CallFunc { func_id: fib_func_id },
        Mov8 { src_fp_offset: callee_slot0, dst_fp_offset: 8 },
        SubU64Const { src_fp_offset: 0, val: 2, dst_fp_offset: callee_slot0 },
        CallFunc { func_id: fib_func_id },
        AddU64 { src_fp_offset_1: 8, src_fp_offset_2: callee_slot0, dst_fp_offset: 0 },
        Return,
    ];

    let mut stack_maps = HashMap::new();
    stack_maps.insert(8, vec![]);  // return site after 1st CallFunc
    stack_maps.insert(11, vec![]); // return site after 2nd CallFunc

    Function {
        code,
        data_size: 16,
        extended_frame_size: 48,
        stack_maps,
    }
}

#[test]
fn fib_10() {
    let functions = vec![make_fun_fib(0)];
    let descriptors = vec![ObjectDescriptor::Trivial];
    let mut ctx = InterpreterContext::new(&functions, &descriptors, 0, &10u64.to_le_bytes());
    ctx.run().unwrap();
    assert_eq!(ctx.root_result(), 55);
}

#[test]
fn fib_0() {
    let functions = vec![make_fun_fib(0)];
    let descriptors = vec![ObjectDescriptor::Trivial];
    let mut ctx = InterpreterContext::new(&functions, &descriptors, 0, &0u64.to_le_bytes());
    ctx.run().unwrap();
    assert_eq!(ctx.root_result(), 0);
}

#[test]
fn fib_1() {
    let functions = vec![make_fun_fib(0)];
    let descriptors = vec![ObjectDescriptor::Trivial];
    let mut ctx = InterpreterContext::new(&functions, &descriptors, 0, &1u64.to_le_bytes());
    ctx.run().unwrap();
    assert_eq!(ctx.root_result(), 1);
}

#[test]
fn fib_20() {
    let functions = vec![make_fun_fib(0)];
    let descriptors = vec![ObjectDescriptor::Trivial];
    let mut ctx = InterpreterContext::new(&functions, &descriptors, 0, &20u64.to_le_bytes());
    ctx.run().unwrap();
    assert_eq!(ctx.root_result(), 6765);
}
