// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for Move enum support (heap-allocated tagged unions).

use mono_move_alloc::{ExecutableArena, GlobalArenaPtr};
use mono_move_core::{
    CodeOffset as CO, DescriptorId, FrameOffset as FO, Function, MicroOp, ENUM_DATA_OFFSET,
    ENUM_TAG_OFFSET,
};
use mono_move_runtime::{
    read_ptr, read_u64, InterpreterContext, ObjectDescriptor, VEC_DATA_OFFSET, VEC_LENGTH_OFFSET,
};

// ---------------------------------------------------------------------------
// Test 1: enum_basic
// ---------------------------------------------------------------------------

#[test]
fn enum_basic() {
    use MicroOp::*;

    let result: u32 = 0;
    let shape: u32 = 8;
    let tmp: u32 = 16;

    let arena = ExecutableArena::new();

    #[rustfmt::skip]
    let code = arena.alloc_slice_fill_iter(vec![
        HeapNew { dst: FO(shape), descriptor_id: DescriptorId(0) },
        MicroOp::enum_set_tag(FO(shape), 1),
        StoreImm8 { dst: FO(tmp), imm: 3 },
        MicroOp::enum_store8(FO(shape), 0, FO(tmp)),
        StoreImm8 { dst: FO(tmp), imm: 4 },
        MicroOp::enum_store8(FO(shape), 8, FO(tmp)),
        MicroOp::enum_get_tag(FO(shape), FO(result)),
        MicroOp::enum_load8(FO(shape), 0, FO(tmp)),
        MicroOp::enum_load8(FO(shape), 0, FO(result)),
        MicroOp::enum_load8(FO(shape), 8, FO(tmp)),
        AddU64 { dst: FO(result), lhs: FO(result), rhs: FO(tmp) },
        Return,
    ]);
    let pointer_offsets = arena.alloc_slice_fill_iter(vec![FO(shape)]);

    let functions = [arena.alloc(Function {
        name: GlobalArenaPtr::from_static("test"),
        code,
        args_size: 0,
        args_and_locals_size: 24,
        extended_frame_size: 48,
        zero_frame: true,
        pointer_offsets,
    })];
    let descriptors = vec![ObjectDescriptor::Enum {
        size: 24,
        variant_pointer_offsets: vec![vec![], vec![]],
    }];
    let mut ctx = InterpreterContext::new(&descriptors, unsafe { functions[0].as_ref_unchecked() });
    ctx.run().unwrap();

    assert_eq!(ctx.root_result(), 7, "result should be 3 + 4 = 7");

    let shape_ptr = ctx.root_heap_ptr(8);
    let tag = unsafe { read_u64(shape_ptr, ENUM_TAG_OFFSET) };
    assert_eq!(tag, 1, "tag should be 1 (Rect)");
    let w = unsafe { read_u64(shape_ptr, ENUM_DATA_OFFSET) };
    let h = unsafe { read_u64(shape_ptr, ENUM_DATA_OFFSET + 8) };
    assert_eq!(w, 3);
    assert_eq!(h, 4);
}

// ---------------------------------------------------------------------------
// Test 2: enum_survives_gc
// ---------------------------------------------------------------------------

