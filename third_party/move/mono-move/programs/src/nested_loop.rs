// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Nested loop — O(n²) iterations with simple number crunching.
//!
//! Useful as a benchmark for loop dispatch overhead: the inner body does
//! minimal work (XOR + wrapping add) so timing is dominated by the
//! interpreter loop and branch instructions.

/// Test cases: (input, expected output).
pub const NESTED_LOOP_CASES: &[(u64, u64)] = &[
    (0, 0),
    (1, 0),  // i=0, j=0: 0^0 = 0
    (2, 2),  // (0^0)+(0^1)+(1^0)+(1^1) = 0+1+1+0 = 2
    (4, 24), // sum of i^j for i,j in 0..4
    (10, 594),
];

// ---------------------------------------------------------------------------
// Native Rust
// ---------------------------------------------------------------------------

pub fn native_nested_loop(n: u64) -> u64 {
    let mut sum = 0u64;
    let mut i = 0u64;
    while i < n {
        let mut j = 0u64;
        while j < n {
            sum = std::hint::black_box(sum.wrapping_add(i ^ j));
            j += 1;
        }
        i += 1;
    }
    sum
}

// ---------------------------------------------------------------------------
// Micro-op
// ---------------------------------------------------------------------------

/// Pseudocode:
///   fn nested_loop(n: u64) -> u64 {
///       let mut sum: u64 = 0;
///       let mut i: u64 = 0;
///       while i < n {
///           let mut j: u64 = 0;
///           while j < n {
///               sum = sum.wrapping_add(i ^ j);
///               j = j + 1;
///           }
///           i = i + 1;
///       }
///       return sum;
///   }
///
/// Frame layout:
///   [0]  n (arg)      [8]  sum (result)
///   [16] i            [24] j
///   [32] tmp
#[cfg(feature = "micro-op")]
mod micro_op {
    use mono_move_alloc::{ExecutableArena, ExecutableArenaPtr, GlobalArenaPtr};
    use mono_move_core::{
        CodeOffset as CO, FrameLayoutInfo, FrameOffset as FO, Function, MicroOp::*,
        SortedSafePointEntries,
    };
    use mono_move_runtime::ObjectDescriptorTable;

    pub fn program() -> (
        Vec<ExecutableArenaPtr<Function>>,
        ObjectDescriptorTable,
        ExecutableArena,
    ) {
        let arena = ExecutableArena::new();
        let n = 0u32;
        let sum = 8u32;
        let i = 16u32;
        let j = 24u32;
        let tmp = 32u32;
        let param_and_local_sizes_sum = 40u32;

        #[rustfmt::skip]
        let code = [
            // sum = 0; i = 0;
            StoreImm8 { dst: FO(sum), imm: 0 },                    // 0
            StoreImm8 { dst: FO(i), imm: 0 },                      // 1
            // OUTER_LOOP (2): if i < n goto OUTER_BODY
            JumpLessU64 { target: CO(4), lhs: FO(i), rhs: FO(n) }, // 2
            Jump { target: CO(13) },                                // 3: goto END
            // OUTER_BODY: j = 0
            StoreImm8 { dst: FO(j), imm: 0 },                      // 4
            // INNER_LOOP (5): if j < n goto INNER_BODY
            JumpLessU64 { target: CO(7), lhs: FO(j), rhs: FO(n) }, // 5
            Jump { target: CO(11) },                                // 6: goto INNER_END
            // INNER_BODY: sum += i ^ j; j += 1
            BitXorU64 { dst: FO(tmp), lhs: FO(i), rhs: FO(j) },   // 7
            AddU64 { dst: FO(sum), lhs: FO(sum), rhs: FO(tmp) },   // 8
            AddU64Imm { dst: FO(j), src: FO(j), imm: 1 },          // 9
            Jump { target: CO(5) },                                 // 10: goto INNER_LOOP
            // INNER_END (11): i += 1; goto OUTER_LOOP
            AddU64Imm { dst: FO(i), src: FO(i), imm: 1 },          // 11
            Jump { target: CO(2) },                                 // 12: goto OUTER_LOOP
            // END (13): result = sum
            Move8 { dst: FO(n), src: FO(sum) },                    // 13
            Return,                                                 // 14
        ];

        let code = arena.alloc_slice_fill_iter(code);

        let func = arena.alloc(Function {
            name: GlobalArenaPtr::from_static("nested_loop"),
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

        (vec![func], ObjectDescriptorTable::new(), arena)
    }
}

#[cfg(feature = "micro-op")]
pub use micro_op::program as micro_op_nested_loop;

// ---------------------------------------------------------------------------
// Move bytecode
// ---------------------------------------------------------------------------

#[cfg(feature = "move-bytecode")]
mod move_bytecode {
    use move_binary_format::file_format::CompiledModule;

    pub const SOURCE: &str = "
module 0x1::nested_loop {
    public fun nested_loop(n: u64): u64 {
        let sum: u64 = 0;
        let i: u64 = 0;
        while (i < n) {
            let j: u64 = 0;
            while (j < n) {
                sum = sum + (i ^ j);
                j = j + 1;
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
pub use move_bytecode::program as move_bytecode_nested_loop;
