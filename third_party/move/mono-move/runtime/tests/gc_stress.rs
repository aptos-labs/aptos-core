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

use mono_move_alloc::{ExecutableArena, ExecutableArenaPtr, GlobalArenaPtr};
use mono_move_core::{
    CodeOffset as CO, DescriptorId, FrameLayoutInfo, FrameOffset as FO, Function, MicroOp,
    SortedSafePointEntries, STRUCT_DATA_OFFSET,
};
use mono_move_runtime::{
    read_ptr, read_u64, InterpreterContext, ObjectDescriptor, VEC_DATA_OFFSET, VEC_LENGTH_OFFSET,
};
use rand::{rngs::StdRng, Rng, SeedableRng};

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

/// Function 0 (main) frame layout (128 bytes):
///   [fp +  0] : outer_vec     (ptr, descriptor 2 — vector of Entry pointers)
///   [fp +  8] : i             (loop counter)
///   [fp + 16] : r1            (scratch: action random, val, idx_raw)
///   [fp + 24] : len           (outer_vec length, computed per-branch)
///   [fp + 32] : tmp           (scratch for % results, indices)
///   [fp + 40] : const_hundred (= 100)
///   [fp + 48] : outer_vec_ref (16-byte fat pointer to outer_vec slot)
///   [fp + 64] : call-site metadata (24 bytes: saved_pc, saved_fp, saved_func_id)
///   [fp + 88] : callee arg: val       (= callee fp + 0)
///   [fp + 96] : callee scratch: vec   (= callee fp + 8)
///   [fp +104] : callee result: entry  (= callee fp + 16)
///   [fp +112] : callee scratch: vec_ref (= callee fp + 24, 16 bytes)
///
/// Function 1 (make_entry) data segment (40 bytes):
///   [fp +  0] : val      (argument from caller)
///   [fp +  8] : vec      (inner vector, scratch)
///   [fp + 16] : entry    (result: heap pointer to Entry struct)
///   [fp + 24] : vec_ref  (16-byte fat pointer to vec slot)
fn make_gc_stress_program(
    arena: &ExecutableArena,
    num_iterations: u64,
    max_len: u64,
) -> (
    Vec<Option<ExecutableArenaPtr<Function>>>,
    Vec<ObjectDescriptor>,
) {
    use MicroOp::*;

    // -- Function 1: make_entry(val) -> entry_ptr --
    let callee_val: u32 = 0;
    let callee_vec: u32 = 8;
    let callee_entry: u32 = 16;
    let callee_vec_ref: u32 = 24;

    #[rustfmt::skip]
    let make_entry_code = arena.alloc_slice_fill_iter(vec![
        // PC 0: vec = VecNew(descriptor=0, elem_size=8)
        VecNew { dst: FO(callee_vec) },
        // PC 1: vec_ref = SlotBorrow(vec)
        SlotBorrow { dst: FO(callee_vec_ref), local: FO(callee_vec) },
        // PC 2: VecPushBack(vec_ref, val)
        VecPushBack { vec_ref: FO(callee_vec_ref), elem: FO(callee_val), elem_size: 8, descriptor_id: DescriptorId(0) },
        // PC 3: entry = HeapNew(descriptor=1)
        HeapNew { dst: FO(callee_entry), descriptor_id: DescriptorId(1) },
        // PC 4: entry.key = val
        MicroOp::struct_store8(FO(callee_entry), 0, FO(callee_val)),
        // PC 5: entry.values = vec
        MicroOp::struct_store8(FO(callee_entry), 8, FO(callee_vec)),
        // PC 6: return
        Return,
    ]);
    let callee_func = arena.alloc(Function {
        name: GlobalArenaPtr::from_static("test"),
        code: make_entry_code,
        args_size: 8,
        args_and_locals_size: 40,
        extended_frame_size: 64,
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(arena, vec![
            FO(callee_vec),
            FO(callee_entry),
            FO(callee_vec_ref),
        ]),
        safe_point_layouts: SortedSafePointEntries::empty(arena),
    });

    // -- Function 0: main --
    let outer_vec: u32 = 0;
    let i: u32 = 8;
    let r1: u32 = 16;
    let len: u32 = 24;
    let tmp: u32 = 32;
    let const_hundred: u32 = 40;
    let outer_vec_ref: u32 = 48; // 16-byte fat pointer to outer_vec slot
    let callee_arg: u32 = 88; // args_and_locals_size (64) + FRAME_METADATA_SIZE (24)
    let entry_ptr: u32 = 104; // callee result slot (callee fp + 16)

    #[rustfmt::skip]
    let code = arena.alloc_slice_fill_iter(vec![
        // ---- Setup ----
        // PC 0: outer_vec = VecNew(descriptor=2, elem_size=8)
        VecNew { dst: FO(outer_vec) },
        // PC 1: outer_vec_ref = SlotBorrow(outer_vec)
        SlotBorrow { dst: FO(outer_vec_ref), local: FO(outer_vec) },
        // PC 2: i = 0
        StoreImm8 { dst: FO(i), imm: 0 },
        // PC 3: const_hundred = 100
        StoreImm8 { dst: FO(const_hundred), imm: 100 },

        // ---- LOOP (PC 4) ----
        JumpGreaterEqualU64Imm { target: CO(30), src: FO(i), imm: num_iterations },

        // PC 5: r1 = random (action)
        StoreRandomU64 { dst: FO(r1) },
        // PC 6: tmp = r1 % 100
        ModU64 { dst: FO(tmp), lhs: FO(r1), rhs: FO(const_hundred) },
        // PC 7: if tmp >= 45: goto GARBAGE (PC 25)
        JumpGreaterEqualU64Imm { target: CO(25), src: FO(tmp), imm: 45 },
        // PC 8: if tmp >= 30: goto POP (PC 20)
        JumpGreaterEqualU64Imm { target: CO(20), src: FO(tmp), imm: 30 },

        // ---- PUSH_OR_REPLACE (action 0..29) ----
        // PC 9: r1 = random (val)
        StoreRandomU64 { dst: FO(r1) },
        // PC 10: write val to callee's argument slot
        Move8 { dst: FO(callee_arg), src: FO(r1) },
        // PC 11: call make_entry (func 1)
        CallFunc { func_id: 1 },
        // After return: entry pointer is at fp+104 (callee's fp+16)
        // PC 12: len = VecLen(outer_vec_ref)
        VecLen { dst: FO(len), vec_ref: FO(outer_vec_ref) },
        // PC 13: if len >= max_len: goto REPLACE (PC 16)
        JumpGreaterEqualU64Imm { target: CO(16), src: FO(len), imm: max_len },
        // ---- PUSH (PC 14) ----
        VecPushBack { vec_ref: FO(outer_vec_ref), elem: FO(entry_ptr), elem_size: 8, descriptor_id: DescriptorId(2) },
        // PC 15: goto NEXT (PC 28)
        Jump { target: CO(28) },

        // ---- REPLACE (PC 16) ----
        // PC 16: r1 = random (idx_raw)
        StoreRandomU64 { dst: FO(r1) },
        // PC 17: tmp = r1 % len
        ModU64 { dst: FO(tmp), lhs: FO(r1), rhs: FO(len) },
        // PC 18: outer_vec[tmp] = entry_ptr
        VecStoreElem { vec_ref: FO(outer_vec_ref), idx: FO(tmp), src: FO(entry_ptr), elem_size: 8 },
        // PC 19: goto NEXT (PC 28)
        Jump { target: CO(28) },

        // ---- POP (PC 20) ----
        // PC 20: len = VecLen(outer_vec_ref)
        VecLen { dst: FO(len), vec_ref: FO(outer_vec_ref) },
        // PC 21: if len > 0: goto DO_POP (PC 23)
        JumpNotZeroU64 { target: CO(23), src: FO(len) },
        // PC 22: len == 0, skip → goto NEXT (PC 28)
        Jump { target: CO(28) },
        // PC 23: VecPopBack(outer_vec_ref) — discard into r1
        VecPopBack { dst: FO(r1), vec_ref: FO(outer_vec_ref), elem_size: 8 },
        // PC 24: goto NEXT (PC 28)
        Jump { target: CO(28) },

        // ---- GARBAGE (PC 25) ----
        // PC 25: r1 = random (val)
        StoreRandomU64 { dst: FO(r1) },
        // PC 26: write val to callee's argument slot
        Move8 { dst: FO(callee_arg), src: FO(r1) },
        // PC 27: call make_entry (func 1) — result becomes garbage
        CallFunc { func_id: 1 },
        // falls through to NEXT

        // ---- NEXT (PC 28) ----
        AddU64Imm { dst: FO(i), src: FO(i), imm: 1 },
        // PC 29: goto LOOP (PC 4)
        Jump { target: CO(4) },

        // ---- DONE (PC 30) ----
        Return,
    ]);
    let main_func = arena.alloc(Function {
        name: GlobalArenaPtr::from_static("test"),
        code,
        args_size: 0,
        args_and_locals_size: 64,
        extended_frame_size: 128,
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(arena, vec![
            FO(outer_vec),
            FO(outer_vec_ref),
            FO(entry_ptr),
        ]),
        safe_point_layouts: SortedSafePointEntries::empty(arena),
    });

    let descriptors = vec![
        // Descriptor 0: trivial — inner vectors hold plain u64 values
        ObjectDescriptor::Trivial,
        // Descriptor 1: Entry struct { key: u64, values: *vec }
        ObjectDescriptor::Struct {
            size: 16,
            pointer_offsets: vec![8],
        },
        // Descriptor 2: outer vector whose 8-byte elements are heap pointers (to Entry structs)
        ObjectDescriptor::Vector {
            elem_size: 8,
            elem_pointer_offsets: vec![0],
        },
    ];

    (vec![Some(main_func), Some(callee_func)], descriptors)
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

    let arena = ExecutableArena::new();
    let (functions, descriptors) = make_gc_stress_program(&arena, n, max_len);
    // SAFETY: Exclusive access during test setup; arena is alive.
    unsafe { Function::resolve_calls(&functions) };
    let mut ctx = InterpreterContext::with_heap_size(
        &descriptors,
        unsafe { functions[0].unwrap().as_ref_unchecked() },
        8 * 1024,
    );
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
