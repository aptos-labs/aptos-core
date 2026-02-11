// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for Move struct support (both inline and heap-allocated).

use aptos_experimental_new_interpreter::{
    interpreter::InterpreterContext, read_ptr, read_u64, Function, Instruction, ObjectDescriptor,
    STRUCT_DATA_OFFSET, VEC_DATA_OFFSET, VEC_LENGTH_OFFSET,
};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Test 1: struct_inline — inline struct on the stack (no new instructions)
//
// struct Pair { a: u64, b: u64 } laid out at fp+8..fp+24.
// Stores a=10, b=20, computes result = a + b.
// ---------------------------------------------------------------------------

/// Data segment (24 bytes):
///   [fp +  0] : result  (u64)
///   [fp +  8] : pair.a  (u64)
///   [fp + 16] : pair.b  (u64)
#[test]
fn struct_inline() {
    use Instruction::*;

    let result: u32 = 0;
    let pair_a: u32 = 8;
    let pair_b: u32 = 16;

    #[rustfmt::skip]
    let code = vec![
        StoreU64 { dst_fp_offset: pair_a, val: 10 },
        StoreU64 { dst_fp_offset: pair_b, val: 20 },
        AddU64 { src_fp_offset_1: pair_a, src_fp_offset_2: pair_b, dst_fp_offset: result },
        Return,
    ];

    let functions = [Function {
        code,
        data_size: 24,
        extended_frame_size: 48,
        stack_maps: HashMap::new(),
    }];
    let descriptors = vec![ObjectDescriptor::Trivial];
    let mut ctx = InterpreterContext::new(&functions, &descriptors, 0, &[]);
    ctx.run().unwrap();

    assert_eq!(ctx.root_result(), 30, "result should be 10 + 20 = 30");
}

// ---------------------------------------------------------------------------
// Test 2: struct_inline_borrow — borrow an inline struct field
//
// struct Pair { a: u64, b: u64 } at frame offsets 8/16.
// BorrowLocal field b, write 99 through the ref, check it landed.
// ---------------------------------------------------------------------------

