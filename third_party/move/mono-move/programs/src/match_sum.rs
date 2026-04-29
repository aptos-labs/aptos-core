// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Match sum — O(n) loop with a 4-arm match inside.
//!
//! The inner body computes `r = i % 4` and branches to one of four arms,
//! all converging to a single merge block. This creates a "wide diamond"
//! CFG shape (multiple basic blocks with edges to the same successor),
//! which stresses gas instrumentation at basic-block boundaries.

/// Test cases: (input, expected output).
///
/// Each full cycle of 4 iterations contributes 10+20+30+40 = 100.
pub const MATCH_SUM_CASES: &[(u64, u64)] = &[
    (0, 0),
    (1, 10),
    (2, 30),
    (3, 60),
    (4, 100),
    (8, 200),
    (100, 2500),
];

// ---------------------------------------------------------------------------
// Native Rust
// ---------------------------------------------------------------------------

pub fn native_match_sum(n: u64) -> u64 {
    let mut sum = 0u64;
    let mut i = 0u64;
    while i < n {
        sum += match i % 4 {
            0 => 10,
            1 => 20,
            2 => 30,
            _ => 40,
        };
        i += 1;
    }
    sum
}

// ---------------------------------------------------------------------------
// Micro-op
// ---------------------------------------------------------------------------

/// Pseudocode:
///   fn match_sum(n: u64) -> u64 {
///       let sum: u64 = 0;
///       let i:   u64 = 0;
///       while i < n {
///           match i % 4 {
///               0 => sum += 10,
///               1 => sum += 20,
///               2 => sum += 30,
///               _ => sum += 40,
///           }
///           i += 1;
///       }
///       sum
///   }
///
/// Frame layout:
///   [0]  n (arg) / result
///   [8]  sum
///   [16] i
///   [24] r  (i % 4)
///   [32] const4
///
/// CFG shape — "wide diamond":
///
///   LOOP ──► BODY ──► head ──► CASE0 ────────────────────╮
///                          └─► GE1  ──► CASE1 ───────────┤
///                                   └─► GE2  ──► CASE2 ──┤
///                                            └─► CASE3 ──┤ (fallthrough)
///                                                        │
///                                                     MERGE ──► LOOP / END
///
/// All four arms (CASE0–CASE3) jump (or fall through) to MERGE, giving it
/// four predecessors — the "wide diamond" this benchmark is designed to test.
#[cfg(feature = "micro-op")]
mod micro_op {
    use mono_move_alloc::{ExecutableArena, ExecutableArenaPtr, GlobalArenaPtr};
    use mono_move_core::{
        CodeOffset as CO, FrameLayoutInfo, FrameOffset as FO, Function, MicroOp::*,
        SortedSafePointEntries,
    };
    use mono_move_runtime::ObjectDescriptor;

    pub fn program() -> (
        Vec<ExecutableArenaPtr<Function>>,
        Vec<ObjectDescriptor>,
        ExecutableArena,
    ) {
        let arena = ExecutableArena::new();
        let n = 0u32;
        let sum = 8u32;
        let i = 16u32;
        let r = 24u32;
        let c4 = 32u32;
        let param_and_local_sizes_sum = 40u32;

        #[rustfmt::skip]
        let code = vec![
            StoreImm8 { dst: FO(sum), imm: 0 },               // 0: sum = 0
            StoreImm8 { dst: FO(i),   imm: 0 },               // 1: i = 0
            StoreImm8 { dst: FO(c4),  imm: 4 },               // 2: const4 = 4

            // LOOP (3): if i < n goto BODY else goto END
            JumpLessU64 { target: CO(6), lhs: FO(i), rhs: FO(n) }, // 3
            Move8 { dst: FO(n), src: FO(sum) },                // 4: result = sum
            Return,                                            // 5

            // BODY (6): r = i % 4
            ModU64 { dst: FO(r), lhs: FO(i), rhs: FO(c4) },  // 6

            // if r >= 1 goto GE1 else CASE0
            JumpGreaterEqualU64Imm { target: CO(10), src: FO(r), imm: 1 }, // 7
            // CASE0 (8): sum += 10; goto MERGE
            AddU64Imm { dst: FO(sum), src: FO(sum), imm: 10 }, // 8
            Jump { target: CO(17) },                           // 9

            // GE1 (10): if r >= 2 goto GE2 else CASE1
            JumpGreaterEqualU64Imm { target: CO(13), src: FO(r), imm: 2 }, // 10
            // CASE1 (11): sum += 20; goto MERGE
            AddU64Imm { dst: FO(sum), src: FO(sum), imm: 20 }, // 11
            Jump { target: CO(17) },                           // 12

            // GE2 (13): if r >= 3 goto CASE3 else CASE2
            JumpGreaterEqualU64Imm { target: CO(16), src: FO(r), imm: 3 }, // 13
            // CASE2 (14): sum += 30; goto MERGE
            AddU64Imm { dst: FO(sum), src: FO(sum), imm: 30 }, // 14
            Jump { target: CO(17) },                           // 15

            // CASE3 (16): sum += 40 (fallthrough to MERGE)
            AddU64Imm { dst: FO(sum), src: FO(sum), imm: 40 }, // 16

            // MERGE (17): i += 1; goto LOOP
            AddU64Imm { dst: FO(i), src: FO(i), imm: 1 },     // 17
            Jump { target: CO(3) },                            // 18
        ];

        let code = arena.alloc_slice_fill_iter(code);

        let func = arena.alloc(Function {
            name: GlobalArenaPtr::from_static("match_sum"),
            code,
            param_sizes: ExecutableArenaPtr::empty_slice(),
            param_sizes_sum: 8,
            param_and_local_sizes_sum: param_and_local_sizes_sum as usize,
            extended_frame_size: param_and_local_sizes_sum as usize
                + mono_move_core::FRAME_METADATA_SIZE,
            zero_frame: false,
            frame_layout: FrameLayoutInfo::empty(),
            safe_point_layouts: SortedSafePointEntries::empty(),
        });

        (vec![func], vec![ObjectDescriptor::Trivial], arena)
    }
}

#[cfg(feature = "micro-op")]
pub use micro_op::program as micro_op_match_sum;

// ---------------------------------------------------------------------------
// Move bytecode
// ---------------------------------------------------------------------------

#[cfg(feature = "move-bytecode")]
mod move_bytecode {
    use move_binary_format::file_format::CompiledModule;

    pub const SOURCE: &str = "
module 0x1::match_sum {
    public fun match_sum(n: u64): u64 {
        let sum: u64 = 0;
        let i: u64 = 0;
        while (i < n) {
            let r = i % 4;
            if (r == 0) {
                sum = sum + 10;
            } else if (r == 1) {
                sum = sum + 20;
            } else if (r == 2) {
                sum = sum + 30;
            } else {
                sum = sum + 40;
            };
            i = i + 1;
        };
        sum
    }
}
";

    pub fn program() -> CompiledModule {
        crate::compile_move_source(SOURCE)
    }
}

#[cfg(feature = "move-bytecode")]
pub use move_bytecode::program as move_bytecode_match_sum;