#[test]
fn enum_survives_gc() {
    use MicroOp::*;

    let result: u32 = 0;
    let shape: u32 = 8;
    let tmp: u32 = 16;

    let arena = ExecutableArena::new();

    #[rustfmt::skip]
    let code = arena.alloc_slice_fill_iter(vec![
        HeapNew { dst: FO(shape), descriptor_id: DescriptorId(0) },
        MicroOp::enum_set_tag(FO(shape), 0),
        StoreImm8 { dst: FO(tmp), imm: 42 },
        MicroOp::enum_store8(FO(shape), 0, FO(tmp)),
        ForceGC,
        MicroOp::enum_get_tag(FO(shape), FO(result)),
        MicroOp::enum_load8(FO(shape), 0, FO(tmp)),
        AddU64 { dst: FO(result), lhs: FO(result), rhs: FO(tmp) },
        Return,
    ]);
    let pointer_offsets = arena.alloc_slice_fill_iter(vec![FO(shape)]);

    let functions = [arena.alloc(Function {
        name: GlobalArenaPtr::from_static("test"),
        code,
        args_size: 0,
        args_and_locals_size: 24,
        extended_frame_size: 48,
        zero_frame: true,
        pointer_offsets,
    })];
    let descriptors = vec![ObjectDescriptor::Enum {
        size: 24,
        variant_pointer_offsets: vec![vec![], vec![]],
    }];
    let mut ctx = InterpreterContext::new(&descriptors, unsafe { functions[0].as_ref_unchecked() });
    ctx.run().unwrap();

    assert_eq!(
        ctx.root_result(),
        42,
        "result should be 0 + 42 = 42 after GC"
    );
    assert_eq!(ctx.gc_count(), 1);
}

// ---------------------------------------------------------------------------
// Test 3: enum_gc_traces_refs
// ---------------------------------------------------------------------------

#[test]
fn enum_gc_traces_refs() {
    use MicroOp::*;

    let result: u32 = 0;
    let val: u32 = 8;
    let vec: u32 = 16;
    let tmp: u32 = 24;
    let vec_ref: u32 = 32;

    let arena = ExecutableArena::new();

    #[rustfmt::skip]
    let code = arena.alloc_slice_fill_iter(vec![
        VecNew { dst: FO(vec) },
        SlotBorrow { dst: FO(vec_ref), local: FO(vec) },
        StoreImm8 { dst: FO(tmp), imm: 10 },
        VecPushBack { vec_ref: FO(vec_ref), elem: FO(tmp), elem_size: 8, descriptor_id: DescriptorId(1) },
        StoreImm8 { dst: FO(tmp), imm: 20 },
        VecPushBack { vec_ref: FO(vec_ref), elem: FO(tmp), elem_size: 8, descriptor_id: DescriptorId(1) },
        StoreImm8 { dst: FO(tmp), imm: 30 },
        VecPushBack { vec_ref: FO(vec_ref), elem: FO(tmp), elem_size: 8, descriptor_id: DescriptorId(1) },
        HeapNew { dst: FO(val), descriptor_id: DescriptorId(0) },
        MicroOp::enum_set_tag(FO(val), 1),
        MicroOp::enum_store8(FO(val), 0, FO(vec)),
        ForceGC,
        MicroOp::enum_load8(FO(val), 0, FO(vec)),
        VecLen { dst: FO(result), vec_ref: FO(vec_ref) },
        StoreImm8 { dst: FO(tmp), imm: 0 },
        VecLoadElem { dst: FO(result), vec_ref: FO(vec_ref), idx: FO(tmp), elem_size: 8 },
        Return,
    ]);
    let pointer_offsets = arena.alloc_slice_fill_iter(vec![FO(val), FO(vec), FO(vec_ref)]);

    let functions = [arena.alloc(Function {
        name: GlobalArenaPtr::from_static("test"),
        code,
        args_size: 0,
        args_and_locals_size: 48,
        extended_frame_size: 72,
        zero_frame: true,
        pointer_offsets,
    })];
    let descriptors = vec![
        ObjectDescriptor::Enum {
            size: 16,
            variant_pointer_offsets: vec![vec![], vec![0]],
        },
        ObjectDescriptor::Trivial,
    ];
    let mut ctx = InterpreterContext::new(&descriptors, unsafe { functions[0].as_ref_unchecked() });
    ctx.run().unwrap();

    assert_eq!(ctx.root_result(), 10, "vec[0] should be 10 after GC");
    assert_eq!(ctx.gc_count(), 1);

    let val_ptr = ctx.root_heap_ptr(8);
    let tag = unsafe { read_u64(val_ptr, ENUM_TAG_OFFSET) };
    assert_eq!(tag, 1, "tag should be 1 (List)");
    let vec_ptr = unsafe { read_ptr(val_ptr, ENUM_DATA_OFFSET) };
    let len = unsafe { read_u64(vec_ptr, VEC_LENGTH_OFFSET) };
    assert_eq!(len, 3);
    let e0 = unsafe { read_u64(vec_ptr, VEC_DATA_OFFSET) };
    let e1 = unsafe { read_u64(vec_ptr, VEC_DATA_OFFSET + 8) };
    let e2 = unsafe { read_u64(vec_ptr, VEC_DATA_OFFSET + 16) };
    assert_eq!(e0, 10);
    assert_eq!(e1, 20);
    assert_eq!(e2, 30);
}

