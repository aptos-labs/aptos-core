// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for fat-pointer references (VecBorrow, BorrowLocal, ReadRef, WriteRef,
//! StructBorrow).

use aptos_experimental_new_interpreter::{
    interpreter::InterpreterContext, read_ptr, read_u64, Function, Instruction, ObjectDescriptor,
    FRAME_METADATA_SIZE, STRUCT_DATA_OFFSET, VEC_DATA_OFFSET,
};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Test 1: ref_basic — single-frame, no GC
//
// Creates vector<u64> = [10, 20, 30], borrows element 1 (value 20),
// reads through the reference, writes 99 through it, then verifies via
// VecLoadElem that element 1 is now 99.
// ---------------------------------------------------------------------------

/// Data segment (56 bytes):
///   [fp +  0] : result (u64)
///   [fp +  8] : vec    (heap pointer)
///   [fp + 16] : idx    (u64)
///   [fp + 24] : tmp    (u64 scratch)
///   [fp + 32] : ref    (fat pointer: base @ +32, offset @ +40)
///   [fp + 48] : val    (u64 scratch for write)
#[test]
fn ref_basic() {
    use Instruction::*;

    let result: u32 = 0;
    let vec: u32 = 8;
    let idx: u32 = 16;
    let tmp: u32 = 24;
    let r#ref: u32 = 32;
    let val: u32 = 48;

    #[rustfmt::skip]
    let code = vec![
        VecNew { descriptor_id: 0, elem_size: 8, initial_capacity: 4, dst_fp_offset: vec },
        StoreU64 { dst_fp_offset: tmp, val: 10 },
        VecPushBack { vec_fp_offset: vec, elem_fp_offset: tmp, elem_size: 8 },
        StoreU64 { dst_fp_offset: tmp, val: 20 },
        VecPushBack { vec_fp_offset: vec, elem_fp_offset: tmp, elem_size: 8 },
        StoreU64 { dst_fp_offset: tmp, val: 30 },
        VecPushBack { vec_fp_offset: vec, elem_fp_offset: tmp, elem_size: 8 },
        StoreU64 { dst_fp_offset: idx, val: 1 },
        VecBorrow { vec_fp_offset: vec, idx_fp_offset: idx, elem_size: 8, dst_fp_offset: r#ref },
        ReadRef { ref_fp_offset: r#ref, dst_fp_offset: result, size: 8 },
        StoreU64 { dst_fp_offset: val, val: 99 },
        WriteRef { ref_fp_offset: r#ref, src_fp_offset: val, size: 8 },
        VecLoadElem { vec_fp_offset: vec, idx_fp_offset: idx, dst_fp_offset: tmp, elem_size: 8 },
        Return,
    ];

    let mut stack_maps = HashMap::new();
    stack_maps.insert(0, vec![]);
    stack_maps.insert(2, vec![vec]);
    stack_maps.insert(4, vec![vec]);
    stack_maps.insert(6, vec![vec]);

    let functions = [Function {
        code,
        data_size: 56,
        extended_frame_size: 80,
        stack_maps,
    }];
    let descriptors = vec![ObjectDescriptor::Trivial];
    let mut ctx = InterpreterContext::new(&functions, &descriptors, 0, &[]);
    ctx.run().unwrap();

    assert_eq!(ctx.root_result(), 20, "ReadRef should have read 20 from vec[1]");
    let vec_ptr = ctx.root_heap_ptr(8);
    let elem1 = unsafe { read_u64(vec_ptr, VEC_DATA_OFFSET + 8) };
    assert_eq!(elem1, 99, "WriteRef should have written 99 to vec[1]");
}

// ---------------------------------------------------------------------------
// Test 2: ref_survives_gc — single-frame, ForceGC between borrow and deref
//
// Creates a vector [100, 200, 300], borrows element 2 (300), then forces
// a GC cycle. After GC relocates the vector, reads/writes through the
// fat pointer to verify the base was updated correctly.
// ---------------------------------------------------------------------------

/// Data segment (48 bytes):
///   [fp +  0] : result  (u64)
///   [fp +  8] : vec     (heap pointer)
///   [fp + 16] : idx     (u64)
///   [fp + 24] : tmp     (u64 scratch)
///   [fp + 32] : ref     (fat pointer: base @ +32, offset @ +40)
#[test]
fn ref_survives_gc() {
    use Instruction::*;

    let result: u32 = 0;
    let vec: u32 = 8;
    let idx: u32 = 16;
    let tmp: u32 = 24;
    let r#ref: u32 = 32;
    let ref_base: u32 = 32;

    #[rustfmt::skip]
    let code = vec![
        // PC 0
        VecNew { descriptor_id: 0, elem_size: 8, initial_capacity: 3, dst_fp_offset: vec },
        StoreU64 { dst_fp_offset: tmp, val: 100 },
        VecPushBack { vec_fp_offset: vec, elem_fp_offset: tmp, elem_size: 8 },
        StoreU64 { dst_fp_offset: tmp, val: 200 },
        VecPushBack { vec_fp_offset: vec, elem_fp_offset: tmp, elem_size: 8 },
        StoreU64 { dst_fp_offset: tmp, val: 300 },
        VecPushBack { vec_fp_offset: vec, elem_fp_offset: tmp, elem_size: 8 },
        // PC 7: borrow vec[2] (value 300)
        StoreU64 { dst_fp_offset: idx, val: 2 },
        VecBorrow { vec_fp_offset: vec, idx_fp_offset: idx, elem_size: 8, dst_fp_offset: r#ref },
        // PC 9: force GC — vec gets relocated, fat pointer base must be updated
        ForceGC,
        // PC 10: read through the reference — should still get 300
        ReadRef { ref_fp_offset: r#ref, dst_fp_offset: result, size: 8 },
        // PC 11: write 42 through the reference
        StoreU64 { dst_fp_offset: tmp, val: 42 },
        WriteRef { ref_fp_offset: r#ref, src_fp_offset: tmp, size: 8 },
        // PC 13: read vec[2] directly to verify
        VecLoadElem { vec_fp_offset: vec, idx_fp_offset: idx, dst_fp_offset: tmp, elem_size: 8 },
        Return,
    ];

    let mut stack_maps = HashMap::new();
    stack_maps.insert(0, vec![]);
    stack_maps.insert(2, vec![vec]);
    stack_maps.insert(4, vec![vec]);
    stack_maps.insert(6, vec![vec]);
    stack_maps.insert(9, vec![vec, ref_base]);

    let functions = [Function {
        code,
        data_size: 48,
        extended_frame_size: 72,
        stack_maps,
    }];
    let descriptors = vec![ObjectDescriptor::Trivial];
    let mut ctx = InterpreterContext::new(&functions, &descriptors, 0, &[]);
    ctx.run().unwrap();

    assert_eq!(ctx.root_result(), 300, "ReadRef should read 300 from vec[2] after GC");
    let vec_ptr = ctx.root_heap_ptr(8);
    let elem2 = unsafe { read_u64(vec_ptr, VEC_DATA_OFFSET + 2 * 8) };
    assert_eq!(elem2, 42, "WriteRef should have written 42 to vec[2] after GC");
    assert_eq!(ctx.gc_count(), 1, "ForceGC should have run exactly once");
}

// ---------------------------------------------------------------------------
// Test 3: ref_cross_frame — fat pointer passed as callee argument
//
// Main (func 0) creates vector<u64> = [10, 20, 30], borrows element 1,
// writes the fat pointer into the callee's arg slot, and calls func 1.
// The callee forces GC (relocating the vector), then writes 77 through
// the reference. Back in main, we verify vec[1] == 77.
// ---------------------------------------------------------------------------

/// Function 0 (main) frame layout (72 bytes):
///   [fp +  0] : result    (u64)
///   [fp +  8] : vec       (heap pointer)
///   [fp + 16] : idx       (u64)
///   [fp + 24] : tmp       (u64 scratch)
///   [fp + 32] : call-site metadata (24 bytes)
///   [fp + 56] : callee arg: ref (fat pointer, 16 bytes = callee fp+0/+8)
///
/// Function 1 (write_through_ref) data segment (24 bytes):
///   [fp +  0] : ref       (fat pointer: base @ +0, offset @ +8)
///   [fp + 16] : val       (u64)
#[test]
fn ref_cross_frame() {
    use Instruction::*;

    // -- Function 1: write_through_ref(ref) --
    let c_ref: u32 = 0;
    let c_ref_base: u32 = 0;
    let c_val: u32 = 16;

    #[rustfmt::skip]
    let callee_code = vec![
        // PC 0: force GC — must preserve the fat pointer's base
        ForceGC,
        // PC 1: write 77 through the reference
        StoreU64 { dst_fp_offset: c_val, val: 77 },
        WriteRef { ref_fp_offset: c_ref, src_fp_offset: c_val, size: 8 },
        Return,
    ];

    let mut callee_sm = HashMap::new();
    callee_sm.insert(0, vec![c_ref_base]);

    let callee_func = Function {
        code: callee_code,
        data_size: 24,
        extended_frame_size: 48,
        stack_maps: callee_sm,
    };

    // -- Function 0: main --
    let m_result: u32 = 0;
    let m_vec: u32 = 8;
    let m_idx: u32 = 16;
    let m_tmp: u32 = 24;
    let m_call_site: u32 = 32;
    let m_callee_ref: u32 = m_call_site + FRAME_METADATA_SIZE as u32; // 56

    #[rustfmt::skip]
    let main_code = vec![
        // PC 0
        VecNew { descriptor_id: 0, elem_size: 8, initial_capacity: 4, dst_fp_offset: m_vec },
        StoreU64 { dst_fp_offset: m_tmp, val: 10 },
        VecPushBack { vec_fp_offset: m_vec, elem_fp_offset: m_tmp, elem_size: 8 },
        StoreU64 { dst_fp_offset: m_tmp, val: 20 },
        VecPushBack { vec_fp_offset: m_vec, elem_fp_offset: m_tmp, elem_size: 8 },
        StoreU64 { dst_fp_offset: m_tmp, val: 30 },
        VecPushBack { vec_fp_offset: m_vec, elem_fp_offset: m_tmp, elem_size: 8 },
        // PC 7: borrow element 1 → write fat pointer into callee's arg slot
        StoreU64 { dst_fp_offset: m_idx, val: 1 },
        VecBorrow { vec_fp_offset: m_vec, idx_fp_offset: m_idx, elem_size: 8, dst_fp_offset: m_callee_ref },
        // PC 9: call func 1
        CallFunc { func_id: 1 },
        // PC 10 (return site): read vec[1] to verify the write
        VecLoadElem { vec_fp_offset: m_vec, idx_fp_offset: m_idx, dst_fp_offset: m_result, elem_size: 8 },
        Return,
    ];

    let mut main_sm = HashMap::new();
    main_sm.insert(0, vec![]);
    main_sm.insert(2, vec![m_vec]);
    main_sm.insert(4, vec![m_vec]);
    main_sm.insert(6, vec![m_vec]);
    main_sm.insert(10, vec![m_vec, m_callee_ref]);

    let main_func = Function {
        code: main_code,
        data_size: 32,
        extended_frame_size: 72,
        stack_maps: main_sm,
    };

    let descriptors = vec![ObjectDescriptor::Trivial];
    let functions = [main_func, callee_func];
    let mut ctx = InterpreterContext::new(&functions, &descriptors, 0, &[]);
    ctx.run().unwrap();

    assert_eq!(ctx.root_result(), 77, "callee should have written 77 through the ref to vec[1]");
    let vec_ptr = ctx.root_heap_ptr(8);
    let elem0 = unsafe { read_u64(vec_ptr, VEC_DATA_OFFSET) };
    let elem1 = unsafe { read_u64(vec_ptr, VEC_DATA_OFFSET + 8) };
    let elem2 = unsafe { read_u64(vec_ptr, VEC_DATA_OFFSET + 16) };
    assert_eq!(elem0, 10, "vec[0] should be untouched");
    assert_eq!(elem1, 77, "vec[1] should be 77 after WriteRef");
    assert_eq!(elem2, 30, "vec[2] should be untouched");
    assert_eq!(ctx.gc_count(), 1, "ForceGC should have run exactly once");
}

// ---------------------------------------------------------------------------
// Test 4: ref_multiple_borrows — two fat pointers into the same vector
//
// Creates vec = [10, 20, 30, 40], borrows element 1 (ref_a) and element 3
// (ref_b). Forces GC, then reads and writes through both. Both fat pointers
// share the same base (the vector), which exercises forwarding-pointer
// deduplication during GC.
// ---------------------------------------------------------------------------

/// Data segment (72 bytes):
///   [fp +  0] : result (u64)
///   [fp +  8] : vec    (heap pointer)
///   [fp + 16] : idx    (u64)
///   [fp + 24] : tmp    (u64)
///   [fp + 32] : ref_a  (fat pointer: base @ +32, offset @ +40)
///   [fp + 48] : ref_b  (fat pointer: base @ +48, offset @ +56)
///   [fp + 64] : val    (u64)
#[test]
fn ref_multiple_borrows() {
    use Instruction::*;

    let result: u32 = 0;
    let vec: u32 = 8;
    let idx: u32 = 16;
    let tmp: u32 = 24;
    let ref_a: u32 = 32;
    let ref_a_base: u32 = 32;
    let ref_b: u32 = 48;
    let ref_b_base: u32 = 48;
    let val: u32 = 64;

    #[rustfmt::skip]
    let code = vec![
        // PC 0: vec = [10, 20, 30, 40]
        VecNew { descriptor_id: 0, elem_size: 8, initial_capacity: 4, dst_fp_offset: vec },
        StoreU64 { dst_fp_offset: tmp, val: 10 },
        VecPushBack { vec_fp_offset: vec, elem_fp_offset: tmp, elem_size: 8 },
        StoreU64 { dst_fp_offset: tmp, val: 20 },
        VecPushBack { vec_fp_offset: vec, elem_fp_offset: tmp, elem_size: 8 },
        StoreU64 { dst_fp_offset: tmp, val: 30 },
        VecPushBack { vec_fp_offset: vec, elem_fp_offset: tmp, elem_size: 8 },
        StoreU64 { dst_fp_offset: tmp, val: 40 },
        VecPushBack { vec_fp_offset: vec, elem_fp_offset: tmp, elem_size: 8 },
        // PC 9: borrow vec[1] and vec[3]
        StoreU64 { dst_fp_offset: idx, val: 1 },
        VecBorrow { vec_fp_offset: vec, idx_fp_offset: idx, elem_size: 8, dst_fp_offset: ref_a },
        StoreU64 { dst_fp_offset: idx, val: 3 },
        VecBorrow { vec_fp_offset: vec, idx_fp_offset: idx, elem_size: 8, dst_fp_offset: ref_b },
        // PC 13: ForceGC — both bases point to the same object
        ForceGC,
        // PC 14: read ref_a → expect 20
        ReadRef { ref_fp_offset: ref_a, dst_fp_offset: result, size: 8 },
        // PC 15-18: write 55 through ref_a, 66 through ref_b
        StoreU64 { dst_fp_offset: val, val: 55 },
        WriteRef { ref_fp_offset: ref_a, src_fp_offset: val, size: 8 },
        StoreU64 { dst_fp_offset: val, val: 66 },
        WriteRef { ref_fp_offset: ref_b, src_fp_offset: val, size: 8 },
        Return,
    ];

    let mut stack_maps = HashMap::new();
    stack_maps.insert(0, vec![]);
    stack_maps.insert(2, vec![vec]);
    stack_maps.insert(4, vec![vec]);
    stack_maps.insert(6, vec![vec]);
    stack_maps.insert(8, vec![vec]);
    stack_maps.insert(13, vec![vec, ref_a_base, ref_b_base]);

    let functions = [Function {
        code,
        data_size: 72,
        extended_frame_size: 96,
        stack_maps,
    }];
    let descriptors = vec![ObjectDescriptor::Trivial];
    let mut ctx = InterpreterContext::new(&functions, &descriptors, 0, &[]);
    ctx.run().unwrap();

    assert_eq!(ctx.root_result(), 20, "ReadRef through ref_a should yield 20 after GC");
    let vec_ptr = ctx.root_heap_ptr(8);
    let elem0 = unsafe { read_u64(vec_ptr, VEC_DATA_OFFSET) };
    let elem1 = unsafe { read_u64(vec_ptr, VEC_DATA_OFFSET + 8) };
    let elem2 = unsafe { read_u64(vec_ptr, VEC_DATA_OFFSET + 16) };
    let elem3 = unsafe { read_u64(vec_ptr, VEC_DATA_OFFSET + 24) };
    assert_eq!(elem0, 10, "vec[0] should be untouched");
    assert_eq!(elem1, 55, "vec[1] should be 55 (written through ref_a)");
    assert_eq!(elem2, 30, "vec[2] should be untouched");
    assert_eq!(elem3, 66, "vec[3] should be 66 (written through ref_b)");
    assert_eq!(ctx.gc_count(), 1);
}

// ---------------------------------------------------------------------------
// Test 5: ref_borrow_local — stack-based fat pointer survives GC
//
// Borrows a stack local (u64) via BorrowLocal. Also allocates a heap vector
// so GC has real work to do. Forces GC, then reads and writes through the
// stack reference. Verifies that is_heap_ptr correctly skips the stack
// address and the reference remains valid.
// ---------------------------------------------------------------------------

/// Data segment (48 bytes):
///   [fp +  0] : result    (u64)
///   [fp +  8] : local_val (u64 — the stack value we borrow)
///   [fp + 16] : ref       (fat pointer: base @ +16, offset @ +24)
///   [fp + 32] : vec       (heap pointer — gives GC actual work)
///   [fp + 40] : tmp       (u64)
#[test]
fn ref_borrow_local() {
    use Instruction::*;

    let result: u32 = 0;
    let local_val: u32 = 8;
    let r#ref: u32 = 16;
    let ref_base: u32 = 16;
    let vec: u32 = 32;
    let tmp: u32 = 40;

    #[rustfmt::skip]
    let code = vec![
        // PC 0: store 42, then borrow the stack local
        StoreU64 { dst_fp_offset: local_val, val: 42 },
        BorrowLocal { local_fp_offset: local_val, dst_fp_offset: r#ref },
        // PC 2: allocate a heap vector so GC has something to relocate
        VecNew { descriptor_id: 0, elem_size: 8, initial_capacity: 4, dst_fp_offset: vec },
        // PC 3: ForceGC — vec is relocated, stack ref base must be untouched
        ForceGC,
        // PC 4: read through the stack ref — should get 42
        ReadRef { ref_fp_offset: r#ref, dst_fp_offset: result, size: 8 },
        // PC 5-6: write 99 through the stack ref
        StoreU64 { dst_fp_offset: tmp, val: 99 },
        WriteRef { ref_fp_offset: r#ref, src_fp_offset: tmp, size: 8 },
        // PC 7: re-read through the ref to confirm the write landed
        ReadRef { ref_fp_offset: r#ref, dst_fp_offset: result, size: 8 },
        Return,
    ];

    let mut stack_maps = HashMap::new();
    stack_maps.insert(2, vec![]);
    // ref_base holds a stack address → is_heap_ptr returns false, GC skips it.
    // vec is a genuine heap root.
    stack_maps.insert(3, vec![vec, ref_base]);

    let functions = [Function {
        code,
        data_size: 48,
        extended_frame_size: 72,
        stack_maps,
    }];
    let descriptors = vec![ObjectDescriptor::Trivial];
    let mut ctx = InterpreterContext::new(&functions, &descriptors, 0, &[]);
    ctx.run().unwrap();

    assert_eq!(ctx.root_result(), 99, "WriteRef through stack ref should have written 99");
    assert_eq!(ctx.gc_count(), 1);
}

// ---------------------------------------------------------------------------
// Test 6: ref_nested_vectors — descriptor-based transitive GC tracing
//
// Builds a vector<vector<u64>> (outer vector with descriptor_id 1 whose
// elements are heap pointers to inner vectors with descriptor_id 0).
// Borrows an element from inner1, forces GC, then verifies the read/write
// through the reference. Also checks that inner0 (reachable only through
// outer's descriptor-driven element tracing, not from any stack root)
// survived GC correctly.
// ---------------------------------------------------------------------------

/// Data segment (64 bytes):
///   [fp +  0] : result    (u64)
///   [fp +  8] : outer     (heap pointer — vector<ptr>, descriptor_id 1)
///   [fp + 16] : idx       (u64)
///   [fp + 24] : tmp       (u64)
///   [fp + 32] : inner_ptr (heap pointer — reused during construction, NOT a GC root)
///   [fp + 40] : ref       (fat pointer: base @ +40, offset @ +48)
///   [fp + 56] : val       (u64)
#[test]
fn ref_nested_vectors() {
    use Instruction::*;

    let result: u32 = 0;
    let outer: u32 = 8;
    let idx: u32 = 16;
    let tmp: u32 = 24;
    let inner_ptr: u32 = 32;
    let r#ref: u32 = 40;
    let ref_base: u32 = 40;
    let val: u32 = 56;

    #[rustfmt::skip]
    let code = vec![
        // -- Build inner0 = [100, 200] --
        // PC 0
        VecNew { descriptor_id: 0, elem_size: 8, initial_capacity: 4, dst_fp_offset: inner_ptr },
        StoreU64 { dst_fp_offset: tmp, val: 100 },
        VecPushBack { vec_fp_offset: inner_ptr, elem_fp_offset: tmp, elem_size: 8 },
        StoreU64 { dst_fp_offset: tmp, val: 200 },
        VecPushBack { vec_fp_offset: inner_ptr, elem_fp_offset: tmp, elem_size: 8 },
        // -- Build outer (descriptor 1: elements are heap pointers) --
        // PC 5
        VecNew { descriptor_id: 1, elem_size: 8, initial_capacity: 4, dst_fp_offset: outer },
        VecPushBack { vec_fp_offset: outer, elem_fp_offset: inner_ptr, elem_size: 8 },
        // -- Build inner1 = [300, 400, 500] (reuses inner_ptr slot) --
        // PC 7
        VecNew { descriptor_id: 0, elem_size: 8, initial_capacity: 4, dst_fp_offset: inner_ptr },
        StoreU64 { dst_fp_offset: tmp, val: 300 },
        VecPushBack { vec_fp_offset: inner_ptr, elem_fp_offset: tmp, elem_size: 8 },
        StoreU64 { dst_fp_offset: tmp, val: 400 },
        VecPushBack { vec_fp_offset: inner_ptr, elem_fp_offset: tmp, elem_size: 8 },
        StoreU64 { dst_fp_offset: tmp, val: 500 },
        VecPushBack { vec_fp_offset: inner_ptr, elem_fp_offset: tmp, elem_size: 8 },
        VecPushBack { vec_fp_offset: outer, elem_fp_offset: inner_ptr, elem_size: 8 },
        // -- Load outer[1] → inner1 ptr, borrow inner1[2] --
        // PC 15
        StoreU64 { dst_fp_offset: idx, val: 1 },
        VecLoadElem { vec_fp_offset: outer, idx_fp_offset: idx, dst_fp_offset: inner_ptr, elem_size: 8 },
        StoreU64 { dst_fp_offset: idx, val: 2 },
        VecBorrow { vec_fp_offset: inner_ptr, idx_fp_offset: idx, elem_size: 8, dst_fp_offset: r#ref },
        // PC 19: ForceGC — outer is a root; GC traces its elements (descriptor 1)
        // to reach and relocate inner0 and inner1. ref_base is also updated.
        ForceGC,
        // PC 20: read through ref → expect 500
        ReadRef { ref_fp_offset: r#ref, dst_fp_offset: result, size: 8 },
        // PC 21-22: write 999 through ref
        StoreU64 { dst_fp_offset: val, val: 999 },
        WriteRef { ref_fp_offset: r#ref, src_fp_offset: val, size: 8 },
        Return,
    ];

    let mut stack_maps = HashMap::new();
    stack_maps.insert(0, vec![]);
    stack_maps.insert(2, vec![inner_ptr]);
    stack_maps.insert(4, vec![inner_ptr]);
    stack_maps.insert(5, vec![inner_ptr]);
    stack_maps.insert(6, vec![inner_ptr, outer]);
    stack_maps.insert(7, vec![outer]);
    stack_maps.insert(9, vec![outer, inner_ptr]);
    stack_maps.insert(11, vec![outer, inner_ptr]);
    stack_maps.insert(13, vec![outer, inner_ptr]);
    stack_maps.insert(14, vec![outer, inner_ptr]);
    // inner_ptr is NOT listed at PC 19 — it's stale after the VecBorrow.
    // Only outer (root of the object graph) and ref_base are live.
    stack_maps.insert(19, vec![outer, ref_base]);

    let functions = [Function {
        code,
        data_size: 64,
        extended_frame_size: 88,
        stack_maps,
    }];
    let descriptors = vec![
        ObjectDescriptor::Trivial,
        ObjectDescriptor::Vector {
            elem_size: 8,
            elem_ref_offsets: vec![0],
        },
    ];
    let mut ctx = InterpreterContext::new(&functions, &descriptors, 0, &[]);
    ctx.run().unwrap();

    assert_eq!(ctx.root_result(), 500, "ReadRef should yield 500 from inner1[2] after GC");

    let outer_ptr = ctx.root_heap_ptr(8);

    // Verify inner1[2] = 999 (written through the fat pointer after GC)
    let inner1_ptr = unsafe { read_ptr(outer_ptr, VEC_DATA_OFFSET + 8) };
    let inner1_0 = unsafe { read_u64(inner1_ptr, VEC_DATA_OFFSET) };
    let inner1_1 = unsafe { read_u64(inner1_ptr, VEC_DATA_OFFSET + 8) };
    let inner1_2 = unsafe { read_u64(inner1_ptr, VEC_DATA_OFFSET + 16) };
    assert_eq!(inner1_0, 300, "inner1[0] should be untouched");
    assert_eq!(inner1_1, 400, "inner1[1] should be untouched");
    assert_eq!(inner1_2, 999, "inner1[2] should be 999 after WriteRef");

    // Verify inner0 survived — it's only reachable through outer's descriptor
    // tracing, not from any stack root directly.
    let inner0_ptr = unsafe { read_ptr(outer_ptr, VEC_DATA_OFFSET) };
    let inner0_0 = unsafe { read_u64(inner0_ptr, VEC_DATA_OFFSET) };
    let inner0_1 = unsafe { read_u64(inner0_ptr, VEC_DATA_OFFSET + 8) };
    assert_eq!(inner0_0, 100, "inner0[0] should survive GC via transitive tracing");
    assert_eq!(inner0_1, 200, "inner0[1] should survive GC via transitive tracing");

    assert_eq!(ctx.gc_count(), 1);
}

// ---------------------------------------------------------------------------
// Test 7: ref_survives_double_gc — fat pointer survives two GC cycles
//
// Borrows a vector element, then forces GC twice in a row. The vector is
// relocated from from-space to to-space on each cycle. After both cycles,
// reads and writes through the reference to verify the base was updated
// correctly both times.
// ---------------------------------------------------------------------------

/// Data segment (48 bytes):
///   [fp +  0] : result (u64)
///   [fp +  8] : vec    (heap pointer)
///   [fp + 16] : idx    (u64)
///   [fp + 24] : tmp    (u64)
///   [fp + 32] : ref    (fat pointer: base @ +32, offset @ +40)
#[test]
fn ref_survives_double_gc() {
    use Instruction::*;

    let result: u32 = 0;
    let vec: u32 = 8;
    let idx: u32 = 16;
    let tmp: u32 = 24;
    let r#ref: u32 = 32;
    let ref_base: u32 = 32;

    #[rustfmt::skip]
    let code = vec![
        // PC 0: vec = [10, 20, 30]
        VecNew { descriptor_id: 0, elem_size: 8, initial_capacity: 3, dst_fp_offset: vec },
        StoreU64 { dst_fp_offset: tmp, val: 10 },
        VecPushBack { vec_fp_offset: vec, elem_fp_offset: tmp, elem_size: 8 },
        StoreU64 { dst_fp_offset: tmp, val: 20 },
        VecPushBack { vec_fp_offset: vec, elem_fp_offset: tmp, elem_size: 8 },
        StoreU64 { dst_fp_offset: tmp, val: 30 },
        VecPushBack { vec_fp_offset: vec, elem_fp_offset: tmp, elem_size: 8 },
        // PC 7: borrow vec[1] (value 20)
        StoreU64 { dst_fp_offset: idx, val: 1 },
        VecBorrow { vec_fp_offset: vec, idx_fp_offset: idx, elem_size: 8, dst_fp_offset: r#ref },
        // PC 9-10: two consecutive GC cycles
        ForceGC,
        ForceGC,
        // PC 11: read/write through the twice-relocated reference
        ReadRef { ref_fp_offset: r#ref, dst_fp_offset: result, size: 8 },
        StoreU64 { dst_fp_offset: tmp, val: 77 },
        WriteRef { ref_fp_offset: r#ref, src_fp_offset: tmp, size: 8 },
        Return,
    ];

    let mut stack_maps = HashMap::new();
    stack_maps.insert(0, vec![]);
    stack_maps.insert(2, vec![vec]);
    stack_maps.insert(4, vec![vec]);
    stack_maps.insert(6, vec![vec]);
    stack_maps.insert(9, vec![vec, ref_base]);
    stack_maps.insert(10, vec![vec, ref_base]);

    let functions = [Function {
        code,
        data_size: 48,
        extended_frame_size: 72,
        stack_maps,
    }];
    let descriptors = vec![ObjectDescriptor::Trivial];
    let mut ctx = InterpreterContext::new(&functions, &descriptors, 0, &[]);
    ctx.run().unwrap();

    assert_eq!(ctx.root_result(), 20, "ReadRef should yield 20 after two GC cycles");
    let vec_ptr = ctx.root_heap_ptr(8);
    let elem1 = unsafe { read_u64(vec_ptr, VEC_DATA_OFFSET + 8) };
    assert_eq!(elem1, 77, "WriteRef should have written 77 to vec[1] after double GC");
    assert_eq!(ctx.gc_count(), 2, "ForceGC should have run exactly twice");
}

// ---------------------------------------------------------------------------
// Test 8: ref_struct_field_borrow — borrow a field of a heap struct
//
// Allocate Entry { key: 42, value: 100 }, borrow the value field via
// StructBorrow, read through the reference (expect 100), write 55 through
// it, then load the field directly to confirm the write landed.
// ---------------------------------------------------------------------------

/// Data segment (32 bytes):
///   [fp +  0] : result  (u64)
///   [fp +  8] : entry   (heap pointer)
///   [fp + 16] : ref     (fat pointer: base @ +16, offset @ +24)
#[test]
fn ref_struct_field_borrow() {
    use Instruction::*;

    let result: u32 = 0;
    let entry: u32 = 8;
    let r#ref: u32 = 16;

    #[rustfmt::skip]
    let code = vec![
        // PC 0: allocate Entry
        StructNew { descriptor_id: 0, dst_fp_offset: entry },
        // PC 1-2: entry.key = 42
        StoreU64 { dst_fp_offset: result, val: 42 },
        StructStoreField { struct_fp_offset: entry, field_offset: 0, src_fp_offset: result, size: 8 },
        // PC 3-4: entry.value = 100
        StoreU64 { dst_fp_offset: result, val: 100 },
        StructStoreField { struct_fp_offset: entry, field_offset: 8, src_fp_offset: result, size: 8 },
        // PC 5: borrow entry.value
        StructBorrow { struct_fp_offset: entry, field_offset: 8, dst_fp_offset: r#ref },
        // PC 6: read through ref → expect 100
        ReadRef { ref_fp_offset: r#ref, dst_fp_offset: result, size: 8 },
        // PC 7-8: write 55 through ref
        StoreU64 { dst_fp_offset: result, val: 55 },
        WriteRef { ref_fp_offset: r#ref, src_fp_offset: result, size: 8 },
        // PC 9: read entry.value directly to verify
        StructLoadField { struct_fp_offset: entry, field_offset: 8, dst_fp_offset: result, size: 8 },
        Return,
    ];

    let mut stack_maps = HashMap::new();
    stack_maps.insert(0, vec![]);

    let functions = [Function {
        code,
        data_size: 32,
        extended_frame_size: 56,
        stack_maps,
    }];
    let descriptors = vec![ObjectDescriptor::Struct {
        size: 16,
        ref_offsets: vec![],
    }];
    let mut ctx = InterpreterContext::new(&functions, &descriptors, 0, &[]);
    ctx.run().unwrap();

    assert_eq!(ctx.root_result(), 55, "entry.value should be 55 after WriteRef through StructBorrow");
    let entry_ptr = ctx.root_heap_ptr(8);
    let key = unsafe { read_u64(entry_ptr, STRUCT_DATA_OFFSET) };
    assert_eq!(key, 42, "entry.key should be untouched");
}

// ---------------------------------------------------------------------------
// Test 9: ref_struct_field_survives_gc — borrowed struct field survives GC
//
// Allocate Entry { key: 7, value: 13 }, borrow value field, force GC,
// then read through the reference. The struct is relocated during GC;
// the fat pointer's base must be updated correctly.
// ---------------------------------------------------------------------------

/// Data segment (32 bytes):
///   [fp +  0] : result  (u64)
///   [fp +  8] : entry   (heap pointer)
///   [fp + 16] : ref     (fat pointer: base @ +16, offset @ +24)
#[test]
fn ref_struct_field_survives_gc() {
    use Instruction::*;

    let result: u32 = 0;
    let entry: u32 = 8;
    let r#ref: u32 = 16;
    let ref_base: u32 = 16;

    #[rustfmt::skip]
    let code = vec![
        // PC 0: allocate Entry
        StructNew { descriptor_id: 0, dst_fp_offset: entry },
        // PC 1-2: entry.key = 7
        StoreU64 { dst_fp_offset: result, val: 7 },
        StructStoreField { struct_fp_offset: entry, field_offset: 0, src_fp_offset: result, size: 8 },
        // PC 3-4: entry.value = 13
        StoreU64 { dst_fp_offset: result, val: 13 },
        StructStoreField { struct_fp_offset: entry, field_offset: 8, src_fp_offset: result, size: 8 },
        // PC 5: borrow entry.value
        StructBorrow { struct_fp_offset: entry, field_offset: 8, dst_fp_offset: r#ref },
        // PC 6: force GC — entry gets relocated, fat pointer base must be updated
        ForceGC,
        // PC 7: read through ref → should still be 13
        ReadRef { ref_fp_offset: r#ref, dst_fp_offset: result, size: 8 },
        Return,
    ];

    let mut stack_maps = HashMap::new();
    stack_maps.insert(0, vec![]);
    stack_maps.insert(6, vec![entry, ref_base]);

    let functions = [Function {
        code,
        data_size: 32,
        extended_frame_size: 56,
        stack_maps,
    }];
    let descriptors = vec![ObjectDescriptor::Struct {
        size: 16,
        ref_offsets: vec![],
    }];
    let mut ctx = InterpreterContext::new(&functions, &descriptors, 0, &[]);
    ctx.run().unwrap();

    assert_eq!(ctx.root_result(), 13, "entry.value should be 13 after GC");
    assert_eq!(ctx.gc_count(), 1);

    let entry_ptr = ctx.root_heap_ptr(8);
    let key = unsafe { read_u64(entry_ptr, STRUCT_DATA_OFFSET) };
    assert_eq!(key, 7, "entry.key should survive GC");
}
