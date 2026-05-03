// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for Move struct support (both inline and heap-allocated).

use mono_move_alloc::{ExecutableArena, ExecutableArenaPtr, GlobalArenaPtr};
use mono_move_core::{
    FrameLayoutInfo, FrameOffset as FO, Function, LocalExecutionContext, MicroOp,
    SortedSafePointEntries, STRUCT_DATA_OFFSET,
};
use mono_move_runtime::{
    read_ptr, read_u64, InterpreterContext, ObjectDescriptor, ObjectDescriptorTable,
    VEC_DATA_OFFSET, VEC_LENGTH_OFFSET,
};

// ---------------------------------------------------------------------------
// Test 1: struct_inline — inline struct on the stack (no new instructions)
// ---------------------------------------------------------------------------

#[test]
fn struct_inline() {
    use MicroOp::*;

    let result: u32 = 0;
    let pair_a: u32 = 8;
    let pair_b: u32 = 16;

    let arena = ExecutableArena::new();

    #[rustfmt::skip]
    let code = arena.alloc_slice_fill_iter([
        StoreImm8 { dst: FO(pair_a), imm: 10 },
        StoreImm8 { dst: FO(pair_b), imm: 20 },
        AddU64 { dst: FO(result), lhs: FO(pair_a), rhs: FO(pair_b) },
        Return,
    ]);
    let functions = [arena.alloc(Function {
        name: GlobalArenaPtr::from_static("test"),
        code,
        param_sizes: ExecutableArenaPtr::empty_slice(),
        param_sizes_sum: 0,
        param_and_local_sizes_sum: 24,
        extended_frame_size: 48,
        zero_frame: false,
        frame_layout: FrameLayoutInfo::empty(),
        safe_point_layouts: SortedSafePointEntries::empty(),
    })];
    let descriptors = ObjectDescriptorTable::new();

    let mut exec_ctx = LocalExecutionContext::with_max_budget();
    let mut ctx = InterpreterContext::new(&mut exec_ctx, &descriptors, unsafe {
        functions[0].as_ref_unchecked()
    });
    ctx.run().unwrap();

    assert_eq!(ctx.root_result(), 30, "result should be 10 + 20 = 30");
}

// ---------------------------------------------------------------------------
// Test 2: struct_inline_borrow
// ---------------------------------------------------------------------------