// ---------------------------------------------------------------------------
// Test 4: enum_pattern_match
// ---------------------------------------------------------------------------

#[test]
fn enum_pattern_match() {
    use MicroOp::*;

    let result: u32 = 0;
    let op: u32 = 8;
    let tmp: u32 = 16;

    let arena = ExecutableArena::new();

    #[rustfmt::skip]
    let code = arena.alloc_slice_fill_iter(vec![
        HeapNew { dst: FO(op), descriptor_id: DescriptorId(0) },
        MicroOp::enum_set_tag(FO(op), 0),
        StoreImm8 { dst: FO(tmp), imm: 10 },
        MicroOp::enum_store8(FO(op), 0, FO(tmp)),
        StoreImm8 { dst: FO(tmp), imm: 25 },
        MicroOp::enum_store8(FO(op), 8, FO(tmp)),
        MicroOp::enum_get_tag(FO(op), FO(tmp)),
        JumpNotZeroU64 { target: CO(12), src: FO(tmp) },
        MicroOp::enum_load8(FO(op), 0, FO(result)),
        MicroOp::enum_load8(FO(op), 8, FO(tmp)),
        AddU64 { dst: FO(result), lhs: FO(result), rhs: FO(tmp) },
        Return,
        StoreImm8 { dst: FO(result), imm: 0 },
        Return,
    ]);
    let pointer_offsets = arena.alloc_slice_fill_iter(vec![FO(op)]);

    let functions = [arena.alloc(Function {
        name: GlobalArenaPtr::from_static("test"),
        code,
        args_size: 0,
        args_and_locals_size: 24,
        extended_frame_size: 48,
        zero_frame: true,
        pointer_offsets,
    })];
    let descriptors = vec![ObjectDescriptor::Enum {
        size: 24,
        variant_pointer_offsets: vec![vec![], vec![]],
    }];
    let mut ctx = InterpreterContext::new(&descriptors, unsafe { functions[0].as_ref_unchecked() });
    ctx.run().unwrap();

    assert_eq!(ctx.root_result(), 35, "result should be 10 + 25 = 35");
}

// ---------------------------------------------------------------------------
// Test 5: enum_variant_switch
// ---------------------------------------------------------------------------

#[test]
fn enum_variant_switch() {
    use MicroOp::*;

    let result: u32 = 0;
    let e: u32 = 8;
    let tmp: u32 = 16;

    let arena = ExecutableArena::new();

    #[rustfmt::skip]
    let code = arena.alloc_slice_fill_iter(vec![
        HeapNew { dst: FO(e), descriptor_id: DescriptorId(0) },
        MicroOp::enum_set_tag(FO(e), 0),
        StoreImm8 { dst: FO(tmp), imm: 111 },
        MicroOp::enum_store8(FO(e), 0, FO(tmp)),
        MicroOp::enum_get_tag(FO(e), FO(result)),
        MicroOp::enum_set_tag(FO(e), 1),
        StoreImm8 { dst: FO(tmp), imm: 222 },
        MicroOp::enum_store8(FO(e), 0, FO(tmp)),
        MicroOp::enum_get_tag(FO(e), FO(result)),
        MicroOp::enum_load8(FO(e), 0, FO(tmp)),
        AddU64 { dst: FO(result), lhs: FO(result), rhs: FO(tmp) },
        Return,
    ]);
    let pointer_offsets = arena.alloc_slice_fill_iter(vec![FO(e)]);

    let functions = [arena.alloc(Function {
        name: GlobalArenaPtr::from_static("test"),
        code,
        args_size: 0,
        args_and_locals_size: 24,
        extended_frame_size: 48,
        zero_frame: true,
        pointer_offsets,
    })];
    let descriptors = vec![ObjectDescriptor::Enum {
        size: 16,
        variant_pointer_offsets: vec![vec![], vec![]],
    }];
    let mut ctx = InterpreterContext::new(&descriptors, unsafe { functions[0].as_ref_unchecked() });
    ctx.run().unwrap();

    assert_eq!(ctx.root_result(), 223, "result should be 1 + 222 = 223");
}

