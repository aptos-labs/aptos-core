// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for fat-pointer references (VecBorrow, SlotBorrow, ReadRef, WriteRef,
//! HeapBorrow).

use mono_move_runtime::{
    read_ptr, read_u64, DescriptorId, FrameOffset as FO, Function, InterpreterContext, MicroOp,
    ObjectDescriptor, FRAME_METADATA_SIZE, STRUCT_DATA_OFFSET, VEC_DATA_OFFSET,
};

// ---------------------------------------------------------------------------
// Test 1: ref_basic — single-frame, no GC
// ---------------------------------------------------------------------------

#[test]
fn ref_basic() {
    use MicroOp::*;

    let result: u32 = 0;
    let vec: u32 = 8;
    let idx: u32 = 16;
    let tmp: u32 = 24;
    let r#ref: u32 = 32;
    let val: u32 = 48;

    #[rustfmt::skip]
    let code = vec![
        VecNew { dst: FO(vec), descriptor_id: DescriptorId(0), elem_size: 8, initial_capacity: 4 },
        StoreImm8 { dst: FO(tmp), imm: 10 },
        VecPushBack { heap_ptr: FO(vec), elem: FO(tmp), elem_size: 8 },
        StoreImm8 { dst: FO(tmp), imm: 20 },
        VecPushBack { heap_ptr: FO(vec), elem: FO(tmp), elem_size: 8 },
        StoreImm8 { dst: FO(tmp), imm: 30 },
        VecPushBack { heap_ptr: FO(vec), elem: FO(tmp), elem_size: 8 },
        StoreImm8 { dst: FO(idx), imm: 1 },
        VecBorrow { dst: FO(r#ref), heap_ptr: FO(vec), idx: FO(idx), elem_size: 8 },
        ReadRef { dst: FO(result), ref_ptr: FO(r#ref), size: 8 },
        StoreImm8 { dst: FO(val), imm: 99 },
        WriteRef { ref_ptr: FO(r#ref), src: FO(val), size: 8 },
        VecLoadElem { dst: FO(tmp), heap_ptr: FO(vec), idx: FO(idx), elem_size: 8 },
        Return,
    ];

    let functions = [Function {
        code,
        args_size: 0,
        data_size: 56,
        extended_frame_size: 80,
        zero_locals: true,
        pointer_slots: vec![FO(vec), FO(r#ref)],
    }];
    let descriptors = vec![ObjectDescriptor::Trivial];
    let mut ctx = InterpreterContext::new(&functions, &descriptors, 0);
    ctx.run().unwrap();

    assert_eq!(
        ctx.root_result(),
        20,
        "ReadRef should have read 20 from vec[1]"
    );
    let vec_ptr = ctx.root_heap_ptr(8);
    let elem1 = unsafe { read_u64(vec_ptr, VEC_DATA_OFFSET + 8) };
    assert_eq!(elem1, 99, "WriteRef should have written 99 to vec[1]");
}

// ---------------------------------------------------------------------------
// Test 2: ref_survives_gc
// ---------------------------------------------------------------------------

#[test]
fn ref_survives_gc() {
    use MicroOp::*;

    let result: u32 = 0;
    let vec: u32 = 8;
    let idx: u32 = 16;
    let tmp: u32 = 24;
    let r#ref: u32 = 32;
    let ref_base: u32 = 32;

    #[rustfmt::skip]
    let code = vec![
        VecNew { dst: FO(vec), descriptor_id: DescriptorId(0), elem_size: 8, initial_capacity: 3 },
        StoreImm8 { dst: FO(tmp), imm: 100 },
        VecPushBack { heap_ptr: FO(vec), elem: FO(tmp), elem_size: 8 },
        StoreImm8 { dst: FO(tmp), imm: 200 },
        VecPushBack { heap_ptr: FO(vec), elem: FO(tmp), elem_size: 8 },
        StoreImm8 { dst: FO(tmp), imm: 300 },
        VecPushBack { heap_ptr: FO(vec), elem: FO(tmp), elem_size: 8 },
        StoreImm8 { dst: FO(idx), imm: 2 },
        VecBorrow { dst: FO(r#ref), heap_ptr: FO(vec), idx: FO(idx), elem_size: 8 },
        ForceGC,
        ReadRef { dst: FO(result), ref_ptr: FO(r#ref), size: 8 },
        StoreImm8 { dst: FO(tmp), imm: 42 },
        WriteRef { ref_ptr: FO(r#ref), src: FO(tmp), size: 8 },
        VecLoadElem { dst: FO(tmp), heap_ptr: FO(vec), idx: FO(idx), elem_size: 8 },
        Return,
    ];

    let functions = [Function {
        code,
        args_size: 0,
        data_size: 48,
        extended_frame_size: 72,
        zero_locals: true,
        pointer_slots: vec![FO(vec), FO(ref_base)],
    }];
    let descriptors = vec![ObjectDescriptor::Trivial];
    let mut ctx = InterpreterContext::new(&functions, &descriptors, 0);
    ctx.run().unwrap();

    assert_eq!(
        ctx.root_result(),
        300,
        "ReadRef should read 300 from vec[2] after GC"
    );
    let vec_ptr = ctx.root_heap_ptr(8);
    let elem2 = unsafe { read_u64(vec_ptr, VEC_DATA_OFFSET + 2 * 8) };
    assert_eq!(
        elem2, 42,
        "WriteRef should have written 42 to vec[2] after GC"
    );
    assert_eq!(ctx.gc_count(), 1, "ForceGC should have run exactly once");
}

// ---------------------------------------------------------------------------
// Test 3: ref_cross_frame
// ---------------------------------------------------------------------------

#[test]
fn ref_cross_frame() {
    use MicroOp::*;

    // -- Function 1: write_through_ref(ref) --
    let c_ref: u32 = 0;
    let c_ref_base: u32 = 0;
    let c_val: u32 = 16;

    #[rustfmt::skip]
    let callee_code = vec![
        ForceGC,
        StoreImm8 { dst: FO(c_val), imm: 77 },
        WriteRef { ref_ptr: FO(c_ref), src: FO(c_val), size: 8 },
        Return,
    ];

    let callee_func = Function {
        code: callee_code,
        args_size: 16,
        data_size: 24,
        extended_frame_size: 48,
        zero_locals: true,
        pointer_slots: vec![FO(c_ref_base)],
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
        VecNew { dst: FO(m_vec), descriptor_id: DescriptorId(0), elem_size: 8, initial_capacity: 4 },
        StoreImm8 { dst: FO(m_tmp), imm: 10 },
        VecPushBack { heap_ptr: FO(m_vec), elem: FO(m_tmp), elem_size: 8 },
        StoreImm8 { dst: FO(m_tmp), imm: 20 },
        VecPushBack { heap_ptr: FO(m_vec), elem: FO(m_tmp), elem_size: 8 },
        StoreImm8 { dst: FO(m_tmp), imm: 30 },
        VecPushBack { heap_ptr: FO(m_vec), elem: FO(m_tmp), elem_size: 8 },
        StoreImm8 { dst: FO(m_idx), imm: 1 },
        VecBorrow { dst: FO(m_callee_ref), heap_ptr: FO(m_vec), idx: FO(m_idx), elem_size: 8 },
        CallFunc { func_id: 1 },
        VecLoadElem { dst: FO(m_result), heap_ptr: FO(m_vec), idx: FO(m_idx), elem_size: 8 },
        Return,
    ];

    let main_func = Function {
        code: main_code,
        args_size: 0,
        data_size: 32,
        extended_frame_size: 72,
        zero_locals: true,
        pointer_slots: vec![FO(m_vec), FO(m_callee_ref)],
    };

    let descriptors = vec![ObjectDescriptor::Trivial];
    let functions = [main_func, callee_func];
    let mut ctx = InterpreterContext::new(&functions, &descriptors, 0);
    ctx.run().unwrap();

    assert_eq!(
        ctx.root_result(),
        77,
        "callee should have written 77 through the ref to vec[1]"
    );
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
// Test 4: ref_multiple_borrows
// ---------------------------------------------------------------------------

#[test]
fn ref_multiple_borrows() {
    use MicroOp::*;

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
        VecNew { dst: FO(vec), descriptor_id: DescriptorId(0), elem_size: 8, initial_capacity: 4 },
        StoreImm8 { dst: FO(tmp), imm: 10 },
        VecPushBack { heap_ptr: FO(vec), elem: FO(tmp), elem_size: 8 },
        StoreImm8 { dst: FO(tmp), imm: 20 },
        VecPushBack { heap_ptr: FO(vec), elem: FO(tmp), elem_size: 8 },
        StoreImm8 { dst: FO(tmp), imm: 30 },
        VecPushBack { heap_ptr: FO(vec), elem: FO(tmp), elem_size: 8 },
        StoreImm8 { dst: FO(tmp), imm: 40 },
        VecPushBack { heap_ptr: FO(vec), elem: FO(tmp), elem_size: 8 },
        StoreImm8 { dst: FO(idx), imm: 1 },
        VecBorrow { dst: FO(ref_a), heap_ptr: FO(vec), idx: FO(idx), elem_size: 8 },
        StoreImm8 { dst: FO(idx), imm: 3 },
        VecBorrow { dst: FO(ref_b), heap_ptr: FO(vec), idx: FO(idx), elem_size: 8 },
        ForceGC,
        ReadRef { dst: FO(result), ref_ptr: FO(ref_a), size: 8 },
        StoreImm8 { dst: FO(val), imm: 55 },
        WriteRef { ref_ptr: FO(ref_a), src: FO(val), size: 8 },
        StoreImm8 { dst: FO(val), imm: 66 },
        WriteRef { ref_ptr: FO(ref_b), src: FO(val), size: 8 },
        Return,
    ];

    let functions = [Function {
        code,
        args_size: 0,
        data_size: 72,
        extended_frame_size: 96,
        zero_locals: true,
        pointer_slots: vec![FO(vec), FO(ref_a_base), FO(ref_b_base)],
    }];
    let descriptors = vec![ObjectDescriptor::Trivial];
    let mut ctx = InterpreterContext::new(&functions, &descriptors, 0);
    ctx.run().unwrap();

    assert_eq!(
        ctx.root_result(),
        20,
        "ReadRef through ref_a should yield 20 after GC"
    );
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
// Test 5: ref_borrow_local
// ---------------------------------------------------------------------------

#[test]
fn ref_borrow_local() {
    use MicroOp::*;

    let result: u32 = 0;
    let local_val: u32 = 8;
    let r#ref: u32 = 16;
    let ref_base: u32 = 16;
    let vec: u32 = 32;
    let tmp: u32 = 40;

    #[rustfmt::skip]
    let code = vec![
        StoreImm8 { dst: FO(local_val), imm: 42 },
        SlotBorrow { dst: FO(r#ref), local: FO(local_val) },
        VecNew { dst: FO(vec), descriptor_id: DescriptorId(0), elem_size: 8, initial_capacity: 4 },
        ForceGC,
        ReadRef { dst: FO(result), ref_ptr: FO(r#ref), size: 8 },
        StoreImm8 { dst: FO(tmp), imm: 99 },
        WriteRef { ref_ptr: FO(r#ref), src: FO(tmp), size: 8 },
        ReadRef { dst: FO(result), ref_ptr: FO(r#ref), size: 8 },
        Return,
    ];

    let functions = [Function {
        code,
        args_size: 0,
        data_size: 48,
        extended_frame_size: 72,
        // ref_base holds a stack address → is_heap_ptr returns false, GC skips it.
        // vec is a genuine heap root.
        zero_locals: true,
        pointer_slots: vec![FO(ref_base), FO(vec)],
    }];
    let descriptors = vec![ObjectDescriptor::Trivial];
    let mut ctx = InterpreterContext::new(&functions, &descriptors, 0);
    ctx.run().unwrap();

    assert_eq!(
        ctx.root_result(),
        99,
        "WriteRef through stack ref should have written 99"
    );
    assert_eq!(ctx.gc_count(), 1);
}

// ---------------------------------------------------------------------------
// Test 6: ref_nested_vectors
// ---------------------------------------------------------------------------

#[test]
fn ref_nested_vectors() {
    use MicroOp::*;

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
        VecNew { dst: FO(inner_ptr), descriptor_id: DescriptorId(0), elem_size: 8, initial_capacity: 4 },
        StoreImm8 { dst: FO(tmp), imm: 100 },
        VecPushBack { heap_ptr: FO(inner_ptr), elem: FO(tmp), elem_size: 8 },
        StoreImm8 { dst: FO(tmp), imm: 200 },
        VecPushBack { heap_ptr: FO(inner_ptr), elem: FO(tmp), elem_size: 8 },
        // -- Build outer --
        VecNew { dst: FO(outer), descriptor_id: DescriptorId(1), elem_size: 8, initial_capacity: 4 },
        VecPushBack { heap_ptr: FO(outer), elem: FO(inner_ptr), elem_size: 8 },
        // -- Build inner1 = [300, 400, 500] --
        VecNew { dst: FO(inner_ptr), descriptor_id: DescriptorId(0), elem_size: 8, initial_capacity: 4 },
        StoreImm8 { dst: FO(tmp), imm: 300 },
        VecPushBack { heap_ptr: FO(inner_ptr), elem: FO(tmp), elem_size: 8 },
        StoreImm8 { dst: FO(tmp), imm: 400 },
        VecPushBack { heap_ptr: FO(inner_ptr), elem: FO(tmp), elem_size: 8 },
        StoreImm8 { dst: FO(tmp), imm: 500 },
        VecPushBack { heap_ptr: FO(inner_ptr), elem: FO(tmp), elem_size: 8 },
        VecPushBack { heap_ptr: FO(outer), elem: FO(inner_ptr), elem_size: 8 },
        // -- Load outer[1] → inner1 ptr, borrow inner1[2] --
        StoreImm8 { dst: FO(idx), imm: 1 },
        VecLoadElem { dst: FO(inner_ptr), heap_ptr: FO(outer), idx: FO(idx), elem_size: 8 },
        StoreImm8 { dst: FO(idx), imm: 2 },
        VecBorrow { dst: FO(r#ref), heap_ptr: FO(inner_ptr), idx: FO(idx), elem_size: 8 },
        ForceGC,
        ReadRef { dst: FO(result), ref_ptr: FO(r#ref), size: 8 },
        StoreImm8 { dst: FO(val), imm: 999 },
        WriteRef { ref_ptr: FO(r#ref), src: FO(val), size: 8 },
        Return,
    ];

    let functions = [Function {
        code,
        args_size: 0,
        data_size: 64,
        extended_frame_size: 88,
        zero_locals: true,
        pointer_slots: vec![FO(outer), FO(inner_ptr), FO(ref_base)],
    }];
    let descriptors = vec![ObjectDescriptor::Trivial, ObjectDescriptor::Vector {
        elem_size: 8,
        elem_ref_offsets: vec![0],
    }];
    let mut ctx = InterpreterContext::new(&functions, &descriptors, 0);
    ctx.run().unwrap();

    assert_eq!(
        ctx.root_result(),
        500,
        "ReadRef should yield 500 from inner1[2] after GC"
    );

    let outer_ptr = ctx.root_heap_ptr(8);

    let inner1_ptr = unsafe { read_ptr(outer_ptr, VEC_DATA_OFFSET + 8) };
    let inner1_0 = unsafe { read_u64(inner1_ptr, VEC_DATA_OFFSET) };
    let inner1_1 = unsafe { read_u64(inner1_ptr, VEC_DATA_OFFSET + 8) };
    let inner1_2 = unsafe { read_u64(inner1_ptr, VEC_DATA_OFFSET + 16) };
    assert_eq!(inner1_0, 300, "inner1[0] should be untouched");
    assert_eq!(inner1_1, 400, "inner1[1] should be untouched");
    assert_eq!(inner1_2, 999, "inner1[2] should be 999 after WriteRef");

    let inner0_ptr = unsafe { read_ptr(outer_ptr, VEC_DATA_OFFSET) };
    let inner0_0 = unsafe { read_u64(inner0_ptr, VEC_DATA_OFFSET) };
    let inner0_1 = unsafe { read_u64(inner0_ptr, VEC_DATA_OFFSET + 8) };
    assert_eq!(
        inner0_0, 100,
        "inner0[0] should survive GC via transitive tracing"
    );
    assert_eq!(
        inner0_1, 200,
        "inner0[1] should survive GC via transitive tracing"
    );

    assert_eq!(ctx.gc_count(), 1);
}

// ---------------------------------------------------------------------------
// Test 7: ref_survives_double_gc
// ---------------------------------------------------------------------------

#[test]
fn ref_survives_double_gc() {
    use MicroOp::*;

    let result: u32 = 0;
    let vec: u32 = 8;
    let idx: u32 = 16;
    let tmp: u32 = 24;
    let r#ref: u32 = 32;
    let ref_base: u32 = 32;

    #[rustfmt::skip]
    let code = vec![
        VecNew { dst: FO(vec), descriptor_id: DescriptorId(0), elem_size: 8, initial_capacity: 3 },
        StoreImm8 { dst: FO(tmp), imm: 10 },
        VecPushBack { heap_ptr: FO(vec), elem: FO(tmp), elem_size: 8 },
        StoreImm8 { dst: FO(tmp), imm: 20 },
        VecPushBack { heap_ptr: FO(vec), elem: FO(tmp), elem_size: 8 },
        StoreImm8 { dst: FO(tmp), imm: 30 },
        VecPushBack { heap_ptr: FO(vec), elem: FO(tmp), elem_size: 8 },
        StoreImm8 { dst: FO(idx), imm: 1 },
        VecBorrow { dst: FO(r#ref), heap_ptr: FO(vec), idx: FO(idx), elem_size: 8 },
        ForceGC,
        ForceGC,
        ReadRef { dst: FO(result), ref_ptr: FO(r#ref), size: 8 },
        StoreImm8 { dst: FO(tmp), imm: 77 },
        WriteRef { ref_ptr: FO(r#ref), src: FO(tmp), size: 8 },
        Return,
    ];

    let functions = [Function {
        code,
        args_size: 0,
        data_size: 48,
        extended_frame_size: 72,
        zero_locals: true,
        pointer_slots: vec![FO(vec), FO(ref_base)],
    }];
    let descriptors = vec![ObjectDescriptor::Trivial];
    let mut ctx = InterpreterContext::new(&functions, &descriptors, 0);
    ctx.run().unwrap();

    assert_eq!(
        ctx.root_result(),
        20,
        "ReadRef should yield 20 after two GC cycles"
    );
    let vec_ptr = ctx.root_heap_ptr(8);
    let elem1 = unsafe { read_u64(vec_ptr, VEC_DATA_OFFSET + 8) };
    assert_eq!(
        elem1, 77,
        "WriteRef should have written 77 to vec[1] after double GC"
    );
    assert_eq!(ctx.gc_count(), 2, "ForceGC should have run exactly twice");
}

// ---------------------------------------------------------------------------
// Test 8: ref_struct_field_borrow
// ---------------------------------------------------------------------------

#[test]
fn ref_struct_field_borrow() {
    use MicroOp::*;

    let result: u32 = 0;
    let entry: u32 = 8;
    let r#ref: u32 = 16;

    #[rustfmt::skip]
    let code = vec![
        HeapNew { dst: FO(entry), descriptor_id: DescriptorId(0) },
        StoreImm8 { dst: FO(result), imm: 42 },
        MicroOp::struct_store8(FO(entry), 0, FO(result)),
        StoreImm8 { dst: FO(result), imm: 100 },
        MicroOp::struct_store8(FO(entry), 8, FO(result)),
        MicroOp::struct_borrow(FO(entry), 8, FO(r#ref)),
        ReadRef { dst: FO(result), ref_ptr: FO(r#ref), size: 8 },
        StoreImm8 { dst: FO(result), imm: 55 },
        WriteRef { ref_ptr: FO(r#ref), src: FO(result), size: 8 },
        MicroOp::struct_load8(FO(entry), 8, FO(result)),
        Return,
    ];

    let functions = [Function {
        code,
        args_size: 0,
        data_size: 32,
        extended_frame_size: 56,
        zero_locals: true,
        pointer_slots: vec![FO(entry), FO(r#ref)],
    }];
    let descriptors = vec![ObjectDescriptor::Struct {
        size: 16,
        ref_offsets: vec![],
    }];
    let mut ctx = InterpreterContext::new(&functions, &descriptors, 0);
    ctx.run().unwrap();

    assert_eq!(
        ctx.root_result(),
        55,
        "entry.value should be 55 after WriteRef through StructBorrow"
    );
    let entry_ptr = ctx.root_heap_ptr(8);
    let key = unsafe { read_u64(entry_ptr, STRUCT_DATA_OFFSET) };
    assert_eq!(key, 42, "entry.key should be untouched");
}

// ---------------------------------------------------------------------------
// Test 9: ref_struct_field_survives_gc
// ---------------------------------------------------------------------------

#[test]
fn ref_struct_field_survives_gc() {
    use MicroOp::*;

    let result: u32 = 0;
    let entry: u32 = 8;
    let r#ref: u32 = 16;
    let ref_base: u32 = 16;

    #[rustfmt::skip]
    let code = vec![
        HeapNew { dst: FO(entry), descriptor_id: DescriptorId(0) },
        StoreImm8 { dst: FO(result), imm: 7 },
        MicroOp::struct_store8(FO(entry), 0, FO(result)),
        StoreImm8 { dst: FO(result), imm: 13 },
        MicroOp::struct_store8(FO(entry), 8, FO(result)),
        MicroOp::struct_borrow(FO(entry), 8, FO(r#ref)),
        ForceGC,
        ReadRef { dst: FO(result), ref_ptr: FO(r#ref), size: 8 },
        Return,
    ];

    let functions = [Function {
        code,
        args_size: 0,
        data_size: 32,
        extended_frame_size: 56,
        zero_locals: true,
        pointer_slots: vec![FO(entry), FO(ref_base)],
    }];
    let descriptors = vec![ObjectDescriptor::Struct {
        size: 16,
        ref_offsets: vec![],
    }];
    let mut ctx = InterpreterContext::new(&functions, &descriptors, 0);
    ctx.run().unwrap();

    assert_eq!(ctx.root_result(), 13, "entry.value should be 13 after GC");
    assert_eq!(ctx.gc_count(), 1);

    let entry_ptr = ctx.root_heap_ptr(8);
    let key = unsafe { read_u64(entry_ptr, STRUCT_DATA_OFFSET) };
    assert_eq!(key, 7, "entry.key should survive GC");
}