#[test]
fn struct_inline_borrow() {
    use MicroOp::*;

    let result: u32 = 0;
    let pair_a: u32 = 8;
    let pair_b: u32 = 16;
    let r#ref: u32 = 24;

    let arena = ExecutableArena::new();

    #[rustfmt::skip]
    let code = arena.alloc_slice_fill_iter([
        StoreImm8 { dst: FO(pair_a), imm: 10 },
        StoreImm8 { dst: FO(pair_b), imm: 20 },
        SlotBorrow { dst: FO(r#ref), local: FO(pair_b) },
        ReadRef { dst: FO(result), ref_ptr: FO(r#ref), size: 8 },
        StoreImm8 { dst: FO(result), imm: 99 },
        WriteRef { ref_ptr: FO(r#ref), src: FO(result), size: 8 },
        Move8 { dst: FO(result), src: FO(pair_b) },
        Return,
    ]);
    let functions = [arena.alloc(Function {
        name: GlobalArenaPtr::from_static("test"),
        code,
        param_sizes: ExecutableArenaPtr::empty_slice(),
        param_sizes_sum: 0,
        param_and_local_sizes_sum: 40,
        extended_frame_size: 64,
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(&arena, [FO(r#ref)]),
        safe_point_layouts: SortedSafePointEntries::empty(),
    })];
    let descriptors = ObjectDescriptorTable::new();

    let mut exec_ctx = LocalExecutionContext::with_max_budget();
    let mut ctx = InterpreterContext::new(&mut exec_ctx, &descriptors, unsafe {
        functions[0].as_ref_unchecked()
    });
    ctx.run().unwrap();

    assert_eq!(ctx.root_result(), 99, "pair.b should be 99 after WriteRef");
}

// ---------------------------------------------------------------------------
// Test 3: struct_heap_basic
// ---------------------------------------------------------------------------

#[test]
fn struct_heap_basic() {
    use MicroOp::*;

    let result: u32 = 0;
    let entry: u32 = 8;
    let tmp: u32 = 16;

    let arena = ExecutableArena::new();

    let mut descriptors = ObjectDescriptorTable::new();
    let desc_entry_struct = descriptors.push(ObjectDescriptor::new_struct(16, vec![]).unwrap());

    #[rustfmt::skip]
    let code = arena.alloc_slice_fill_iter([
        HeapNew { dst: FO(entry), descriptor_id: desc_entry_struct },
        StoreImm8 { dst: FO(tmp), imm: 42 },
        MicroOp::struct_store8(FO(entry), 0, FO(tmp)),
        StoreImm8 { dst: FO(tmp), imm: 100 },
        MicroOp::struct_store8(FO(entry), 8, FO(tmp)),
        MicroOp::struct_load8(FO(entry), 0, FO(result)),
        MicroOp::struct_load8(FO(entry), 8, FO(tmp)),
        AddU64 { dst: FO(result), lhs: FO(result), rhs: FO(tmp) },
        Return,
    ]);
    let functions = [arena.alloc(Function {
        name: GlobalArenaPtr::from_static("test"),
        code,
        param_sizes: ExecutableArenaPtr::empty_slice(),
        param_sizes_sum: 0,
        param_and_local_sizes_sum: 24,
        extended_frame_size: 48,
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(&arena, [FO(entry)]),
        safe_point_layouts: SortedSafePointEntries::empty(),
    })];

    let mut exec_ctx = LocalExecutionContext::with_max_budget();
    let mut ctx = InterpreterContext::new(&mut exec_ctx, &descriptors, unsafe {
        functions[0].as_ref_unchecked()
    });
    ctx.run().unwrap();

    assert_eq!(ctx.root_result(), 142, "result should be 42 + 100 = 142");
}

// ---------------------------------------------------------------------------
// Test 4: struct_heap_survives_gc
// ---------------------------------------------------------------------------

#[test]
fn struct_heap_survives_gc() {
    use MicroOp::*;

    let result: u32 = 0;
    let entry: u32 = 8;
    let tmp: u32 = 16;

    let arena = ExecutableArena::new();

    let mut descriptors = ObjectDescriptorTable::new();
    let desc_entry_struct = descriptors.push(ObjectDescriptor::new_struct(16, vec![]).unwrap());

    #[rustfmt::skip]
    let code = arena.alloc_slice_fill_iter([
        HeapNew { dst: FO(entry), descriptor_id: desc_entry_struct },
        StoreImm8 { dst: FO(tmp), imm: 7 },
        MicroOp::struct_store8(FO(entry), 0, FO(tmp)),
        StoreImm8 { dst: FO(tmp), imm: 13 },
        MicroOp::struct_store8(FO(entry), 8, FO(tmp)),
        ForceGC,
        MicroOp::struct_load8(FO(entry), 0, FO(result)),
        MicroOp::struct_load8(FO(entry), 8, FO(tmp)),
        AddU64 { dst: FO(result), lhs: FO(result), rhs: FO(tmp) },
        Return,
    ]);
    let functions = [arena.alloc(Function {
        name: GlobalArenaPtr::from_static("test"),
        code,
        param_sizes: ExecutableArenaPtr::empty_slice(),
        param_sizes_sum: 0,
        param_and_local_sizes_sum: 24,
        extended_frame_size: 48,
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(&arena, [FO(entry)]),
        safe_point_layouts: SortedSafePointEntries::empty(),
    })];

    let mut exec_ctx = LocalExecutionContext::with_max_budget();
    let mut ctx = InterpreterContext::new(&mut exec_ctx, &descriptors, unsafe {
        functions[0].as_ref_unchecked()
    });
    ctx.run().unwrap();

    assert_eq!(
        ctx.root_result(),
        20,
        "result should be 7 + 13 = 20 after GC"
    );
    assert_eq!(ctx.gc_count(), 1);
}

// ---------------------------------------------------------------------------
// Test 5: struct_with_vector_field
// ---------------------------------------------------------------------------

#[test]
fn struct_with_vector_field() {
    use MicroOp::*;

    let result: u32 = 0;
    let ctr: u32 = 8;
    let items: u32 = 16;
    let tmp: u32 = 24;
    let vec_ref: u32 = 32; // 16-byte fat pointer referencing the vector
    let ctr_ref: u32 = 48; // 16-byte fat pointer ref to ctr (for struct_borrow)

    let arena = ExecutableArena::new();

    let mut descriptors = ObjectDescriptorTable::new();
    let desc_ctr_struct = descriptors.push(ObjectDescriptor::new_struct(16, vec![8]).unwrap());
    let desc_vec_u64 = descriptors.push(ObjectDescriptor::new_vector(8, vec![]).unwrap());

    #[rustfmt::skip]
    let code = arena.alloc_slice_fill_iter([
        HeapNew { dst: FO(ctr), descriptor_id: desc_ctr_struct },
        StoreImm8 { dst: FO(tmp), imm: 999 },
        MicroOp::struct_store8(FO(ctr), 0, FO(tmp)),
        VecNew { dst: FO(items) },
        SlotBorrow { dst: FO(vec_ref), local: FO(items) },
        StoreImm8 { dst: FO(tmp), imm: 10 },
        VecPushBack { vec_ref: FO(vec_ref), elem: FO(tmp), elem_size: 8, descriptor_id: desc_vec_u64 },
        StoreImm8 { dst: FO(tmp), imm: 20 },
        VecPushBack { vec_ref: FO(vec_ref), elem: FO(tmp), elem_size: 8, descriptor_id: desc_vec_u64 },
        StoreImm8 { dst: FO(tmp), imm: 30 },
        VecPushBack { vec_ref: FO(vec_ref), elem: FO(tmp), elem_size: 8, descriptor_id: desc_vec_u64 },
        MicroOp::struct_store8(FO(ctr), 8, FO(items)),
        ForceGC,
        SlotBorrow { dst: FO(ctr_ref), local: FO(ctr) },
        MicroOp::struct_borrow(FO(ctr_ref), 8, FO(vec_ref)),
        MicroOp::struct_load8(FO(ctr), 0, FO(result)),
        VecLen { dst: FO(tmp), vec_ref: FO(vec_ref) },
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
        frame_layout: FrameLayoutInfo::new(&arena, [FO(ctr), FO(items), FO(vec_ref), FO(ctr_ref)]),
        safe_point_layouts: SortedSafePointEntries::empty(),
    })];

    let mut exec_ctx = LocalExecutionContext::with_max_budget();
    let mut ctx = InterpreterContext::new(&mut exec_ctx, &descriptors, unsafe {
        functions[0].as_ref_unchecked()
    });
    ctx.run().unwrap();

    assert_eq!(ctx.root_result(), 999, "ctr.tag should be 999 after GC");

    let ctr_ptr = ctx.root_heap_ptr(8);
    let items_ptr = unsafe { read_ptr(ctr_ptr, STRUCT_DATA_OFFSET + 8) };
    let len = unsafe { read_u64(items_ptr, VEC_LENGTH_OFFSET) };
    assert_eq!(len, 3, "items vector should have 3 elements");
    let e0 = unsafe { read_u64(items_ptr, VEC_DATA_OFFSET) };
    let e1 = unsafe { read_u64(items_ptr, VEC_DATA_OFFSET + 8) };
    let e2 = unsafe { read_u64(items_ptr, VEC_DATA_OFFSET + 16) };
    assert_eq!(e0, 10);
    assert_eq!(e1, 20);
    assert_eq!(e2, 30);
    assert_eq!(ctx.gc_count(), 1);
}

// ---------------------------------------------------------------------------
// Test 6: struct_borrow_field
// ---------------------------------------------------------------------------

#[test]
fn struct_borrow_field() {
    use MicroOp::*;

    let result: u32 = 0;
    let entry: u32 = 8;
    let r#ref: u32 = 16;
    let entry_ref: u32 = 32; // 16-byte fat pointer ref to entry (for struct_borrow)

    let arena = ExecutableArena::new();

    let mut descriptors = ObjectDescriptorTable::new();
    let desc_entry_struct = descriptors.push(ObjectDescriptor::new_struct(16, vec![]).unwrap());

    #[rustfmt::skip]
    let code = arena.alloc_slice_fill_iter([
        HeapNew { dst: FO(entry), descriptor_id: desc_entry_struct },
        StoreImm8 { dst: FO(result), imm: 5 },
        MicroOp::struct_store8(FO(entry), 0, FO(result)),
        StoreImm8 { dst: FO(result), imm: 10 },
        MicroOp::struct_store8(FO(entry), 8, FO(result)),
        SlotBorrow { dst: FO(entry_ref), local: FO(entry) },
        MicroOp::struct_borrow(FO(entry_ref), 8, FO(r#ref)),
        ReadRef { dst: FO(result), ref_ptr: FO(r#ref), size: 8 },
        StoreImm8 { dst: FO(result), imm: 77 },
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

    let mut exec_ctx = LocalExecutionContext::with_max_budget();
    let mut ctx = InterpreterContext::new(&mut exec_ctx, &descriptors, unsafe {
        functions[0].as_ref_unchecked()
    });
    ctx.run().unwrap();

    assert_eq!(
        ctx.root_result(),
        77,
        "entry.value should be 77 after WriteRef"
    );
}

// ---------------------------------------------------------------------------
// Test 7: struct_borrow_survives_gc
// ---------------------------------------------------------------------------

#[test]
fn struct_borrow_survives_gc() {
    use MicroOp::*;

    let result: u32 = 0;
    let entry: u32 = 8;
    let r#ref: u32 = 16;
    let ref_base: u32 = 16;
    let entry_ref: u32 = 32; // 16-byte fat pointer ref to entry (for struct_borrow)

    let arena = ExecutableArena::new();

    let mut descriptors = ObjectDescriptorTable::new();
    let desc_entry_struct = descriptors.push(ObjectDescriptor::new_struct(16, vec![]).unwrap());

    #[rustfmt::skip]
    let code = arena.alloc_slice_fill_iter([
        HeapNew { dst: FO(entry), descriptor_id: desc_entry_struct },
        StoreImm8 { dst: FO(result), imm: 100 },
        MicroOp::struct_store8(FO(entry), 0, FO(result)),
        StoreImm8 { dst: FO(result), imm: 200 },
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

    let mut exec_ctx = LocalExecutionContext::with_max_budget();
    let mut ctx = InterpreterContext::new(&mut exec_ctx, &descriptors, unsafe {
        functions[0].as_ref_unchecked()
    });
    ctx.run().unwrap();

    assert_eq!(ctx.root_result(), 200, "entry.value should be 200 after GC");
    assert_eq!(ctx.gc_count(), 1);

    let entry_ptr = ctx.root_heap_ptr(8);
    let key = unsafe { read_u64(entry_ptr, STRUCT_DATA_OFFSET) };
    assert_eq!(key, 100, "entry.key should be 100 after GC");
}
