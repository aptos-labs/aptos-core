// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Generic instrumentation pass: embed block gas costs into jump instructions.
//!
//! Each basic block's static cost is baked into the jump instructions that
//! *enter* the block, so the interpreter charges gas before executing any
//! work in the block (charge-before-work).
//!
//! ## Gas embedding
//!
//! - **Entry block (block 0)**: its cost is returned as `entry_gas` from
//!   [`GasInstrumentor::run`] and stored in the function's metadata. The
//!   runtime charges it at call time, before any instruction in block 0
//!   executes.
//!
//! - **All other blocks**: each block's cost is written into the jump
//!   instruction(s) that target the block. The interpreter charges the cost
//!   before taking the jump.
//!
//! For **unconditional** jumps, `with_gas` receives the destination block's
//! cost as `taken` and `0` as `fallthrough`.
//!
//! For **conditional** jumps, `with_gas` receives the taken block's cost as
//! `taken` and the fallthrough block's cost as `fallthrough`.
//!
//! For **return** and other instructions without a branch target, `with_gas`
//! is not called — these terminators carry no gas charge because their block's
//! cost was already charged on entry.
//!
//! ## No instruction insertion
//!
//! This pass produces an output sequence of the **same length** as the input.
//! Branch targets remain valid without any remapping.

use crate::{compute_basic_blocks, GasSchedule, HasCfgInfo};

// ---------------------------------------------------------------------------
// Traits
// ---------------------------------------------------------------------------

/// Embeds destination-block gas costs into a jump instruction.
///
/// Implemented alongside the ISA's instruction type. [`GasInstrumentor::run`]
/// calls `with_gas` on every jump instruction (those for which
/// [`HasCfgInfo::branch_target`] returns `Some`).
///
/// - `taken`: cost of the block reached by the primary jump target.
/// - `fallthrough`: cost of the block reached by falling through to the next
///   instruction. Zero for unconditional jumps (their fallthrough is dead code
///   and is never charged).
///
/// For non-jump instructions (return, arithmetic, etc.) the implementation
/// should return `self` unchanged.
pub trait ChargeOnJump: Sized {
    /// Return a copy of `self` with the given block costs embedded.
    fn with_gas(self, taken: u64, fallthrough: u64) -> Self;
}

// ---------------------------------------------------------------------------
// GasInstrumentor
// ---------------------------------------------------------------------------

/// Annotates jump instructions with destination-block gas costs.
///
/// For each basic block, computes the sum of the base costs of all
/// instructions, then writes that cost into every jump instruction that
/// targets the block. The entry block's cost is returned separately as
/// `entry_gas` and must be charged at the call site.
///
/// The output sequence has the **same length** as the input — no instructions
/// are inserted and no branch targets need remapping.
///
/// TODO: this runs as a separate pass over the instruction sequence. Ideally
/// it would be fused with an earlier compiler pass to avoid redundant
/// traversals, since instrumentation may run on a critical path (e.g. on a
/// cache miss).
pub struct GasInstrumentor<S> {
    pub schedule: S,
}

impl<S> GasInstrumentor<S> {
    pub fn new(schedule: S) -> Self {
        Self { schedule }
    }

    /// Instrument `ops` and return `(annotated_ops, entry_gas)`.
    ///
    /// `entry_gas` is the static cost of block 0. The caller must store it in
    /// the function's metadata and charge it at call time before executing any
    /// instruction.
    pub fn run<I>(&self, ops: Vec<I>) -> (Vec<I>, u64)
    where
        I: HasCfgInfo + ChargeOnJump,
        S: GasSchedule<I>,
    {
        if ops.is_empty() {
            return (vec![], 0);
        }

        let blocks = compute_basic_blocks(&ops);

        // Compute the static cost of each basic block.
        let block_costs: Vec<u64> = blocks
            .iter()
            .map(|bb| {
                (bb.start..bb.end)
                    .map(|i| self.schedule.cost(&ops[i]))
                    .sum()
            })
            .collect();

        // entry_gas = cost of block 0, charged at function call time.
        let entry_gas = block_costs.first().copied().unwrap_or(0);

        // Build a lookup table: cost_at_start[i] = cost of the block whose
        // leader is at index i, or 0 if no block starts there.
        // Size is ops.len()+1 so that `bb.end` for the last block is in bounds.
        let mut cost_at_start = vec![0u64; ops.len() + 1];
        for (bb, &cost) in blocks.iter().zip(&block_costs) {
            cost_at_start[bb.start] = cost;
        }

        // For each block terminator that is a jump, record the (taken, fallthrough)
        // destination costs to embed.
        let mut gas_vec: Vec<Option<(u64, u64)>> = vec![None; ops.len()];
        for bb in &blocks {
            let term_idx = bb.end - 1;
            if let Some(taken_target) = ops[term_idx].branch_target() {
                let taken_cost = *cost_at_start.get(taken_target).unwrap_or(&0);
                let fallthrough_cost = *cost_at_start.get(bb.end).unwrap_or(&0);
                gas_vec[term_idx] = Some((taken_cost, fallthrough_cost));
            }
            // Return and other non-jump terminators: no gas annotation.
        }

        let instrumented = ops
            .into_iter()
            .enumerate()
            .map(|(i, op)| match gas_vec[i] {
                Some((taken, fallthrough)) => op.with_gas(taken, fallthrough),
                None => op,
            })
            .collect();

        (instrumented, entry_gas)
    }

