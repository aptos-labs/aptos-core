// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Merge sort — recursive, O(n log n) with temp-vector merge.
//!
//! Exercises function calls, loops, and vector operations. A good
//! complement to `fib` (call overhead) and `nested_loop` (loop overhead).

use rand::{rngs::StdRng, Rng, SeedableRng};

/// Fisher-Yates shuffle of [0, 1, ..., n-1] using a seeded RNG.
pub fn shuffled_range(n: u64, seed: u64) -> Vec<u64> {
    let mut values: Vec<u64> = (0..n).collect();
    let mut rng = StdRng::seed_from_u64(seed);
    for i in (1..n as usize).rev() {
        let j = rng.gen_range(0, i + 1);
        values.swap(i, j);
    }
    values
}

// ---------------------------------------------------------------------------
// Native Rust
// ---------------------------------------------------------------------------

pub fn native_merge_sort(v: &mut [u64]) {
    let len = v.len();
    if len <= 1 {
        return;
    }
    let mid = len / 2;
    native_merge_sort(&mut v[..mid]);
    native_merge_sort(&mut v[mid..]);

    let mut tmp = Vec::with_capacity(len);
    let (mut i, mut j) = (0, mid);
    while i < mid && j < len {
        if v[i] <= v[j] {
            tmp.push(v[i]);
            i += 1;
        } else {
            tmp.push(v[j]);
            j += 1;
        }
    }
    tmp.extend_from_slice(&v[i..mid]);
    tmp.extend_from_slice(&v[j..len]);
    v.copy_from_slice(&tmp);
}

// ---------------------------------------------------------------------------
// Micro-op
// ---------------------------------------------------------------------------

/// Three functions:
///   func 0 (merge_sort)       — entry point: merge_sort(vec)
///   func 1 (merge_sort_range) — recursive sort on vec[lo..hi)
///   func 2 (merge)            — merge two sorted halves via temp vec
///
/// See `runtime/tests/merge_sort.rs` for the detailed frame layouts
/// and pseudocode.
#[cfg(feature = "micro-op")]
mod micro_op {
    use mono_move_alloc::{ExecutableArena, ExecutableArenaPtr, GlobalArenaPtr};
    use mono_move_core::{
        CodeOffset as CO, DescriptorId, FrameLayoutInfo, FrameOffset as FO, Function, MicroOp::*,
        SortedSafePointEntries, FRAME_METADATA_SIZE,
    };
    use mono_move_runtime::ObjectDescriptor;