/// Data segment (40 bytes):
///   [fp +  0] : result  (u64)
///   [fp +  8] : pair.a  (u64)
///   [fp + 16] : pair.b  (u64)
///   [fp + 24] : ref     (fat pointer, 16 bytes)
#[test]
fn struct_inline_borrow() {
    use Instruction::*;

    let result: u32 = 0;
    let pair_a: u32 = 8;
    let pair_b: u32 = 16;
    let r#ref: u32 = 24;

    #[rustfmt::skip]
    let code = vec![
        StoreU64 { dst_fp_offset: pair_a, val: 10 },
        StoreU64 { dst_fp_offset: pair_b, val: 20 },
        BorrowLocal { local_fp_offset: pair_b, dst_fp_offset: r#ref },
        // Read through ref → should be 20
        ReadRef { ref_fp_offset: r#ref, dst_fp_offset: result, size: 8 },
        // Write 99 through ref
        StoreU64 { dst_fp_offset: result, val: 99 },
        WriteRef { ref_fp_offset: r#ref, src_fp_offset: result, size: 8 },
        // Read pair.b directly into result
        Mov8 { src_fp_offset: pair_b, dst_fp_offset: result },
        Return,
    ];

    let functions = [Function {
        code,
        data_size: 40,
        extended_frame_size: 64,
        stack_maps: HashMap::new(),
    }];
    let descriptors = vec![ObjectDescriptor::Trivial];
    let mut ctx = InterpreterContext::new(&functions, &descriptors, 0, &[]);
    ctx.run().unwrap();

    assert_eq!(ctx.root_result(), 99, "pair.b should be 99 after WriteRef");
}

// ---------------------------------------------------------------------------
// Test 3: struct_heap_basic — allocate and use a heap struct
//
// struct Entry { key: u64, value: u64 } — descriptor with size=16, no refs.
// StructNew, StructStoreField, StructLoadField.
// ---------------------------------------------------------------------------

/// Data segment (24 bytes):
///   [fp +  0] : result  (u64)
///   [fp +  8] : entry   (heap pointer)
///   [fp + 16] : tmp     (u64)
#[test]
fn struct_heap_basic() {
    use Instruction::*;

    let result: u32 = 0;
    let entry: u32 = 8;
    let tmp: u32 = 16;

    #[rustfmt::skip]
    let code = vec![
        // PC 0: allocate Entry
        StructNew { descriptor_id: 0, dst_fp_offset: entry },
        // PC 1-2: entry.key = 42
        StoreU64 { dst_fp_offset: tmp, val: 42 },
        StructStoreField { struct_fp_offset: entry, field_offset: 0, src_fp_offset: tmp, size: 8 },
        // PC 3-4: entry.value = 100
        StoreU64 { dst_fp_offset: tmp, val: 100 },
        StructStoreField { struct_fp_offset: entry, field_offset: 8, src_fp_offset: tmp, size: 8 },
        // PC 5: read entry.key → result
        StructLoadField { struct_fp_offset: entry, field_offset: 0, dst_fp_offset: result, size: 8 },
        // PC 6: read entry.value → tmp
        StructLoadField { struct_fp_offset: entry, field_offset: 8, dst_fp_offset: tmp, size: 8 },
        // PC 7: result = key + value
        AddU64 { src_fp_offset_1: result, src_fp_offset_2: tmp, dst_fp_offset: result },
        Return,
    ];

    let mut stack_maps = HashMap::new();
    stack_maps.insert(0, vec![]);

    let functions = [Function {
        code,
        data_size: 24,
        extended_frame_size: 48,
        stack_maps,
    }];
    let descriptors = vec![ObjectDescriptor::Struct {
        size: 16,
        ref_offsets: vec![],
    }];
    let mut ctx = InterpreterContext::new(&functions, &descriptors, 0, &[]);
    ctx.run().unwrap();

    assert_eq!(ctx.root_result(), 142, "result should be 42 + 100 = 142");
}

// ---------------------------------------------------------------------------
// Test 4: struct_heap_survives_gc — heap struct survives GC
//
// Allocate Entry{key=7, value=13}, force GC, read fields back.
// ---------------------------------------------------------------------------

/// Data segment (24 bytes):
///   [fp +  0] : result  (u64)
///   [fp +  8] : entry   (heap pointer)
///   [fp + 16] : tmp     (u64)
#[test]
fn struct_heap_survives_gc() {
    use Instruction::*;

    let result: u32 = 0;
    let entry: u32 = 8;
    let tmp: u32 = 16;

    #[rustfmt::skip]
    let code = vec![
        // PC 0: allocate
        StructNew { descriptor_id: 0, dst_fp_offset: entry },
        // PC 1-2: entry.key = 7
        StoreU64 { dst_fp_offset: tmp, val: 7 },
        StructStoreField { struct_fp_offset: entry, field_offset: 0, src_fp_offset: tmp, size: 8 },
        // PC 3-4: entry.value = 13
        StoreU64 { dst_fp_offset: tmp, val: 13 },
        StructStoreField { struct_fp_offset: entry, field_offset: 8, src_fp_offset: tmp, size: 8 },
        // PC 5: force GC
        ForceGC,
        // PC 6-7: read back key and value
        StructLoadField { struct_fp_offset: entry, field_offset: 0, dst_fp_offset: result, size: 8 },
        StructLoadField { struct_fp_offset: entry, field_offset: 8, dst_fp_offset: tmp, size: 8 },
        // PC 8: result = key + value
        AddU64 { src_fp_offset_1: result, src_fp_offset_2: tmp, dst_fp_offset: result },
        Return,
    ];

    let mut stack_maps = HashMap::new();
    stack_maps.insert(0, vec![]);
    stack_maps.insert(5, vec![entry]);

    let functions = [Function {
        code,
        data_size: 24,
        extended_frame_size: 48,
        stack_maps,
    }];
    let descriptors = vec![ObjectDescriptor::Struct {
        size: 16,
        ref_offsets: vec![],
    }];
    let mut ctx = InterpreterContext::new(&functions, &descriptors, 0, &[]);
    ctx.run().unwrap();

    assert_eq!(ctx.root_result(), 20, "result should be 7 + 13 = 20 after GC");
    assert_eq!(ctx.gc_count(), 1);
}

// ---------------------------------------------------------------------------
// Test 5: struct_with_vector_field — struct containing a vector pointer
//
// struct Container { tag: u64, items: vector<u64> }
// Descriptor: size=16, ref_offsets=[8] (items is a heap ptr at byte 8).
// Allocates the struct, creates a vector, stores it in the struct, pushes
// elements, forces GC, then reads back via the struct.
// ---------------------------------------------------------------------------

/// Data segment (32 bytes):
///   [fp +  0] : result   (u64)
///   [fp +  8] : ctr      (heap pointer to Container)
///   [fp + 16] : items    (heap pointer to vector, scratch)
///   [fp + 24] : tmp      (u64)
#[test]
fn struct_with_vector_field() {
    use Instruction::*;

    let result: u32 = 0;
    let ctr: u32 = 8;
    let items: u32 = 16;
    let tmp: u32 = 24;

    // Descriptor 0: Struct Container { tag: u64, items: *vec } — ref at offset 8
    // Descriptor 1: Trivial (inner vector holds plain u64s)

    #[rustfmt::skip]
    let code = vec![
        // PC 0: allocate Container
        StructNew { descriptor_id: 0, dst_fp_offset: ctr },
        // PC 1-2: ctr.tag = 999
        StoreU64 { dst_fp_offset: tmp, val: 999 },
        StructStoreField { struct_fp_offset: ctr, field_offset: 0, src_fp_offset: tmp, size: 8 },
        // PC 3: allocate vector<u64>
        VecNew { descriptor_id: 1, elem_size: 8, initial_capacity: 4, dst_fp_offset: items },
        // PC 4-5: push 10, 20, 30
        StoreU64 { dst_fp_offset: tmp, val: 10 },
        VecPushBack { vec_fp_offset: items, elem_fp_offset: tmp, elem_size: 8 },
        StoreU64 { dst_fp_offset: tmp, val: 20 },
        VecPushBack { vec_fp_offset: items, elem_fp_offset: tmp, elem_size: 8 },
        StoreU64 { dst_fp_offset: tmp, val: 30 },
        VecPushBack { vec_fp_offset: items, elem_fp_offset: tmp, elem_size: 8 },
        // PC 10: ctr.items = items
        StructStoreField { struct_fp_offset: ctr, field_offset: 8, src_fp_offset: items, size: 8 },
        // PC 11: ForceGC — GC must trace ctr → items vector
        ForceGC,
        // PC 12: read ctr.items back into items slot
        StructLoadField { struct_fp_offset: ctr, field_offset: 8, dst_fp_offset: items, size: 8 },
        // PC 13: read ctr.tag → result
        StructLoadField { struct_fp_offset: ctr, field_offset: 0, dst_fp_offset: result, size: 8 },
        // PC 14: check vector length
        VecLen { vec_fp_offset: items, dst_fp_offset: tmp },
        Return,
    ];

    let mut stack_maps = HashMap::new();
    stack_maps.insert(0, vec![]);
    stack_maps.insert(3, vec![ctr]);
    stack_maps.insert(5, vec![ctr, items]);
    stack_maps.insert(7, vec![ctr, items]);
    stack_maps.insert(9, vec![ctr, items]);
    stack_maps.insert(11, vec![ctr]);

    let functions = [Function {
        code,
        data_size: 32,
        extended_frame_size: 56,
        stack_maps,
    }];
    let descriptors = vec![
        ObjectDescriptor::Struct {
            size: 16,
            ref_offsets: vec![8],
        },
        ObjectDescriptor::Trivial,
    ];
    let mut ctx = InterpreterContext::new(&functions, &descriptors, 0, &[]);
    ctx.run().unwrap();

    assert_eq!(ctx.root_result(), 999, "ctr.tag should be 999 after GC");

    // Verify the vector contents survived GC via struct tracing
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
// Test 6: struct_borrow_field — borrow a field of a heap struct
//
// Allocate Entry{key=5, value=10}, StructBorrow the value field,
// read/write through the fat pointer.
// ---------------------------------------------------------------------------

/// Data segment (32 bytes):
///   [fp +  0] : result  (u64)
///   [fp +  8] : entry   (heap pointer)
///   [fp + 16] : ref     (fat pointer, 16 bytes)
#[test]
fn struct_borrow_field() {
    use Instruction::*;

    let result: u32 = 0;
    let entry: u32 = 8;
    let r#ref: u32 = 16;

    #[rustfmt::skip]
    let code = vec![
        // PC 0: allocate
        StructNew { descriptor_id: 0, dst_fp_offset: entry },
        // PC 1: entry.key = 5
        StoreU64 { dst_fp_offset: result, val: 5 },
        StructStoreField { struct_fp_offset: entry, field_offset: 0, src_fp_offset: result, size: 8 },
        // PC 3: entry.value = 10
        StoreU64 { dst_fp_offset: result, val: 10 },
        StructStoreField { struct_fp_offset: entry, field_offset: 8, src_fp_offset: result, size: 8 },
        // PC 5: borrow entry.value
        StructBorrow { struct_fp_offset: entry, field_offset: 8, dst_fp_offset: r#ref },
        // PC 6: read through ref → expect 10
        ReadRef { ref_fp_offset: r#ref, dst_fp_offset: result, size: 8 },
        // PC 7-8: write 77 through ref, then read entry.value directly
        StoreU64 { dst_fp_offset: result, val: 77 },
        WriteRef { ref_fp_offset: r#ref, src_fp_offset: result, size: 8 },
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

    assert_eq!(ctx.root_result(), 77, "entry.value should be 77 after WriteRef");
}

// ---------------------------------------------------------------------------
// Test 7: struct_borrow_survives_gc — borrowed struct field survives GC
//
// Allocate Entry{key=100, value=200}, borrow value field, force GC,
// read/write through the reference.
// ---------------------------------------------------------------------------

/// Data segment (32 bytes):
///   [fp +  0] : result    (u64)
///   [fp +  8] : entry     (heap pointer)
///   [fp + 16] : ref       (fat pointer, 16 bytes)
#[test]
fn struct_borrow_survives_gc() {
    use Instruction::*;

    let result: u32 = 0;
    let entry: u32 = 8;
    let r#ref: u32 = 16;
    let ref_base: u32 = 16;

    #[rustfmt::skip]
    let code = vec![
        // PC 0: allocate
        StructNew { descriptor_id: 0, dst_fp_offset: entry },
        // PC 1-2: entry.key = 100
        StoreU64 { dst_fp_offset: result, val: 100 },
        StructStoreField { struct_fp_offset: entry, field_offset: 0, src_fp_offset: result, size: 8 },
        // PC 3-4: entry.value = 200
        StoreU64 { dst_fp_offset: result, val: 200 },
        StructStoreField { struct_fp_offset: entry, field_offset: 8, src_fp_offset: result, size: 8 },
        // PC 5: borrow entry.value
        StructBorrow { struct_fp_offset: entry, field_offset: 8, dst_fp_offset: r#ref },
        // PC 6: ForceGC — entry is relocated, ref's base must be updated
        ForceGC,
        // PC 7: read through ref → should still be 200
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

    assert_eq!(ctx.root_result(), 200, "entry.value should be 200 after GC");
    assert_eq!(ctx.gc_count(), 1);

    // Also verify entry.key survived
    let entry_ptr = ctx.root_heap_ptr(8);
    let key = unsafe { read_u64(entry_ptr, STRUCT_DATA_OFFSET) };
    assert_eq!(key, 100, "entry.key should be 100 after GC");
}
