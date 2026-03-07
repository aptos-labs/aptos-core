// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_experimental_new_interpreter::{
    interpreter::InterpreterContext, read_u64, Function, Instruction, ObjectDescriptor,
    FRAME_METADATA_SIZE, VEC_DATA_OFFSET,
};
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::collections::HashMap;

/// Fisher-Yates shuffle of [0, 1, ..., n-1] using a seeded StdRng.
fn shuffled_range(n: u64, seed: u64) -> Vec<u64> {
    let mut values: Vec<u64> = (0..n).collect();
    let mut rng = StdRng::seed_from_u64(seed);
    for i in (1..n as usize).rev() {
        let j = rng.gen_range(0, i + 1);
        values.swap(i, j);
    }
    values
}

/// Three functions:
///   func 0 (main)       — builds vec from `values`, calls merge_sort, returns vec[0]
///   func 1 (merge_sort) — recursive sort on vec[lo..hi)
///   func 2 (merge)      — merge two sorted halves in place
fn make_merge_sort_program(values: &[u64]) -> (Vec<Function>, Vec<ObjectDescriptor>) {
    use Instruction::*;

    let n = values.len() as u64;
    let meta = FRAME_METADATA_SIZE as u32;

    // ===================================================================
    // Function 0 — main
    //
    // Frame layout:
    //   [0]  result   [8]  vec    [16] tmp    [24] (unused)
    //   [32] metadata (24 bytes)
    //   [56] callee 0 (vec)  [64] callee 1 (lo)  [72] callee 2 (hi)
    // ===================================================================
    let (r, v, tmp) = (0u32, 8, 16);
    let meta0 = 32u32;
    let c0 = meta0 + meta;
    let (c1, c2) = (c0 + 8, c0 + 16);

    let mut main_code: Vec<Instruction> = vec![VecNew {
        descriptor_id: 0,
        elem_size: 8,
        initial_capacity: 4,
        dst_fp_offset: v,
    }];
    let mut sm0 = HashMap::new();
    sm0.insert(0, vec![]);

    for &val in values {
        main_code.push(StoreU64 {
            dst_fp_offset: tmp,
            val,
        });
        let push_pc = main_code.len();
        main_code.push(VecPushBack {
            vec_fp_offset: v,
            elem_fp_offset: tmp,
            elem_size: 8,
        });
        sm0.insert(push_pc, vec![v]);
    }

    main_code.push(Mov8 {
        src_fp_offset: v,
        dst_fp_offset: c0,
    });
    main_code.push(StoreU64 {
        dst_fp_offset: c1,
        val: 0,
    });
    main_code.push(StoreU64 {
        dst_fp_offset: c2,
        val: n,
    });
    main_code.push(CallFunc { func_id: 1 });
    let return_pc = main_code.len();
    sm0.insert(return_pc, vec![v]);

    main_code.push(StoreU64 {
        dst_fp_offset: tmp,
        val: 0,
    });
    main_code.push(VecLoadElem {
        vec_fp_offset: v,
        idx_fp_offset: tmp,
        dst_fp_offset: r,
        elem_size: 8,
    });
    main_code.push(Return);

    let func_main = Function {
        code: main_code,
        data_size: 32,
        extended_frame_size: 80,
        stack_maps: sm0,
    };

    // ===================================================================
    // Function 1 — merge_sort(vec, lo, hi)
    //
    // Frame layout:
    //   [0]  vec  [8]  lo  [16] hi  [24] mid  [32] scratch
    //   [40] metadata (24 bytes)
    //   [64] callee 0  [72] callee 1  [80] callee 2  [88] callee 3
    // ===================================================================
    let (sv, slo, shi, smid, stmp) = (0u32, 8, 16, 24, 32);
    let meta1 = 40u32;
    let q0 = meta1 + meta;
    let (q1, q2, q3) = (q0 + 8, q0 + 16, q0 + 24);

    #[rustfmt::skip]
    let ms_code = vec![
        AddU64Const { src_fp_offset: slo, val: 1, dst_fp_offset: stmp },
        JumpIfLessU64 { lhs_fp_offset: stmp, rhs_fp_offset: shi, dst_pc: 3 },
        Return,
        AddU64 { src_fp_offset_1: slo, src_fp_offset_2: shi, dst_fp_offset: smid },
        ShrU64Const { src_fp_offset: smid, val: 1, dst_fp_offset: smid },
        Mov8 { src_fp_offset: sv, dst_fp_offset: q0 },
        Mov8 { src_fp_offset: slo, dst_fp_offset: q1 },
        Mov8 { src_fp_offset: smid, dst_fp_offset: q2 },
        CallFunc { func_id: 1 },
        Mov8 { src_fp_offset: sv, dst_fp_offset: q0 },
        Mov8 { src_fp_offset: smid, dst_fp_offset: q1 },
        Mov8 { src_fp_offset: shi, dst_fp_offset: q2 },
        CallFunc { func_id: 1 },
        Mov8 { src_fp_offset: sv, dst_fp_offset: q0 },
        Mov8 { src_fp_offset: slo, dst_fp_offset: q1 },
        Mov8 { src_fp_offset: smid, dst_fp_offset: q2 },
        Mov8 { src_fp_offset: shi, dst_fp_offset: q3 },
        CallFunc { func_id: 2 },
        Return,
    ];

    let mut sm1 = HashMap::new();
    sm1.insert(9, vec![sv]);
    sm1.insert(13, vec![sv]);
    sm1.insert(18, vec![sv]);

    let func_merge_sort = Function {
        code: ms_code,
        data_size: 40,
        extended_frame_size: 96,
        stack_maps: sm1,
    };

    // ===================================================================
    // Function 2 — merge(vec, lo, mid, hi)
    //
    // Frame layout:
    //   [0]  vec     [8]  lo      [16] mid     [24] hi
    //   [32] tmp_vec [40] i       [48] j
    //   [56] elem_a  [64] elem_b  [72] k       [80] tmp_idx
    // ===================================================================
    let (mv, mlo, mmid, mhi) = (0u32, 8, 16, 24);
    let (mtv, mi, mj) = (32u32, 40, 48);
    let (ma, mb, mk, mtidx) = (56u32, 64, 72, 80);

    #[rustfmt::skip]
    let merge_code = vec![
        Mov8 { src_fp_offset: mlo, dst_fp_offset: mi },
        Mov8 { src_fp_offset: mmid, dst_fp_offset: mj },
        VecNew { descriptor_id: 0, elem_size: 8, initial_capacity: 4, dst_fp_offset: mtv },
        // MERGE_LOOP (pc 3)
        JumpIfLessU64 { lhs_fp_offset: mi, rhs_fp_offset: mmid, dst_pc: 6 },
        JumpIfLessU64 { lhs_fp_offset: mj, rhs_fp_offset: mhi, dst_pc: 23 },
        Jump { dst_pc: 29 },
        JumpIfLessU64 { lhs_fp_offset: mj, rhs_fp_offset: mhi, dst_pc: 8 },
        Jump { dst_pc: 17 },
        VecLoadElem { vec_fp_offset: mv, idx_fp_offset: mi, dst_fp_offset: ma, elem_size: 8 },
        VecLoadElem { vec_fp_offset: mv, idx_fp_offset: mj, dst_fp_offset: mb, elem_size: 8 },
        JumpIfLessU64 { lhs_fp_offset: ma, rhs_fp_offset: mb, dst_pc: 14 },
        VecPushBack { vec_fp_offset: mtv, elem_fp_offset: mb, elem_size: 8 },
        AddU64Const { src_fp_offset: mj, val: 1, dst_fp_offset: mj },
        Jump { dst_pc: 3 },
        // PUSH_LEFT (pc 14)
        VecPushBack { vec_fp_offset: mtv, elem_fp_offset: ma, elem_size: 8 },
        AddU64Const { src_fp_offset: mi, val: 1, dst_fp_offset: mi },
        Jump { dst_pc: 3 },
        // DRAIN_LEFT (pc 17)
        JumpIfLessU64 { lhs_fp_offset: mi, rhs_fp_offset: mmid, dst_pc: 19 },
        Jump { dst_pc: 29 },
        VecLoadElem { vec_fp_offset: mv, idx_fp_offset: mi, dst_fp_offset: ma, elem_size: 8 },
        VecPushBack { vec_fp_offset: mtv, elem_fp_offset: ma, elem_size: 8 },
        AddU64Const { src_fp_offset: mi, val: 1, dst_fp_offset: mi },
        Jump { dst_pc: 17 },
        // DRAIN_RIGHT (pc 23)
        JumpIfLessU64 { lhs_fp_offset: mj, rhs_fp_offset: mhi, dst_pc: 25 },
        Jump { dst_pc: 29 },
        VecLoadElem { vec_fp_offset: mv, idx_fp_offset: mj, dst_fp_offset: mb, elem_size: 8 },
        VecPushBack { vec_fp_offset: mtv, elem_fp_offset: mb, elem_size: 8 },
        AddU64Const { src_fp_offset: mj, val: 1, dst_fp_offset: mj },
        Jump { dst_pc: 23 },
        // COPY_BACK (pc 29)
        Mov8 { src_fp_offset: mlo, dst_fp_offset: mk },
        StoreU64 { dst_fp_offset: mtidx, val: 0 },
        // COPY_LOOP (pc 31)
        JumpIfLessU64 { lhs_fp_offset: mk, rhs_fp_offset: mhi, dst_pc: 33 },
        Return,
        VecLoadElem { vec_fp_offset: mtv, idx_fp_offset: mtidx, dst_fp_offset: ma, elem_size: 8 },
        VecStoreElem { vec_fp_offset: mv, idx_fp_offset: mk, src_fp_offset: ma, elem_size: 8 },
        AddU64Const { src_fp_offset: mk, val: 1, dst_fp_offset: mk },
        AddU64Const { src_fp_offset: mtidx, val: 1, dst_fp_offset: mtidx },
        Jump { dst_pc: 31 },
    ];

    let mut sm2 = HashMap::new();
    sm2.insert(2, vec![mv]);
    sm2.insert(11, vec![mv, mtv]);
    sm2.insert(14, vec![mv, mtv]);
    sm2.insert(20, vec![mv, mtv]);
    sm2.insert(26, vec![mv, mtv]);

    let func_merge = Function {
        code: merge_code,
        data_size: 88,
        extended_frame_size: 112,
        stack_maps: sm2,
    };

    let descriptors = vec![ObjectDescriptor::Trivial];
    (vec![func_main, func_merge_sort, func_merge], descriptors)
}

fn verify_sorted(ctx: &InterpreterContext, n: u64) {
    let vec_ptr = ctx.root_heap_ptr(8);
    for idx in 0..n {
        let elem = unsafe { read_u64(vec_ptr, VEC_DATA_OFFSET + idx as usize * 8) };
        assert_eq!(elem, idx, "vec[{}] = {} but expected {}", idx, elem, idx);
    }
}

#[test]
fn merge_sort_small() {
    let values = shuffled_range(10, 42);
    let (functions, descriptors) = make_merge_sort_program(&values);
    let mut ctx = InterpreterContext::new(&functions, &descriptors, 0, &[]);
    ctx.run().unwrap();
    verify_sorted(&ctx, 10);
}

#[test]
fn merge_sort_large_with_gc() {
    let values = shuffled_range(1000, 123456789);
    let (functions, descriptors) = make_merge_sort_program(&values);
    let mut ctx = InterpreterContext::with_heap_size(&functions, &descriptors, 0, &[], 24 * 1024);
    ctx.run().unwrap();
    verify_sorted(&ctx, 1000);
    assert!(ctx.gc_count() > 10, "GC should have run more than 10 times");
}