    pub fn program() -> (
        Vec<Option<ExecutableArenaPtr<Function>>>,
        Vec<ObjectDescriptor>,
        ExecutableArena,
    ) {
        let arena = ExecutableArena::new();
        let meta = FRAME_METADATA_SIZE as u32;

        // =================================================================
        // Function 0 — merge_sort(vec)
        //
        // Pseudocode:
        //   let len = vec.len();
        //   merge_sort_range(vec, 0, len);
        //
        // Frame layout:
        //   [0]  vec  [8]  len  [16] vec_ref (16 bytes)
        //   [32] metadata (24 bytes)
        //   [56] callee: vec  [64] callee: lo  [72] callee: hi
        // =================================================================
        let func_merge_sort = {
            let vec = 0u32;
            let len = 8u32;
            let vec_ref = 16u32;
            let param_and_local_sizes_sum = 32u32;
            let callee_vec = param_and_local_sizes_sum + meta;
            let callee_lo = callee_vec + 8;
            let callee_hi = callee_lo + 8;

            #[rustfmt::skip]
            let code = [
                SlotBorrow { dst: FO(vec_ref), local: FO(vec) },
                VecLen { dst: FO(len), vec_ref: FO(vec_ref) },
                Move8 { dst: FO(callee_vec), src: FO(vec) },
                StoreImm8 { dst: FO(callee_lo), imm: 0 },
                Move8 { dst: FO(callee_hi), src: FO(len) },
                CallFunc { func_id: 1 },
                Return,
            ];

            let code = arena.alloc_slice_fill_iter(code);
            arena.alloc(Function {
                name: GlobalArenaPtr::from_static("merge_sort"),
                code,
                param_sizes: ExecutableArenaPtr::empty_slice(),
                param_sizes_sum: 8,
                param_and_local_sizes_sum: param_and_local_sizes_sum as usize,
                extended_frame_size: (callee_hi + 8) as usize,
                zero_frame: true,
                frame_layout: FrameLayoutInfo::new(&arena, [FO(vec), FO(vec_ref)]),
                safe_point_layouts: SortedSafePointEntries::empty(),
            })
        };

        // =================================================================
        // Function 1 — merge_sort_range(vec, lo, hi)
        //
        // Pseudocode:
        //   if hi - lo <= 1 { return; }
        //   let mid = (lo + hi) / 2;
        //   merge_sort_range(vec, lo, mid);
        //   merge_sort_range(vec, mid, hi);
        //   merge(vec, lo, mid, hi);
        //
        // Frame layout:
        //   [0]  vec  [8]  lo  [16] hi  [24] mid  [32] tmp
        //   [40] metadata (24 bytes)
        //   [64] callee_0  [72] callee_1  [80] callee_2  [88] callee_3
        // =================================================================
        let func_merge_sort_range = {
            let vec = 0u32;
            let lo = 8u32;
            let hi = 16u32;
            let mid = 24u32;
            let tmp = 32u32;
            let param_and_local_sizes_sum = 40u32;
            let callee_0 = param_and_local_sizes_sum + meta;
            let callee_1 = callee_0 + 8;
            let callee_2 = callee_1 + 8;
            let callee_3 = callee_2 + 8;

            #[rustfmt::skip]
            let code = [
                // if lo + 1 < hi, continue; else return
                AddU64Imm { dst: FO(tmp), src: FO(lo), imm: 1 },
                JumpLessU64 { target: CO(3), lhs: FO(tmp), rhs: FO(hi) },
                Return,
                // mid = (lo + hi) / 2
                AddU64 { dst: FO(mid), lhs: FO(lo), rhs: FO(hi) },
                ShrU64Imm { dst: FO(mid), src: FO(mid), imm: 1 },
                // merge_sort_range(vec, lo, mid)
                Move8 { dst: FO(callee_0), src: FO(vec) },
                Move8 { dst: FO(callee_1), src: FO(lo) },
                Move8 { dst: FO(callee_2), src: FO(mid) },
                CallFunc { func_id: 1 },
                // merge_sort_range(vec, mid, hi)
                Move8 { dst: FO(callee_0), src: FO(vec) },
                Move8 { dst: FO(callee_1), src: FO(mid) },
                Move8 { dst: FO(callee_2), src: FO(hi) },
                CallFunc { func_id: 1 },
                // merge(vec, lo, mid, hi)
                Move8 { dst: FO(callee_0), src: FO(vec) },
                Move8 { dst: FO(callee_1), src: FO(lo) },
                Move8 { dst: FO(callee_2), src: FO(mid) },
                Move8 { dst: FO(callee_3), src: FO(hi) },
                CallFunc { func_id: 2 },
                Return,
            ];

            let code = arena.alloc_slice_fill_iter(code);
            arena.alloc(Function {
                name: GlobalArenaPtr::from_static("merge_sort_range"),
                code,
                param_sizes: ExecutableArenaPtr::empty_slice(),
                param_sizes_sum: 24,
                param_and_local_sizes_sum: param_and_local_sizes_sum as usize,
                extended_frame_size: (callee_3 + 8) as usize,
                zero_frame: true,
                frame_layout: FrameLayoutInfo::new(&arena, [FO(vec)]),
                safe_point_layouts: SortedSafePointEntries::empty(),
            })
        };

        // =================================================================
        // Function 2 — merge(vec, lo, mid, hi)
        //
        // Pseudocode:
        //   let tmp = [];
        //   let i = lo, j = mid;
        //   while i < mid && j < hi:
        //     if vec[i] < vec[j]: tmp.push(vec[i]); i += 1;
        //     else:               tmp.push(vec[j]); j += 1;
        //   drain remaining from left/right into tmp
        //   copy tmp back into vec[lo..hi)
        //
        // Frame layout:
        //   [0]  vec      [8]  lo       [16] mid      [24] hi
        //   [32] tmp      [40] i        [48] j
        //   [56] elem_a   [64] elem_b   [72] k        [80] tmp_idx
        //   [88] vec_ref (16 bytes)     [104] tmp_ref (16 bytes)
        // =================================================================
        let func_merge = {
            let vec = 0u32;
            let lo = 8u32;
            let mid = 16u32;
            let hi = 24u32;
            let tmp = 32u32;
            let i = 40u32;
            let j = 48u32;
            let elem_a = 56u32;
            let elem_b = 64u32;
            let k = 72u32;
            let tmp_idx = 80u32;
            let vec_ref = 88u32;
            let tmp_ref = 104u32;

            #[rustfmt::skip]
            let code = [
                // i = lo; j = mid; tmp = new vec
                Move8 { dst: FO(i), src: FO(lo) },                              // 0
                Move8 { dst: FO(j), src: FO(mid) },                             // 1
                VecNew { dst: FO(tmp) },                                          // 2
                SlotBorrow { dst: FO(vec_ref), local: FO(vec) },                 // 3
                SlotBorrow { dst: FO(tmp_ref), local: FO(tmp) },                 // 4

                // MERGE_LOOP (5): both halves have elements?
                JumpLessU64 { target: CO(8), lhs: FO(i), rhs: FO(mid) },        // 5
                JumpLessU64 { target: CO(25), lhs: FO(j), rhs: FO(hi) },        // 6: drain right
                Jump { target: CO(31) },                                         // 7: copy back
                JumpLessU64 { target: CO(10), lhs: FO(j), rhs: FO(hi) },        // 8
                Jump { target: CO(19) },                                         // 9: drain left

                // COMPARE (10): both i and j valid
                VecLoadElem { dst: FO(elem_a), vec_ref: FO(vec_ref),
                              idx: FO(i), elem_size: 8 },                       // 10
                VecLoadElem { dst: FO(elem_b), vec_ref: FO(vec_ref),
                              idx: FO(j), elem_size: 8 },                       // 11
                JumpLessU64 { target: CO(16), lhs: FO(elem_a), rhs: FO(elem_b) }, // 12
                // a >= b: push b
                VecPushBack { vec_ref: FO(tmp_ref), elem: FO(elem_b), elem_size: 8, descriptor_id: DescriptorId(0) }, // 13
                AddU64Imm { dst: FO(j), src: FO(j), imm: 1 },                  // 14
                Jump { target: CO(5) },                                          // 15

                // PUSH_LEFT (16): a < b, push a
                VecPushBack { vec_ref: FO(tmp_ref), elem: FO(elem_a), elem_size: 8, descriptor_id: DescriptorId(0) }, // 16
                AddU64Imm { dst: FO(i), src: FO(i), imm: 1 },                  // 17
                Jump { target: CO(5) },                                          // 18

                // DRAIN_LEFT (19): right exhausted
                JumpLessU64 { target: CO(21), lhs: FO(i), rhs: FO(mid) },       // 19
                Jump { target: CO(31) },                                         // 20
                VecLoadElem { dst: FO(elem_a), vec_ref: FO(vec_ref),
                              idx: FO(i), elem_size: 8 },                       // 21
                VecPushBack { vec_ref: FO(tmp_ref), elem: FO(elem_a), elem_size: 8, descriptor_id: DescriptorId(0) }, // 22
                AddU64Imm { dst: FO(i), src: FO(i), imm: 1 },                  // 23
                Jump { target: CO(19) },                                         // 24

                // DRAIN_RIGHT (25): left exhausted
                JumpLessU64 { target: CO(27), lhs: FO(j), rhs: FO(hi) },        // 25
                Jump { target: CO(31) },                                         // 26
                VecLoadElem { dst: FO(elem_b), vec_ref: FO(vec_ref),
                              idx: FO(j), elem_size: 8 },                       // 27
                VecPushBack { vec_ref: FO(tmp_ref), elem: FO(elem_b), elem_size: 8, descriptor_id: DescriptorId(0) }, // 28
                AddU64Imm { dst: FO(j), src: FO(j), imm: 1 },                  // 29
                Jump { target: CO(25) },                                         // 30

                // COPY_BACK (31): copy tmp back into vec[lo..hi)
                Move8 { dst: FO(k), src: FO(lo) },                              // 31
                StoreImm8 { dst: FO(tmp_idx), imm: 0 },                         // 32
                // COPY_LOOP (33)
                JumpLessU64 { target: CO(35), lhs: FO(k), rhs: FO(hi) },        // 33
                Return,                                                          // 34
                VecLoadElem { dst: FO(elem_a), vec_ref: FO(tmp_ref),
                              idx: FO(tmp_idx), elem_size: 8 },                 // 35
                VecStoreElem { vec_ref: FO(vec_ref), idx: FO(k),
                               src: FO(elem_a), elem_size: 8 },                 // 36
                AddU64Imm { dst: FO(k), src: FO(k), imm: 1 },                  // 37
                AddU64Imm { dst: FO(tmp_idx), src: FO(tmp_idx), imm: 1 },      // 38
                Jump { target: CO(33) },                                         // 39
            ];

            let code = arena.alloc_slice_fill_iter(code);
            arena.alloc(Function {
                name: GlobalArenaPtr::from_static("merge"),
                code,
                param_sizes: ExecutableArenaPtr::empty_slice(),
                param_sizes_sum: 32,
                param_and_local_sizes_sum: 120,
                extended_frame_size: 144,
                zero_frame: true,
                frame_layout: FrameLayoutInfo::new(&arena, [
                    FO(vec),
                    FO(tmp),
                    FO(vec_ref),
                    FO(tmp_ref),
                ]),
                safe_point_layouts: SortedSafePointEntries::empty(),
            })
        };

        let descriptors = vec![ObjectDescriptor::Trivial];
        (
            vec![
                Some(func_merge_sort),
                Some(func_merge_sort_range),
                Some(func_merge),
            ],
            descriptors,
            arena,
        )
    }
}