    /// Instrument every function in `program` and return the results in the
    /// same order as `(annotated_ops, entry_gas)` pairs.
    ///
    /// Equivalent to calling [`run`](Self::run) on each element individually.
    pub fn run_all<I>(&self, program: Vec<Vec<I>>) -> Vec<(Vec<I>, u64)>
    where
        I: HasCfgInfo + ChargeOnJump,
        S: GasSchedule<I>,
    {
        program.into_iter().map(|ops| self.run(ops)).collect()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GasSchedule;

    // Minimal stub instruction set for testing.
    //
    // Jump and CondJump carry gas fields that `with_gas` populates.
    // Return and Nop have no gas fields.
    #[derive(Debug, Clone, PartialEq)]
    enum TestOp {
        Nop,
        Jump {
            target: usize,
            gas: u64,
        },
        CondJump {
            target: usize,
            gas_taken: u64,
            gas_fallthrough: u64,
        },
        Return,
    }

    impl ChargeOnJump for TestOp {
        fn with_gas(self, taken: u64, fallthrough: u64) -> Self {
            match self {
                TestOp::Jump { target, .. } => TestOp::Jump { target, gas: taken },
                TestOp::CondJump { target, .. } => TestOp::CondJump {
                    target,
                    gas_taken: taken,
                    gas_fallthrough: fallthrough,
                },
                other => other,
            }
        }
    }

    impl HasCfgInfo for TestOp {
        fn branch_target(&self) -> Option<usize> {
            match self {
                TestOp::Jump { target, .. } | TestOp::CondJump { target, .. } => Some(*target),
                _ => None,
            }
        }
    }

    #[test]
    fn empty() {
        struct TestSchedule;
        impl GasSchedule<TestOp> for TestSchedule {
            fn cost(&self, _: &TestOp) -> u64 {
                0
            }
        }
        let (ops, entry_gas) = GasInstrumentor::new(TestSchedule).run(vec![]);
        assert!(ops.is_empty());
        assert_eq!(entry_gas, 0);
    }

    /// Single block: entry_gas = sum of all instruction costs; no jump to annotate.
    ///
    /// Input:
    ///   0: Nop    — cost 2
    ///   1: Nop    — cost 2
    ///   2: Return — cost 2
    ///
    /// Output:
    ///   ops: [Nop, Nop, Return]  (unchanged — Return has no branch_target)
    ///   entry_gas: 6
    #[test]
    fn single_block_entry_gas_is_sum() {
        struct TestSchedule;
        impl GasSchedule<TestOp> for TestSchedule {
            fn cost(&self, _: &TestOp) -> u64 {
                2
            }
        }
        let ops = vec![TestOp::Nop, TestOp::Nop, TestOp::Return];
        let (result, entry_gas) = GasInstrumentor::new(TestSchedule).run(ops);
        assert_eq!(result, vec![TestOp::Nop, TestOp::Nop, TestOp::Return]);
        assert_eq!(entry_gas, 6);
    }

    /// Unconditional jump carries the destination block's cost.
    ///
    /// Input:
    ///   0: Jump(2) — cost 1
    ///   1: Nop     — cost 1  (dead block — cost never charged)
    ///   2: Return  — cost 1
    ///
    /// Blocks:
    ///   [0..1]: Jump(2), cost 1   → entry_gas
    ///   [1..2]: Nop,     cost 1   (dead)
    ///   [2..3]: Return,  cost 1
    ///
    /// Jump(2).gas = cost(block at 2) = 1
    ///
    /// Output:
    ///   ops: [Jump { target:2, gas:1 }, Nop, Return]
    ///   entry_gas: 1
    #[test]
    fn unconditional_jump_carries_dest_cost() {
        struct TestSchedule;
        impl GasSchedule<TestOp> for TestSchedule {
            fn cost(&self, _: &TestOp) -> u64 {
                1
            }
        }
        let ops = vec![
            TestOp::Jump { target: 2, gas: 0 },
            TestOp::Nop,
            TestOp::Return,
        ];
        let (result, entry_gas) = GasInstrumentor::new(TestSchedule).run(ops);
        assert_eq!(result, vec![
            TestOp::Jump { target: 2, gas: 1 },
            TestOp::Nop,
            TestOp::Return,
        ]);
        assert_eq!(entry_gas, 1);
    }

    /// Conditional jump carries both the taken and fallthrough block costs.
    /// Back-edge jump carries the loop header's cost (ensuring correct per-iteration charge).
    ///
    /// Input:
    ///   0: CondJump(3) — cost 1  (loop header: taken → exit, fallthrough → body)
    ///   1: Nop         — cost 1
    ///   2: Jump(0)     — cost 1  (back edge: destination is the loop header)
    ///   3: Return      — cost 1
    ///
    /// Blocks and costs:
    ///   [0..1]: CondJump, cost 1   → entry_gas = 1
    ///   [1..3]: {Nop, Jump(0)}, cost 2
    ///   [3..4]: Return, cost 1
    ///
    /// CondJump(3): gas_taken = cost(block at 3) = 1, gas_fallthrough = cost(block at 1) = 2
    /// Jump(0):     gas      = cost(block at 0) = 1
    ///
    /// Output:
    ///   ops: [CondJump{3,taken=1,fall=2}, Nop, Jump{0,gas=1}, Return]
    ///   entry_gas: 1
    #[test]
    fn back_edge_carries_loop_header_cost() {
        struct TestSchedule;
        impl GasSchedule<TestOp> for TestSchedule {
            fn cost(&self, _: &TestOp) -> u64 {
                1
            }
        }
        let ops = vec![
            TestOp::CondJump {
                target: 3,
                gas_taken: 0,
                gas_fallthrough: 0,
            },
            TestOp::Nop,
            TestOp::Jump { target: 0, gas: 0 },
            TestOp::Return,
        ];
        let (result, entry_gas) = GasInstrumentor::new(TestSchedule).run(ops);
        assert_eq!(result, vec![
            TestOp::CondJump {
                target: 3,
                gas_taken: 1,
                gas_fallthrough: 2
            },
            TestOp::Nop,
            TestOp::Jump { target: 0, gas: 1 },
            TestOp::Return,
        ]);
        assert_eq!(entry_gas, 1);
    }

    /// Mixed instruction costs: verify that block sums are correct.
    ///
    /// Input:
    ///   0: Nop         — cost 1
    ///   1: CondJump(2) — cost 2
    ///   2: Nop         — cost 1
    ///   3: Return      — cost 3
    ///
    /// Blocks:
    ///   [0..2]: {Nop, CondJump(2)}, cost 1+2 = 3   → entry_gas = 3
    ///   [2..4]: {Nop, Return},      cost 1+3 = 4
    ///
    /// CondJump(2): gas_taken = 4 (taken to block at 2), gas_fallthrough = 4 (fallthrough to 2)
    ///   (both paths lead to block [2..4] in this degenerate case)
    ///
    /// Output:
    ///   ops: [Nop, CondJump{2,taken=4,fall=4}, Nop, Return]
    ///   entry_gas: 3
    #[test]
    fn blocks_have_correct_costs() {
        struct TestSchedule;
        impl GasSchedule<TestOp> for TestSchedule {
            fn cost(&self, i: &TestOp) -> u64 {
                match i {
                    TestOp::Nop => 1,
                    TestOp::Jump { .. } | TestOp::CondJump { .. } => 2,
                    TestOp::Return => 3,
                }
            }
        }
        let ops = vec![
            TestOp::Nop,
            TestOp::CondJump {
                target: 2,
                gas_taken: 0,
                gas_fallthrough: 0,
            },
            TestOp::Nop,
            TestOp::Return,
        ];
        let (result, entry_gas) = GasInstrumentor::new(TestSchedule).run(ops);
        assert_eq!(result, vec![
            TestOp::Nop,
            TestOp::CondJump {
                target: 2,
                gas_taken: 4,
                gas_fallthrough: 4
            },
            TestOp::Nop,
            TestOp::Return,
        ]);
        assert_eq!(entry_gas, 3);
    }
}
