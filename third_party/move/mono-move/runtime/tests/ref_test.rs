// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for fat-pointer references (VecBorrow, SlotBorrow, ReadRef, WriteRef,
//! HeapBorrow).

use mono_move_alloc::{ExecutableArena, ExecutableArenaPtr, GlobalArenaPtr};
use mono_move_core::{
    DescriptorId, FrameLayoutInfo, FrameOffset as FO, Function, MicroOp, NoopTransactionContext,
    SortedSafePointEntries, FRAME_METADATA_SIZE, STRUCT_DATA_OFFSET,
};
use mono_move_gas::SimpleGasMeter;
use mono_move_runtime::{
    read_ptr, read_u64, InterpreterContext, ObjectDescriptor, VEC_DATA_OFFSET,
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
    let vec_ref: u32 = 56;

    let arena = ExecutableArena::new();

    #[rustfmt::skip]
    let code = arena.alloc_slice_fill_iter([
        VecNew { dst: FO(vec) },
        SlotBorrow { dst: FO(vec_ref), local: FO(vec) },
        StoreImm8 { dst: FO(tmp), imm: 10 },
        VecPushBack { vec_ref: FO(vec_ref), elem: FO(tmp), elem_size: 8, descriptor_id: DescriptorId(0) },
        StoreImm8 { dst: FO(tmp), imm: 20 },
        VecPushBack { vec_ref: FO(vec_ref), elem: FO(tmp), elem_size: 8, descriptor_id: DescriptorId(0) },
        StoreImm8 { dst: FO(tmp), imm: 30 },
        VecPushBack { vec_ref: FO(vec_ref), elem: FO(tmp), elem_size: 8, descriptor_id: DescriptorId(0) },
        StoreImm8 { dst: FO(idx), imm: 1 },
        VecBorrow { dst: FO(r#ref), vec_ref: FO(vec_ref), idx: FO(idx), elem_size: 8 },
        ReadRef { dst: FO(result), ref_ptr: FO(r#ref), size: 8 },
        StoreImm8 { dst: FO(val), imm: 99 },
        WriteRef { ref_ptr: FO(r#ref), src: FO(val), size: 8 },
        VecLoadElem { dst: FO(tmp), vec_ref: FO(vec_ref), idx: FO(idx), elem_size: 8 },
        Return,
    ]);
    let functions = [arena.alloc(Function {
        name: GlobalArenaPtr::from_static("test"),
        code,
        param_sizes: ExecutableArenaPtr::empty_slice(),
        param_sizes_sum: 0,
        param_and_local_sizes_sum: 72,
        extended_frame_size: 96,
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(&arena, [FO(vec), FO(r#ref), FO(vec_ref)]),
        safe_point_layouts: SortedSafePointEntries::empty(),
    })];
    let descriptors = [ObjectDescriptor::Trivial];
    let txn_ctx = NoopTransactionContext;
    let gas_meter = SimpleGasMeter::new(u64::MAX);
    let mut ctx = InterpreterContext::new(&txn_ctx, &descriptors, gas_meter, unsafe {
        functions[0].as_ref_unchecked()
    });
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
    let vec_ref: u32 = 48;

    let arena = ExecutableArena::new();

    #[rustfmt::skip]
    let code = arena.alloc_slice_fill_iter([
        VecNew { dst: FO(vec) },
        SlotBorrow { dst: FO(vec_ref), local: FO(vec) },
        StoreImm8 { dst: FO(tmp), imm: 100 },
        VecPushBack { vec_ref: FO(vec_ref), elem: FO(tmp), elem_size: 8, descriptor_id: DescriptorId(0) },
        StoreImm8 { dst: FO(tmp), imm: 200 },
        VecPushBack { vec_ref: FO(vec_ref), elem: FO(tmp), elem_size: 8, descriptor_id: DescriptorId(0) },
        StoreImm8 { dst: FO(tmp), imm: 300 },
        VecPushBack { vec_ref: FO(vec_ref), elem: FO(tmp), elem_size: 8, descriptor_id: DescriptorId(0) },
        StoreImm8 { dst: FO(idx), imm: 2 },
        VecBorrow { dst: FO(r#ref), vec_ref: FO(vec_ref), idx: FO(idx), elem_size: 8 },
        ForceGC,
        ReadRef { dst: FO(result), ref_ptr: FO(r#ref), size: 8 },
        StoreImm8 { dst: FO(tmp), imm: 42 },
        WriteRef { ref_ptr: FO(r#ref), src: FO(tmp), size: 8 },
        VecLoadElem { dst: FO(tmp), vec_ref: FO(vec_ref), idx: FO(idx), elem_size: 8 },
        Return,
    ]);
    let functions = [arena.alloc(Function {
        name: GlobalArenaPtr::from_static("test"),
        code,
        param_sizes: ExecutableArenaPtr::empty_slice(),
        param_sizes_sum: 0,
        param_and_local_sizes_sum: 64,
        extended_frame_size: 88,
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(&arena, [FO(vec), FO(ref_base), FO(vec_ref)]),
        safe_point_layouts: SortedSafePointEntries::empty(),
    })];
    let descriptors = [ObjectDescriptor::Trivial];
    let txn_ctx = NoopTransactionContext;
    let gas_meter = SimpleGasMeter::new(u64::MAX);
    let mut ctx = InterpreterContext::new(&txn_ctx, &descriptors, gas_meter, unsafe {
        functions[0].as_ref_unchecked()
    });
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

    let arena = ExecutableArena::new();

    #[rustfmt::skip]
    let callee_code = arena.alloc_slice_fill_iter([
        ForceGC,
        StoreImm8 { dst: FO(c_val), imm: 77 },
        WriteRef { ref_ptr: FO(c_ref), src: FO(c_val), size: 8 },
        Return,
    ]);
    let callee_func = arena.alloc(Function {
        name: GlobalArenaPtr::from_static("test"),
        code: callee_code,
        param_sizes: ExecutableArenaPtr::empty_slice(),
        param_sizes_sum: 16,
        param_and_local_sizes_sum: 24,
        extended_frame_size: 48,
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(&arena, [FO(c_ref_base)]),
        safe_point_layouts: SortedSafePointEntries::empty(),
    });

    // -- Function 0: main --
    let m_result: u32 = 0;
    let m_vec: u32 = 8;
    let m_idx: u32 = 16;
    let m_tmp: u32 = 24;
    let m_vec_ref: u32 = 32;
    let m_call_site: u32 = 48;
    let m_callee_ref: u32 = m_call_site + FRAME_METADATA_SIZE as u32; // 72

    #[rustfmt::skip]
    let main_code = arena.alloc_slice_fill_iter([
        VecNew { dst: FO(m_vec) },
        SlotBorrow { dst: FO(m_vec_ref), local: FO(m_vec) },
        StoreImm8 { dst: FO(m_tmp), imm: 10 },
        VecPushBack { vec_ref: FO(m_vec_ref), elem: FO(m_tmp), elem_size: 8, descriptor_id: DescriptorId(0) },
        StoreImm8 { dst: FO(m_tmp), imm: 20 },
        VecPushBack { vec_ref: FO(m_vec_ref), elem: FO(m_tmp), elem_size: 8, descriptor_id: DescriptorId(0) },
        StoreImm8 { dst: FO(m_tmp), imm: 30 },
        VecPushBack { vec_ref: FO(m_vec_ref), elem: FO(m_tmp), elem_size: 8, descriptor_id: DescriptorId(0) },
        StoreImm8 { dst: FO(m_idx), imm: 1 },
        VecBorrow { dst: FO(m_callee_ref), vec_ref: FO(m_vec_ref), idx: FO(m_idx), elem_size: 8 },
        CallFunc { func_id: 1 },
        VecLoadElem { dst: FO(m_result), vec_ref: FO(m_vec_ref), idx: FO(m_idx), elem_size: 8 },
        Return,
    ]);
    let main_func = arena.alloc(Function {
        name: GlobalArenaPtr::from_static("test"),
        code: main_code,
        param_sizes: ExecutableArenaPtr::empty_slice(),
        param_sizes_sum: 0,
        param_and_local_sizes_sum: 48,
        extended_frame_size: 88,
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(&arena, [FO(m_vec), FO(m_vec_ref), FO(m_callee_ref)]),
        safe_point_layouts: SortedSafePointEntries::empty(),
    });

    let descriptors = [ObjectDescriptor::Trivial];
    let functions = [Some(main_func), Some(callee_func)];
    // SAFETY: Exclusive access during test setup; arena is alive.
    unsafe { Function::resolve_calls(&functions) };
    let txn_ctx = NoopTransactionContext;
    let gas_meter = SimpleGasMeter::new(u64::MAX);
    let mut ctx = InterpreterContext::new(&txn_ctx, &descriptors, gas_meter, unsafe {
        functions[0].unwrap().as_ref_unchecked()
    });
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
    let vec_ref: u32 = 72;

    let arena = ExecutableArena::new();

    #[rustfmt::skip]
    let code = arena.alloc_slice_fill_iter([
        VecNew { dst: FO(vec) },
        SlotBorrow { dst: FO(vec_ref), local: FO(vec) },
        StoreImm8 { dst: FO(tmp), imm: 10 },
        VecPushBack { vec_ref: FO(vec_ref), elem: FO(tmp), elem_size: 8, descriptor_id: DescriptorId(0) },
        StoreImm8 { dst: FO(tmp), imm: 20 },
        VecPushBack { vec_ref: FO(vec_ref), elem: FO(tmp), elem_size: 8, descriptor_id: DescriptorId(0) },
        StoreImm8 { dst: FO(tmp), imm: 30 },
        VecPushBack { vec_ref: FO(vec_ref), elem: FO(tmp), elem_size: 8, descriptor_id: DescriptorId(0) },
        StoreImm8 { dst: FO(tmp), imm: 40 },
        VecPushBack { vec_ref: FO(vec_ref), elem: FO(tmp), elem_size: 8, descriptor_id: DescriptorId(0) },
        StoreImm8 { dst: FO(idx), imm: 1 },
        VecBorrow { dst: FO(ref_a), vec_ref: FO(vec_ref), idx: FO(idx), elem_size: 8 },
        StoreImm8 { dst: FO(idx), imm: 3 },
        VecBorrow { dst: FO(ref_b), vec_ref: FO(vec_ref), idx: FO(idx), elem_size: 8 },
        ForceGC,
        ReadRef { dst: FO(result), ref_ptr: FO(ref_a), size: 8 },
        StoreImm8 { dst: FO(val), imm: 55 },
        WriteRef { ref_ptr: FO(ref_a), src: FO(val), size: 8 },
        StoreImm8 { dst: FO(val), imm: 66 },
        WriteRef { ref_ptr: FO(ref_b), src: FO(val), size: 8 },
        Return,
    ]);
    let functions = [arena.alloc(Function {
        name: GlobalArenaPtr::from_static("test"),
        code,
        param_sizes: ExecutableArenaPtr::empty_slice(),
        param_sizes_sum: 0,
        param_and_local_sizes_sum: 88,
        extended_frame_size: 112,
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(&arena, [
            FO(vec),
            FO(ref_a_base),
            FO(ref_b_base),
            FO(vec_ref),
        ]),
        safe_point_layouts: SortedSafePointEntries::empty(),
    })];
    let descriptors = [ObjectDescriptor::Trivial];
    let txn_ctx = NoopTransactionContext;
    let gas_meter = SimpleGasMeter::new(u64::MAX);
    let mut ctx = InterpreterContext::new(&txn_ctx, &descriptors, gas_meter, unsafe {
        functions[0].as_ref_unchecked()
    });
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
    let vec_ref: u32 = 48;

    let arena = ExecutableArena::new();

    #[rustfmt::skip]
    let code = arena.alloc_slice_fill_iter([
        StoreImm8 { dst: FO(local_val), imm: 42 },
        SlotBorrow { dst: FO(r#ref), local: FO(local_val) },
        VecNew { dst: FO(vec) },
        SlotBorrow { dst: FO(vec_ref), local: FO(vec) },
        ForceGC,
        ReadRef { dst: FO(result), ref_ptr: FO(r#ref), size: 8 },
        StoreImm8 { dst: FO(tmp), imm: 99 },
        WriteRef { ref_ptr: FO(r#ref), src: FO(tmp), size: 8 },
        ReadRef { dst: FO(result), ref_ptr: FO(r#ref), size: 8 },
        Return,
    ]);
    let functions = [arena.alloc(Function {
        name: GlobalArenaPtr::from_static("test"),
        code,
        param_sizes: ExecutableArenaPtr::empty_slice(),
        param_sizes_sum: 0,
        param_and_local_sizes_sum: 64,
        extended_frame_size: 88,
        // ref_base holds a stack address → is_heap_ptr returns false, GC skips it.
        // vec is a genuine heap root.
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(&arena, [FO(ref_base), FO(vec), FO(vec_ref)]),
        safe_point_layouts: SortedSafePointEntries::empty(),
    })];
    let descriptors = [ObjectDescriptor::Trivial];
    let txn_ctx = NoopTransactionContext;
    let gas_meter = SimpleGasMeter::new(u64::MAX);
    let mut ctx = InterpreterContext::new(&txn_ctx, &descriptors, gas_meter, unsafe {
        functions[0].as_ref_unchecked()
    });
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
    let outer_ref: u32 = 64;
    let inner_ref: u32 = 80;

    let arena = ExecutableArena::new();

    #[rustfmt::skip]
    let code = arena.alloc_slice_fill_iter([
        // -- Build inner0 = [100, 200] --
        VecNew { dst: FO(inner_ptr) },
        SlotBorrow { dst: FO(inner_ref), local: FO(inner_ptr) },
        StoreImm8 { dst: FO(tmp), imm: 100 },
        VecPushBack { vec_ref: FO(inner_ref), elem: FO(tmp), elem_size: 8, descriptor_id: DescriptorId(0) },
        StoreImm8 { dst: FO(tmp), imm: 200 },
        VecPushBack { vec_ref: FO(inner_ref), elem: FO(tmp), elem_size: 8, descriptor_id: DescriptorId(0) },
        // -- Build outer --
        VecNew { dst: FO(outer) },
        SlotBorrow { dst: FO(outer_ref), local: FO(outer) },
        VecPushBack { vec_ref: FO(outer_ref), elem: FO(inner_ptr), elem_size: 8, descriptor_id: DescriptorId(1) },
        // -- Build inner1 = [300, 400, 500] --
        VecNew { dst: FO(inner_ptr) },
        SlotBorrow { dst: FO(inner_ref), local: FO(inner_ptr) },
        StoreImm8 { dst: FO(tmp), imm: 300 },
        VecPushBack { vec_ref: FO(inner_ref), elem: FO(tmp), elem_size: 8, descriptor_id: DescriptorId(0) },
        StoreImm8 { dst: FO(tmp), imm: 400 },
        VecPushBack { vec_ref: FO(inner_ref), elem: FO(tmp), elem_size: 8, descriptor_id: DescriptorId(0) },
        StoreImm8 { dst: FO(tmp), imm: 500 },
        VecPushBack { vec_ref: FO(inner_ref), elem: FO(tmp), elem_size: 8, descriptor_id: DescriptorId(0) },
        VecPushBack { vec_ref: FO(outer_ref), elem: FO(inner_ptr), elem_size: 8, descriptor_id: DescriptorId(1) },
        // -- Load outer[1] → inner1 ptr, borrow inner1[2] --
        StoreImm8 { dst: FO(idx), imm: 1 },
        VecLoadElem { dst: FO(inner_ptr), vec_ref: FO(outer_ref), idx: FO(idx), elem_size: 8 },
        SlotBorrow { dst: FO(inner_ref), local: FO(inner_ptr) },
        StoreImm8 { dst: FO(idx), imm: 2 },
        VecBorrow { dst: FO(r#ref), vec_ref: FO(inner_ref), idx: FO(idx), elem_size: 8 },
        ForceGC,
        ReadRef { dst: FO(result), ref_ptr: FO(r#ref), size: 8 },
        StoreImm8 { dst: FO(val), imm: 999 },
        WriteRef { ref_ptr: FO(r#ref), src: FO(val), size: 8 },
        Return,
    ]);
    let functions = [arena.alloc(Function {
        name: GlobalArenaPtr::from_static("test"),
        code,
        param_sizes: ExecutableArenaPtr::empty_slice(),
        param_sizes_sum: 0,
        param_and_local_sizes_sum: 96,
        extended_frame_size: 120,
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(&arena, [
            FO(outer),
            FO(inner_ptr),
            FO(ref_base),
            FO(outer_ref),
            FO(inner_ref),
        ]),
        safe_point_layouts: SortedSafePointEntries::empty(),
    })];
    let descriptors = [ObjectDescriptor::Trivial, ObjectDescriptor::Vector {
        elem_size: 8,
        elem_pointer_offsets: vec![0],
    }];
    let txn_ctx = NoopTransactionContext;
    let gas_meter = SimpleGasMeter::new(u64::MAX);
    let mut ctx = InterpreterContext::new(&txn_ctx, &descriptors, gas_meter, unsafe {
        functions[0].as_ref_unchecked()
    });
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
    let vec_ref: u32 = 48;

    let arena = ExecutableArena::new();

    #[rustfmt::skip]
    let code = arena.alloc_slice_fill_iter([
        VecNew { dst: FO(vec) },
        SlotBorrow { dst: FO(vec_ref), local: FO(vec) },
        StoreImm8 { dst: FO(tmp), imm: 10 },
        VecPushBack { vec_ref: FO(vec_ref), elem: FO(tmp), elem_size: 8, descriptor_id: DescriptorId(0) },
        StoreImm8 { dst: FO(tmp), imm: 20 },
        VecPushBack { vec_ref: FO(vec_ref), elem: FO(tmp), elem_size: 8, descriptor_id: DescriptorId(0) },
        StoreImm8 { dst: FO(tmp), imm: 30 },
        VecPushBack { vec_ref: FO(vec_ref), elem: FO(tmp), elem_size: 8, descriptor_id: DescriptorId(0) },
        StoreImm8 { dst: FO(idx), imm: 1 },
        VecBorrow { dst: FO(r#ref), vec_ref: FO(vec_ref), idx: FO(idx), elem_size: 8 },
        ForceGC,
        ForceGC,
        ReadRef { dst: FO(result), ref_ptr: FO(r#ref), size: 8 },
        StoreImm8 { dst: FO(tmp), imm: 77 },
        WriteRef { ref_ptr: FO(r#ref), src: FO(tmp), size: 8 },
        Return,
    ]);
    let functions = [arena.alloc(Function {
        name: GlobalArenaPtr::from_static("test"),
        code,
        param_sizes: ExecutableArenaPtr::empty_slice(),
        param_sizes_sum: 0,
        param_and_local_sizes_sum: 64,
        extended_frame_size: 88,
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(&arena, [FO(vec), FO(ref_base), FO(vec_ref)]),
        safe_point_layouts: SortedSafePointEntries::empty(),
    })];
    let descriptors = [ObjectDescriptor::Trivial];
    let txn_ctx = NoopTransactionContext;
    let gas_meter = SimpleGasMeter::new(u64::MAX);
    let mut ctx = InterpreterContext::new(&txn_ctx, &descriptors, gas_meter, unsafe {
        functions[0].as_ref_unchecked()
    });
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
    let entry_ref: u32 = 32; // 16-byte fat pointer ref to entry (for struct_borrow)

    let arena = ExecutableArena::new();

    #[rustfmt::skip]
    let code = arena.alloc_slice_fill_iter([
        HeapNew { dst: FO(entry), descriptor_id: DescriptorId(0) },
        StoreImm8 { dst: FO(result), imm: 42 },
        MicroOp::struct_store8(FO(entry), 0, FO(result)),
        StoreImm8 { dst: FO(result), imm: 100 },
        MicroOp::struct_store8(FO(entry), 8, FO(result)),
        SlotBorrow { dst: FO(entry_ref), local: FO(entry) },
        MicroOp::struct_borrow(FO(entry_ref), 8, FO(r#ref)),
        ReadRef { dst: FO(result), ref_ptr: FO(r#ref), size: 8 },
        StoreImm8 { dst: FO(result), imm: 55 },
        WriteRef { ref_ptr: FO(r#ref), src: FO(result), size: 8 },
        MicroOp::struct_load8(FO(entry), 8, FO(result)),
        Return,
    ]);
    let functions = [arena.alloc(Function {
        name: GlobalArenaPtr::from_static("test"),
        code,
        param_sizes: ExecutableArenaPtr::empty_slice(),
        param_sizes_sum: 0,
        param_and_local_sizes_sum: 48,
        extended_frame_size: 72,
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(&arena, [FO(entry), FO(r#ref), FO(entry_ref)]),
        safe_point_layouts: SortedSafePointEntries::empty(),
    })];
    let descriptors = [ObjectDescriptor::Struct {
        size: 16,
        pointer_offsets: vec![],
    }];
    let txn_ctx = NoopTransactionContext;
    let gas_meter = SimpleGasMeter::new(u64::MAX);
    let mut ctx = InterpreterContext::new(&txn_ctx, &descriptors, gas_meter, unsafe {
        functions[0].as_ref_unchecked()
    });
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
    let entry_ref: u32 = 32; // 16-byte fat pointer ref to entry (for struct_borrow)

    let arena = ExecutableArena::new();

    #[rustfmt::skip]
    let code = arena.alloc_slice_fill_iter([
        HeapNew { dst: FO(entry), descriptor_id: DescriptorId(0) },
        StoreImm8 { dst: FO(result), imm: 7 },
        MicroOp::struct_store8(FO(entry), 0, FO(result)),
        StoreImm8 { dst: FO(result), imm: 13 },
        MicroOp::struct_store8(FO(entry), 8, FO(result)),
        SlotBorrow { dst: FO(entry_ref), local: FO(entry) },
        MicroOp::struct_borrow(FO(entry_ref), 8, FO(r#ref)),
        ForceGC,
        ReadRef { dst: FO(result), ref_ptr: FO(r#ref), size: 8 },
        Return,
    ]);
    let functions = [arena.alloc(Function {
        name: GlobalArenaPtr::from_static("test"),
        code,
        param_sizes: ExecutableArenaPtr::empty_slice(),
        param_sizes_sum: 0,
        param_and_local_sizes_sum: 48,
        extended_frame_size: 72,
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(&arena, [FO(entry), FO(ref_base), FO(entry_ref)]),
        safe_point_layouts: SortedSafePointEntries::empty(),
    })];
    let descriptors = [ObjectDescriptor::Struct {
        size: 16,
        pointer_offsets: vec![],
    }];
    let txn_ctx = NoopTransactionContext;
    let gas_meter = SimpleGasMeter::new(u64::MAX);
    let mut ctx = InterpreterContext::new(&txn_ctx, &descriptors, gas_meter, unsafe {
        functions[0].as_ref_unchecked()
    });
    ctx.run().unwrap();

    assert_eq!(ctx.root_result(), 13, "entry.value should be 13 after GC");
    assert_eq!(ctx.gc_count(), 1);

    let entry_ptr = ctx.root_heap_ptr(8);
    let key = unsafe { read_u64(entry_ptr, STRUCT_DATA_OFFSET) };
    assert_eq!(key, 7, "entry.key should survive GC");
}
