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
    use mono_move_alloc::GlobalArenaPtr;
    use mono_move_core::{
        Code, CodeOffset as CO, FrameLayoutInfo, FrameOffset as FO, Function, FunctionPtr,
        MicroOp::*, SortedSafePointEntries, FRAME_METADATA_SIZE,
    };
    use mono_move_runtime::{ObjectDescriptor, ObjectDescriptorTable};

    #[rustfmt::skip]
    pub fn program() -> (Vec<FunctionPtr>, ObjectDescriptorTable) {
        let meta = FRAME_METADATA_SIZE as u32;

        let mut descriptors = ObjectDescriptorTable::new();
        let desc_vec_u64 = descriptors.push(ObjectDescriptor::new_vector(8, vec![]).unwrap());

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
        let ms_vec = 0u32;
        let ms_vec_ref = 16u32;
        let ms_param_and_local_sizes_sum = 32u32;
        let ms_callee_vec = ms_param_and_local_sizes_sum + meta;
        let ms_callee_lo = ms_callee_vec + 8;
        let ms_callee_hi = ms_callee_lo + 8;

        let merge_sort_ptr = FunctionPtr::new(Box::new(Function {
            name: GlobalArenaPtr::from_static("merge_sort"),
            code: Code::from_vec(vec![]),
            param_sizes: vec![],
            param_sizes_sum: 8,
            param_and_local_sizes_sum: ms_param_and_local_sizes_sum as usize,
            extended_frame_size: (ms_callee_hi + 8) as usize,
            zero_frame: true,
            frame_layout: FrameLayoutInfo::new(vec![FO(ms_vec), FO(ms_vec_ref)]),
            safe_point_layouts: SortedSafePointEntries::empty(),
        }));

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
        let msr_vec = 0u32;
        let msr_param_and_local_sizes_sum = 40u32;
        let msr_callee_3 = msr_param_and_local_sizes_sum + meta + 24;

        let merge_sort_range_ptr = FunctionPtr::new(Box::new(Function {
            name: GlobalArenaPtr::from_static("merge_sort_range"),
            code: Code::from_vec(vec![]),
            param_sizes: vec![],
            param_sizes_sum: 24,
            param_and_local_sizes_sum: msr_param_and_local_sizes_sum as usize,
            extended_frame_size: (msr_callee_3 + 8) as usize,
            zero_frame: true,
            frame_layout: FrameLayoutInfo::new(vec![FO(msr_vec)]),
            safe_point_layouts: SortedSafePointEntries::empty(),
        }));

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
        let m_vec = 0u32;
        let m_tmp = 32u32;
        let m_vec_ref = 88u32;
        let m_tmp_ref = 104u32;

        let merge_ptr = FunctionPtr::new(Box::new(Function {
            name: GlobalArenaPtr::from_static("merge"),
            code: Code::from_vec(vec![]),
            param_sizes: vec![],
            param_sizes_sum: 32,
            param_and_local_sizes_sum: 120,
            extended_frame_size: 144,
            zero_frame: true,
            frame_layout: FrameLayoutInfo::new(vec![
                FO(m_vec),
                FO(m_tmp),
                FO(m_vec_ref),
                FO(m_tmp_ref),
            ]),
            safe_point_layouts: SortedSafePointEntries::empty(),
        }));

        let merge_sort_code = {
            let vec = ms_vec;
            let len = 8u32;
            let vec_ref = ms_vec_ref;
            let callee_vec = ms_callee_vec;
            let callee_lo = ms_callee_lo;
            let callee_hi = ms_callee_hi;

            vec![
                SlotBorrow { dst: FO(vec_ref), local: FO(vec) },
                VecLen { dst: FO(len), vec_ref: FO(vec_ref) },
                Move8 { dst: FO(callee_vec), src: FO(vec) },
                StoreImm8 { dst: FO(callee_lo), imm: 0 },
                Move8 { dst: FO(callee_hi), src: FO(len) },
                CallDirect { ptr: merge_sort_range_ptr },
                Return,
            ]
        };

        let merge_sort_range_code = {
            let vec = msr_vec;
            let lo = 8u32;
            let hi = 16u32;
            let mid = 24u32;
            let tmp = 32u32;
            let callee_0 = msr_param_and_local_sizes_sum + meta;
            let callee_1 = callee_0 + 8;
            let callee_2 = callee_1 + 8;
            let callee_3 = callee_2 + 8;

            vec![
                AddU64Imm { dst: FO(tmp), src: FO(lo), imm: 1 },
                JumpLessU64 { target: CO(3), lhs: FO(tmp), rhs: FO(hi) },
                Return,
                AddU64 { dst: FO(mid), lhs: FO(lo), rhs: FO(hi) },
                ShrU64Imm { dst: FO(mid), src: FO(mid), imm: 1 },
                Move8 { dst: FO(callee_0), src: FO(vec) },
                Move8 { dst: FO(callee_1), src: FO(lo) },
                Move8 { dst: FO(callee_2), src: FO(mid) },
                CallDirect { ptr: merge_sort_range_ptr },
                Move8 { dst: FO(callee_0), src: FO(vec) },
                Move8 { dst: FO(callee_1), src: FO(mid) },
                Move8 { dst: FO(callee_2), src: FO(hi) },
                CallDirect { ptr: merge_sort_range_ptr },
                Move8 { dst: FO(callee_0), src: FO(vec) },
                Move8 { dst: FO(callee_1), src: FO(lo) },
                Move8 { dst: FO(callee_2), src: FO(mid) },
                Move8 { dst: FO(callee_3), src: FO(hi) },
                CallDirect { ptr: merge_ptr },
                Return,
            ]
        };

        let merge_code = {
            let vec = m_vec;
            let lo = 8u32;
            let mid = 16u32;
            let hi = 24u32;
            let tmp = m_tmp;
            let i = 40u32;
            let j = 48u32;
            let elem_a = 56u32;
            let elem_b = 64u32;
            let k = 72u32;
            let tmp_idx = 80u32;
            let vec_ref = m_vec_ref;
            let tmp_ref = m_tmp_ref;

            vec![
                Move8 { dst: FO(i), src: FO(lo) },                              // 0
                Move8 { dst: FO(j), src: FO(mid) },                             // 1
                VecNew { dst: FO(tmp) },                                        // 2
                SlotBorrow { dst: FO(vec_ref), local: FO(vec) },                // 3
                SlotBorrow { dst: FO(tmp_ref), local: FO(tmp) },                // 4

                JumpLessU64 { target: CO(8), lhs: FO(i), rhs: FO(mid) },        // 5
                JumpLessU64 { target: CO(25), lhs: FO(j), rhs: FO(hi) },        // 6
                Jump { target: CO(31) },                                        // 7
                JumpLessU64 { target: CO(10), lhs: FO(j), rhs: FO(hi) },        // 8
                Jump { target: CO(19) },                                        // 9

                VecLoadElem { dst: FO(elem_a), vec_ref: FO(vec_ref),
                              idx: FO(i), elem_size: 8 },                       // 10
                VecLoadElem { dst: FO(elem_b), vec_ref: FO(vec_ref),
                              idx: FO(j), elem_size: 8 },                       // 11
                JumpLessU64 { target: CO(16), lhs: FO(elem_a), rhs: FO(elem_b) }, // 12
                VecPushBack { vec_ref: FO(tmp_ref), elem: FO(elem_b), elem_size: 8, descriptor_id: desc_vec_u64 }, // 13
                AddU64Imm { dst: FO(j), src: FO(j), imm: 1 },                   // 14
                Jump { target: CO(5) },                                         // 15

                VecPushBack { vec_ref: FO(tmp_ref), elem: FO(elem_a), elem_size: 8, descriptor_id: desc_vec_u64 }, // 16
                AddU64Imm { dst: FO(i), src: FO(i), imm: 1 },                   // 17
                Jump { target: CO(5) },                                         // 18

                JumpLessU64 { target: CO(21), lhs: FO(i), rhs: FO(mid) },       // 19
                Jump { target: CO(31) },                                        // 20
                VecLoadElem { dst: FO(elem_a), vec_ref: FO(vec_ref),
                              idx: FO(i), elem_size: 8 },                       // 21
                VecPushBack { vec_ref: FO(tmp_ref), elem: FO(elem_a), elem_size: 8, descriptor_id: desc_vec_u64 }, // 22
                AddU64Imm { dst: FO(i), src: FO(i), imm: 1 },                   // 23
                Jump { target: CO(19) },                                        // 24

                JumpLessU64 { target: CO(27), lhs: FO(j), rhs: FO(hi) },        // 25
                Jump { target: CO(31) },                                        // 26
                VecLoadElem { dst: FO(elem_b), vec_ref: FO(vec_ref),
                              idx: FO(j), elem_size: 8 },                       // 27
                VecPushBack { vec_ref: FO(tmp_ref), elem: FO(elem_b), elem_size: 8, descriptor_id: desc_vec_u64 }, // 28
                AddU64Imm { dst: FO(j), src: FO(j), imm: 1 },                   // 29
                Jump { target: CO(25) },                                        // 30

                Move8 { dst: FO(k), src: FO(lo) },                              // 31
                StoreImm8 { dst: FO(tmp_idx), imm: 0 },                         // 32
                JumpLessU64 { target: CO(35), lhs: FO(k), rhs: FO(hi) },        // 33
                Return,                                                          // 34
                VecLoadElem { dst: FO(elem_a), vec_ref: FO(tmp_ref),
                              idx: FO(tmp_idx), elem_size: 8 },                 // 35
                VecStoreElem { vec_ref: FO(vec_ref), idx: FO(k),
                               src: FO(elem_a), elem_size: 8 },                 // 36
                AddU64Imm { dst: FO(k), src: FO(k), imm: 1 },                   // 37
                AddU64Imm { dst: FO(tmp_idx), src: FO(tmp_idx), imm: 1 },       // 38
                Jump { target: CO(33) },                                        // 39
            ]
        };

        unsafe { merge_sort_ptr.as_ref_unchecked() }
            .code
            .store(merge_sort_code);
        unsafe { merge_sort_range_ptr.as_ref_unchecked() }
            .code
            .store(merge_sort_range_code);
        unsafe { merge_ptr.as_ref_unchecked() }.code.store(merge_code);

        (
            vec![merge_sort_ptr, merge_sort_range_ptr, merge_ptr],
            descriptors,
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
