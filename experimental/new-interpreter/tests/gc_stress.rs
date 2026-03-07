// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! GC stress test with a nested object graph: `vector<*Entry>` where
//! `Entry = { key: u64, values: vector<u64> }` is a heap-allocated struct.
//!
//! This exercises GC tracing through three levels of indirection:
//!   stack root → outer vector → Entry struct → inner vector
//!
//! Maintains a persistent `outer_vec`. Each iteration:
//!   1. Generates `r1`, dispatches on `r1 % 100`:
//!      - 0..30  (30%): push or replace — generates `val`, creates entry;
//!        if full, also generates an index random for replace.
//!      - 30..45 (15%): pop (if non-empty).
//!      - 45..   (55%): create garbage — generates `val`, allocates entry,
//!        then discards it.
//!
//! Different branches consume different numbers of random values, matching
//! the Rust simulation exactly.
//!
//! At the end, walks the VM's outer vector via heap pointers and compares
//! element-by-element against a pure-Rust simulation using the same seed.

use aptos_experimental_new_interpreter::{
    interpreter::InterpreterContext, read_ptr, read_u64, Function, Instruction, ObjectDescriptor,
    STRUCT_DATA_OFFSET, VEC_DATA_OFFSET, VEC_LENGTH_OFFSET,
};
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Rust-side simulation (ground truth)
// ---------------------------------------------------------------------------

fn simulate(n: u64, max_len: u64, seed: u64) -> Vec<(u64, Vec<u64>)> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut outer: Vec<(u64, Vec<u64>)> = Vec::new();

    for _ in 0..n {
        let r1: u64 = rng.r#gen();

        match r1 % 100 {
            0..30 => {
                let val: u64 = rng.r#gen();
                let entry = (val, vec![val]);

                if outer.len() as u64 >= max_len {
                    let idx = rng.r#gen::<u64>() as usize % outer.len();
                    outer[idx] = entry;
                } else {
                    outer.push(entry);
                }
            },
            30..45 => {
                if !outer.is_empty() {
                    outer.pop();
                }
            },
            45.. => {
                let val: u64 = rng.r#gen();
                let _entry = (val, vec![val]);
            },
        }
    }

    outer
}

// ---------------------------------------------------------------------------
// Bytecode program builder
// ---------------------------------------------------------------------------