#[cfg(feature = "micro-op")]
pub use micro_op::program as micro_op_merge_sort;

// ---------------------------------------------------------------------------
// Move bytecode
// ---------------------------------------------------------------------------

#[cfg(feature = "move-bytecode")]
mod move_bytecode {
    use move_binary_format::file_format::CompiledModule;

    pub const SOURCE: &str = "
module 0x1::merge_sort {
    use std::vector;

    public fun merge_sort(v: vector<u64>): vector<u64> {
        let len = vector::length(&v);
        if (len > 1) {
            merge_sort_range(&mut v, 0, len);
        };
        v
    }

    fun merge_sort_range(v: &mut vector<u64>, lo: u64, hi: u64) {
        if (hi - lo <= 1) { return };
        let mid = (lo + hi) / 2;
        merge_sort_range(v, lo, mid);
        merge_sort_range(v, mid, hi);
        merge(v, lo, mid, hi);
    }

    fun merge(v: &mut vector<u64>, lo: u64, mid: u64, hi: u64) {
        let tmp = vector::empty<u64>();
        let i = lo;
        let j = mid;
        while (i < mid && j < hi) {
            let a = *vector::borrow(v, i);
            let b = *vector::borrow(v, j);
            if (a < b) {
                vector::push_back(&mut tmp, a);
                i = i + 1;
            } else {
                vector::push_back(&mut tmp, b);
                j = j + 1;
            };
        };
        while (i < mid) {
            vector::push_back(&mut tmp, *vector::borrow(v, i));
            i = i + 1;
        };
        while (j < hi) {
            vector::push_back(&mut tmp, *vector::borrow(v, j));
            j = j + 1;
        };
        let k = lo;
        let t = 0;
        while (k < hi) {
            *vector::borrow_mut(v, k) = *vector::borrow(&tmp, t);
            k = k + 1;
            t = t + 1;
        };
    }
}
";

    pub fn program() -> CompiledModule {
        crate::compile_move_source_with_deps(SOURCE, &[crate::MOVE_STDLIB_DIR])
    }
}

#[cfg(feature = "move-bytecode")]
pub use move_bytecode::program as move_bytecode_merge_sort;