// ---------------------------------------------------------------------------
// Test 6: enum_borrow_field
// ---------------------------------------------------------------------------

#[test]
fn enum_borrow_field() {
    use MicroOp::*;

    let result: u32 = 0;
    let e: u32 = 8;
    let r#ref: u32 = 16;
    let e_ref: u32 = 32; // 16-byte fat pointer ref to e (for enum_borrow)

    let arena = ExecutableArena::new();

    #[rustfmt::skip]
    let code = arena.alloc_slice_fill_iter(vec![
        HeapNew { dst: FO(e), descriptor_id: DescriptorId(0) },
        MicroOp::enum_set_tag(FO(e), 0),
        StoreImm8 { dst: FO(result), imm: 10 },
        MicroOp::enum_store8(FO(e), 0, FO(result)),
        StoreImm8 { dst: FO(result), imm: 20 },
        MicroOp::enum_store8(FO(e), 8, FO(result)),
        SlotBorrow { dst: FO(e_ref), local: FO(e) },
        MicroOp::enum_borrow(FO(e_ref), 8, FO(r#ref)),
        ReadRef { dst: FO(result), ref_ptr: FO(r#ref), size: 8 },
        StoreImm8 { dst: FO(result), imm: 99 },
        WriteRef { ref_ptr: FO(r#ref), src: FO(result), size: 8 },
        MicroOp::enum_load8(FO(e), 8, FO(result)),
        Return,
    ]);
    let pointer_offsets = arena.alloc_slice_fill_iter(vec![FO(e), FO(r#ref), FO(e_ref)]);

    let functions = [arena.alloc(Function {
        name: GlobalArenaPtr::from_static("test"),
        code,
        args_size: 0,
        args_and_locals_size: 48,
        extended_frame_size: 72,
        zero_frame: true,
        pointer_offsets,
    })];
    let descriptors = vec![ObjectDescriptor::Enum {
        size: 24,
        variant_pointer_offsets: vec![vec![], vec![]],
    }];
    let mut ctx = InterpreterContext::new(&descriptors, unsafe { functions[0].as_ref_unchecked() });
    ctx.run().unwrap();

    assert_eq!(ctx.root_result(), 99, "field_b should be 99 after WriteRef");
}

// ---------------------------------------------------------------------------
// Test 7: enum_gc_variant_switching
// ---------------------------------------------------------------------------

#[test]
fn enum_gc_variant_switching() {
    use MicroOp::*;

    let result: u32 = 0;
    let ctr: u32 = 8;
    let vec: u32 = 16;
    let tmp: u32 = 24;
    let vec_ref: u32 = 32;

    let arena = ExecutableArena::new();

    #[rustfmt::skip]
    let code = arena.alloc_slice_fill_iter(vec![
        VecNew { dst: FO(vec) },
        SlotBorrow { dst: FO(vec_ref), local: FO(vec) },
        StoreImm8 { dst: FO(tmp), imm: 100 },
        VecPushBack { vec_ref: FO(vec_ref), elem: FO(tmp), elem_size: 8, descriptor_id: DescriptorId(1) },
        HeapNew { dst: FO(ctr), descriptor_id: DescriptorId(0) },
        MicroOp::enum_set_tag(FO(ctr), 1),
        MicroOp::enum_store8(FO(ctr), 0, FO(vec)),
        ForceGC,
        MicroOp::enum_load8(FO(ctr), 0, FO(vec)),
        StoreImm8 { dst: FO(tmp), imm: 0 },
        VecLoadElem { dst: FO(result), vec_ref: FO(vec_ref), idx: FO(tmp), elem_size: 8 },
        MicroOp::enum_set_tag(FO(ctr), 0),
        StoreImm8 { dst: FO(tmp), imm: 0 },
        MicroOp::enum_store8(FO(ctr), 0, FO(tmp)),
        ForceGC,
        MicroOp::enum_get_tag(FO(ctr), FO(result)),
        Return,
    ]);
    let pointer_offsets = arena.alloc_slice_fill_iter(vec![FO(ctr), FO(vec), FO(vec_ref)]);

    let functions = [arena.alloc(Function {
        name: GlobalArenaPtr::from_static("test"),
        code,
        args_size: 0,
        args_and_locals_size: 48,
        extended_frame_size: 72,
        zero_frame: true,
        pointer_offsets,
    })];
    let descriptors = vec![
        ObjectDescriptor::Enum {
            size: 16,
            variant_pointer_offsets: vec![vec![], vec![0]],
        },
        ObjectDescriptor::Trivial,
    ];
    let mut ctx = InterpreterContext::new(&descriptors, unsafe { functions[0].as_ref_unchecked() });
    ctx.run().unwrap();

    assert_eq!(
        ctx.root_result(),
        0,
        "tag should be 0 (Empty) after variant switch + GC"
    );
    assert_eq!(ctx.gc_count(), 2);
}

// ---------------------------------------------------------------------------
// Test 8: enum_in_struct
// ---------------------------------------------------------------------------

#[test]
fn enum_in_struct() {
    use MicroOp::*;

    let result: u32 = 0;
    let wrapper: u32 = 8;
    let payload: u32 = 16;
    let tmp: u32 = 24;

    let arena = ExecutableArena::new();

    #[rustfmt::skip]
    let code = arena.alloc_slice_fill_iter(vec![
        HeapNew { dst: FO(payload), descriptor_id: DescriptorId(1) },
        MicroOp::enum_set_tag(FO(payload), 1),
        StoreImm8 { dst: FO(tmp), imm: 42 },
        MicroOp::enum_store8(FO(payload), 0, FO(tmp)),
        HeapNew { dst: FO(wrapper), descriptor_id: DescriptorId(0) },
        StoreImm8 { dst: FO(tmp), imm: 7 },
        MicroOp::struct_store8(FO(wrapper), 0, FO(tmp)),
        MicroOp::struct_store8(FO(wrapper), 8, FO(payload)),
        ForceGC,
        MicroOp::struct_load8(FO(wrapper), 0, FO(result)),
        MicroOp::struct_load8(FO(wrapper), 8, FO(payload)),
        MicroOp::enum_get_tag(FO(payload), FO(tmp)),
        MicroOp::enum_load8(FO(payload), 0, FO(tmp)),
        AddU64 { dst: FO(result), lhs: FO(result), rhs: FO(tmp) },
        Return,
    ]);
    let pointer_offsets = arena.alloc_slice_fill_iter(vec![FO(wrapper), FO(payload)]);

    let functions = [arena.alloc(Function {
        name: GlobalArenaPtr::from_static("test"),
        code,
        args_size: 0,
        args_and_locals_size: 32,
        extended_frame_size: 56,
        zero_frame: true,
        pointer_offsets,
    })];
    let descriptors = vec![
        ObjectDescriptor::Struct {
            size: 16,
            pointer_offsets: vec![8],
        },
        ObjectDescriptor::Enum {
            size: 16,
            variant_pointer_offsets: vec![vec![], vec![]],
        },
    ];
    let mut ctx = InterpreterContext::new(&descriptors, unsafe { functions[0].as_ref_unchecked() });
    ctx.run().unwrap();

    assert_eq!(
        ctx.root_result(),
        49,
        "result should be 7 + 42 = 49 after GC"
    );
    assert_eq!(ctx.gc_count(), 1);
}

// ---------------------------------------------------------------------------
// Test 9: enum_in_vector
// ---------------------------------------------------------------------------

#[test]
fn enum_in_vector() {
    use MicroOp::*;

    let result: u32 = 0;
    let vec: u32 = 8;
    let e: u32 = 16;
    let tmp: u32 = 24;
    let vec_ref: u32 = 32;

    let arena = ExecutableArena::new();

    #[rustfmt::skip]
    let code = arena.alloc_slice_fill_iter(vec![
        VecNew { dst: FO(vec) },
        SlotBorrow { dst: FO(vec_ref), local: FO(vec) },
        HeapNew { dst: FO(e), descriptor_id: DescriptorId(0) },
        MicroOp::enum_set_tag(FO(e), 0),
        StoreImm8 { dst: FO(tmp), imm: 10 },
        MicroOp::enum_store8(FO(e), 0, FO(tmp)),
        VecPushBack { vec_ref: FO(vec_ref), elem: FO(e), elem_size: 8, descriptor_id: DescriptorId(1) },
        HeapNew { dst: FO(e), descriptor_id: DescriptorId(0) },
        MicroOp::enum_set_tag(FO(e), 1),
        StoreImm8 { dst: FO(tmp), imm: 30 },
        MicroOp::enum_store8(FO(e), 0, FO(tmp)),
        StoreImm8 { dst: FO(tmp), imm: 40 },
        MicroOp::enum_store8(FO(e), 8, FO(tmp)),
        VecPushBack { vec_ref: FO(vec_ref), elem: FO(e), elem_size: 8, descriptor_id: DescriptorId(1) },
        ForceGC,
        StoreImm8 { dst: FO(tmp), imm: 0 },
        VecLoadElem { dst: FO(e), vec_ref: FO(vec_ref), idx: FO(tmp), elem_size: 8 },
        MicroOp::enum_load8(FO(e), 0, FO(result)),
        StoreImm8 { dst: FO(tmp), imm: 1 },
        VecLoadElem { dst: FO(e), vec_ref: FO(vec_ref), idx: FO(tmp), elem_size: 8 },
        MicroOp::enum_load8(FO(e), 0, FO(tmp)),
        AddU64 { dst: FO(result), lhs: FO(result), rhs: FO(tmp) },
        MicroOp::enum_load8(FO(e), 8, FO(tmp)),
        AddU64 { dst: FO(result), lhs: FO(result), rhs: FO(tmp) },
        Return,
    ]);
    let pointer_offsets = arena.alloc_slice_fill_iter(vec![FO(vec), FO(e), FO(vec_ref)]);

    let functions = [arena.alloc(Function {
        name: GlobalArenaPtr::from_static("test"),
        code,
        args_size: 0,
        args_and_locals_size: 48,
        extended_frame_size: 72,
        zero_frame: true,
        pointer_offsets,
    })];
    let descriptors = vec![
        ObjectDescriptor::Enum {
            size: 24,
            variant_pointer_offsets: vec![vec![], vec![]],
        },
        ObjectDescriptor::Vector {
            elem_size: 8,
            elem_pointer_offsets: vec![0],
        },
    ];
    let mut ctx = InterpreterContext::new(&descriptors, unsafe { functions[0].as_ref_unchecked() });
    ctx.run().unwrap();

    assert_eq!(
        ctx.root_result(),
        80,
        "result should be 10 + 30 + 40 = 80 after GC"
    );
    assert_eq!(ctx.gc_count(), 1);
}