/// Function 0 (main) frame layout (96 bytes):
///   [fp +  0] : outer_vec     (ptr, descriptor 2 — vector of Entry pointers)
///   [fp +  8] : i             (loop counter)
///   [fp + 16] : r1            (scratch: action random, val, idx_raw)
///   [fp + 24] : len           (outer_vec length, computed per-branch)
///   [fp + 32] : tmp           (scratch for % results, indices)
///   [fp + 40] : const_hundred (= 100)
///   [fp + 48] : call-site metadata (24 bytes: saved_pc, saved_fp, saved_func_id)
///   [fp + 72] : callee arg: val       (= callee fp + 0)
///   [fp + 80] : callee scratch: vec   (= callee fp + 8)
///   [fp + 88] : callee result: entry  (= callee fp + 16)
///
/// Function 1 (make_entry) data segment (24 bytes):
///   [fp +  0] : val    (argument from caller)
///   [fp +  8] : vec    (inner vector, scratch)
///   [fp + 16] : entry  (result: heap pointer to Entry struct)
fn make_gc_stress_program(
    num_iterations: u64,
    max_len: u64,
) -> (Vec<Function>, Vec<ObjectDescriptor>) {
    use Instruction::*;

    // -- Function 1: make_entry(val) -> entry_ptr --
    let callee_val: u32 = 0;
    let callee_vec: u32 = 8;
    let callee_entry: u32 = 16;

    #[rustfmt::skip]
    let make_entry_code = vec![
        // PC 0: vec = VecNew(descriptor=0, elem_size=8)
        VecNew { descriptor_id: 0, elem_size: 8, initial_capacity: 4, dst_fp_offset: callee_vec },
        // PC 1: VecPushBack(vec, val)
        VecPushBack { vec_fp_offset: callee_vec, elem_fp_offset: callee_val, elem_size: 8 },
        // PC 2: entry = StructNew(descriptor=1)
        StructNew { descriptor_id: 1, dst_fp_offset: callee_entry },
        // PC 3: entry.key = val
        StructStoreField { struct_fp_offset: callee_entry, field_offset: 0, src_fp_offset: callee_val, size: 8 },
        // PC 4: entry.values = vec
        StructStoreField { struct_fp_offset: callee_entry, field_offset: 8, src_fp_offset: callee_vec, size: 8 },
        // PC 5: return
        Return,
    ];

    let mut callee_stack_maps = HashMap::new();
    callee_stack_maps.insert(0, vec![]);
    callee_stack_maps.insert(1, vec![callee_vec]);
    callee_stack_maps.insert(2, vec![callee_vec]);

    let callee_func = Function {
        code: make_entry_code,
        data_size: 24,
        extended_frame_size: 48,
        stack_maps: callee_stack_maps,
    };

    // -- Function 0: main --
    let outer_vec: u32 = 0;
    let i: u32 = 8;
    let r1: u32 = 16;
    let len: u32 = 24;
    let tmp: u32 = 32;
    let const_hundred: u32 = 40;
    let callee_arg: u32 = 72; // data_size (48) + FRAME_METADATA_SIZE (24)
    let entry_ptr: u32 = 88; // callee result slot (callee fp + 16)

    #[rustfmt::skip]
    let code = vec![
        // ---- Setup ----
        // PC 0: outer_vec = VecNew(descriptor=2, elem_size=8)
        VecNew { descriptor_id: 2, elem_size: 8, initial_capacity: 4, dst_fp_offset: outer_vec },
        // PC 1: i = 0
        StoreU64 { dst_fp_offset: i, val: 0 },
        // PC 2: const_hundred = 100
        StoreU64 { dst_fp_offset: const_hundred, val: 100 },

        // ---- LOOP (PC 3) ----
        JumpIfGreaterEqualU64Const { src_fp_offset: i, dst_pc: 29, val: num_iterations },

        // PC 4: r1 = random (action)
        StoreRandomU64 { dst_fp_offset: r1 },
        // PC 5: tmp = r1 % 100
        RemU64 { lhs_fp_offset: r1, rhs_fp_offset: const_hundred, dst_fp_offset: tmp },
        // PC 6: if tmp >= 45: goto GARBAGE (PC 24)
        JumpIfGreaterEqualU64Const { src_fp_offset: tmp, dst_pc: 24, val: 45 },
        // PC 7: if tmp >= 30: goto POP (PC 19)
        JumpIfGreaterEqualU64Const { src_fp_offset: tmp, dst_pc: 19, val: 30 },

        // ---- PUSH_OR_REPLACE (action 0..29) ----
        // PC 8: r1 = random (val)
        StoreRandomU64 { dst_fp_offset: r1 },
        // PC 9: write val to callee's argument slot
        Mov8 { src_fp_offset: r1, dst_fp_offset: callee_arg },
        // PC 10: call make_entry (func 1)
        CallFunc { func_id: 1 },
        // After return: entry pointer is at fp+88 (callee's fp+16)
        // PC 11: len = VecLen(outer_vec)
        VecLen { vec_fp_offset: outer_vec, dst_fp_offset: len },
        // PC 12: if len >= max_len: goto REPLACE (PC 15)
        JumpIfGreaterEqualU64Const { src_fp_offset: len, dst_pc: 15, val: max_len },
        // ---- PUSH (PC 13) ----
        VecPushBack { vec_fp_offset: outer_vec, elem_fp_offset: entry_ptr, elem_size: 8 },
        // PC 14: goto NEXT (PC 27)
        Jump { dst_pc: 27 },

        // ---- REPLACE (PC 15) ----
        // PC 15: r1 = random (idx_raw)
        StoreRandomU64 { dst_fp_offset: r1 },
        // PC 16: tmp = r1 % len
        RemU64 { lhs_fp_offset: r1, rhs_fp_offset: len, dst_fp_offset: tmp },
        // PC 17: outer_vec[tmp] = entry_ptr
        VecStoreElem { vec_fp_offset: outer_vec, idx_fp_offset: tmp, src_fp_offset: entry_ptr, elem_size: 8 },
        // PC 18: goto NEXT (PC 27)
        Jump { dst_pc: 27 },

        // ---- POP (PC 19) ----
        // PC 19: len = VecLen(outer_vec)
        VecLen { vec_fp_offset: outer_vec, dst_fp_offset: len },
        // PC 20: if len > 0: goto DO_POP (PC 22)
        JumpIfNotZero { src_fp_offset: len, dst_pc: 22 },
        // PC 21: len == 0, skip → goto NEXT (PC 27)
        Jump { dst_pc: 27 },
        // PC 22: VecPopBack(outer_vec) — discard into r1
        VecPopBack { vec_fp_offset: outer_vec, dst_fp_offset: r1, elem_size: 8 },
        // PC 23: goto NEXT (PC 27)
        Jump { dst_pc: 27 },

        // ---- GARBAGE (PC 24) ----
        // PC 24: r1 = random (val)
        StoreRandomU64 { dst_fp_offset: r1 },
        // PC 25: write val to callee's argument slot
        Mov8 { src_fp_offset: r1, dst_fp_offset: callee_arg },
        // PC 26: call make_entry (func 1) — result becomes garbage
        CallFunc { func_id: 1 },
        // falls through to NEXT

        // ---- NEXT (PC 27) ----
        AddU64Const { src_fp_offset: i, val: 1, dst_fp_offset: i },
        // PC 28: goto LOOP (PC 3)
        Jump { dst_pc: 3 },

        // ---- DONE (PC 29) ----
        Return,
    ];

    let mut stack_maps = HashMap::new();
    stack_maps.insert(0, vec![]);
    stack_maps.insert(11, vec![outer_vec]);
    stack_maps.insert(13, vec![outer_vec, entry_ptr]);
    stack_maps.insert(27, vec![outer_vec]);

    let main_func = Function {
        code,
        data_size: 48,
        extended_frame_size: 96,
        stack_maps,
    };

    let descriptors = vec![
        // Descriptor 0: trivial — inner vectors hold plain u64 values
        ObjectDescriptor::Trivial,
        // Descriptor 1: Entry struct { key: u64, values: *vec }
        ObjectDescriptor::Struct {
            size: 16,
            ref_offsets: vec![8],
        },
        // Descriptor 2: outer vector whose 8-byte elements are heap pointers (to Entry structs)
        ObjectDescriptor::Vector {
            elem_size: 8,
            elem_ref_offsets: vec![0],
        },
    ];

    (vec![main_func, callee_func], descriptors)
}

// ---------------------------------------------------------------------------
// Read back VM results by walking the heap
// ---------------------------------------------------------------------------

unsafe fn read_vm_outer_vec(outer_ptr: *const u8) -> Vec<(u64, Vec<u64>)> {
    unsafe {
        let outer_len = read_u64(outer_ptr, VEC_LENGTH_OFFSET) as usize;
        let mut result = Vec::with_capacity(outer_len);

        for i in 0..outer_len {
            let entry_ptr = read_ptr(outer_ptr, VEC_DATA_OFFSET + i * 8);
            let key = read_u64(entry_ptr, STRUCT_DATA_OFFSET);
            let values_ptr = read_ptr(entry_ptr, STRUCT_DATA_OFFSET + 8);
            let values_len = read_u64(values_ptr, VEC_LENGTH_OFFSET) as usize;
            let mut values = Vec::with_capacity(values_len);
            for j in 0..values_len {
                values.push(read_u64(values_ptr, VEC_DATA_OFFSET + j * 8));
            }
            result.push((key, values));
        }

        result
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn gc_stress() {
    let n: u64 = 10_000;
    let max_len: u64 = 50;
    let seed: u64 = 12345;

    let expected = simulate(n, max_len, seed);

    let (functions, descriptors) = make_gc_stress_program(n, max_len);
    let mut ctx = InterpreterContext::with_heap_size(&functions, &descriptors, 0, &[], 8 * 1024);
    ctx.set_rng_seed(seed);
    ctx.run().unwrap();

    let outer_ptr = ctx.root_heap_ptr(0);
    let vm_values = unsafe { read_vm_outer_vec(outer_ptr) };

    assert_eq!(
        vm_values.len(),
        expected.len(),
        "outer vector length mismatch: VM={} expected={}",
        vm_values.len(),
        expected.len()
    );
    for (i, (vm_entry, exp_entry)) in vm_values.iter().zip(expected.iter()).enumerate() {
        assert_eq!(
            vm_entry, exp_entry,
            "mismatch at index {}: VM={:?} expected={:?}",
            i, vm_entry, exp_entry
        );
    }

    let gc_runs = ctx.gc_count();
    println!(
        "gc_stress: {gc_runs} GC collections over {n} iterations, outer_vec len={}",
        vm_values.len()
    );
    let flat: Vec<u64> = vm_values.iter().map(|e| e.0).collect();
    println!("final outer_vec keys: {:?}", flat);
    assert!(gc_runs > 0, "expected at least one GC collection");
}
