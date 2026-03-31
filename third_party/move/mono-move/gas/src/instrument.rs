// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Generic instrumentation pass: insert gas charge ops into a flat
//! instruction sequence.
//!
//! Two kinds of charge ops are inserted:
//!
//! - **`Charge`** — for each basic block, prepended at the block entry
//!   with the summed static costs of all instructions in the block.
//! - An optional dynamic charge op — for each instruction with a
//!   runtime-variable cost component, appended immediately after it.
//!
//! ## Dead code
//!
//! The pass instruments every basic block, including unreachable ones,
//! potentially doubling program size in the worst case.
//!
//! TODO: the compiler should eliminate dead basic blocks before this pass
//! runs, both to avoid wasted allocation and to prevent dead `Charge` ops
//! from polluting the instruction cache.
//!
//! ## Branch-target remapping
//!
//! Inserting charge ops shifts instruction indices, so all branch targets are
//! rewritten to account for all inserted charge ops. The [`RemapTargets`]
//! trait lets each instruction type perform this rewrite without the gas
//! crate knowing instruction internals.

use crate::{compute_basic_blocks, GasSchedule, HasCfgInfo, InstrCost};

// ---------------------------------------------------------------------------
// Traits
// ---------------------------------------------------------------------------

/// Constructs gas charge instructions within an ISA.
///
/// Implemented alongside the ISA's instruction type.
pub trait GasMeteredInstruction: Sized {
    /// Construct a static gas charge.
    ///
    /// In practice, `cost` will be the pre-summed static cost of every
    /// instruction in a basic block, computed at instrumentation time.
    fn charge(cost: u64) -> Self;
}

/// Rewrites branch-target instruction indices inside an instruction.
///
/// Implemented alongside [`HasCfgInfo`]. [`GasInstrumentor::run`] calls
/// this on every instruction to fix up branch targets after inserting charge
/// ops. Non-branching instructions return `self` unchanged.
pub trait RemapTargets: Sized + HasCfgInfo {
    /// Return a copy of `self` with every branch target index `t` replaced
    /// by `remap(t)`.
    fn remap_targets(self, remap: impl Fn(usize) -> usize) -> Self;
}

// ---------------------------------------------------------------------------
// GasInstrumentor
// ---------------------------------------------------------------------------

/// Inserts gas charge ops into a flat instruction sequence.
///
/// For each basic block, prepends a static block charge with the summed base
/// costs. For each `Dynamic`-cost instruction, also inserts a runtime charge
/// immediately after it. Branch targets are remapped to account for all
/// inserted ops.
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

    pub fn run<I>(&self, ops: Vec<I>) -> Vec<I>
    where
        I: HasCfgInfo + RemapTargets + GasMeteredInstruction,
        S: GasSchedule<I>,
    {
        if ops.is_empty() {
            return vec![];
        }

        let blocks = compute_basic_blocks(&ops);
        let block_starts: Vec<usize> = blocks.iter().map(|bb| bb.start).collect();

        let costs: Vec<InstrCost<I>> = ops.iter().map(|op| self.schedule.cost(op)).collect();

        let block_costs: Vec<u64> = blocks
            .iter()
            .map(|bb| (bb.start..bb.end).map(|i| costs[i].base).sum())
            .collect();

        // d_before[i] = number of dynamic-cost instructions at indices < i.
        let mut n_dynamic = 0usize;
        let d_before: Vec<usize> = costs
            .iter()
            .map(|c| {
                let d = n_dynamic;
                if c.dynamic.is_some() {
                    n_dynamic += 1;
                }
                d
            })
            .collect();

        let remap = |t: usize| t + block_starts.partition_point(|&s| s < t) + d_before[t];

        let mut result = Vec::with_capacity(ops.len() + blocks.len() + n_dynamic);
        let mut bi = 0usize;
        for (i, (op, cost)) in ops.into_iter().zip(costs).enumerate() {
            if bi < block_starts.len() && block_starts[bi] == i {
                result.push(I::charge(block_costs[bi]));
                bi += 1;
            }
            result.push(op.remap_targets(remap));
            if let Some(dynamic) = cost.dynamic {
                result.push(dynamic);
            }
        }

        result
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Minimal stub instruction set for testing.
    #[derive(Debug, Clone, PartialEq)]
    enum TestOp {
        Nop,
        Jump(usize),
        CondJump(usize),
        Return,
        Charge(u64),
        ChargeDynamic(u64),
    }

    impl GasMeteredInstruction for TestOp {
        fn charge(cost: u64) -> Self {
            TestOp::Charge(cost)
        }
    }

    impl HasCfgInfo for TestOp {
        fn branch_target(&self) -> Option<usize> {
            match self {
                TestOp::Jump(t) | TestOp::CondJump(t) => Some(*t),
                _ => None,
            }
        }
    }

    impl RemapTargets for TestOp {
        fn remap_targets(self, remap: impl Fn(usize) -> usize) -> Self {
            match self {
                TestOp::Jump(t) => TestOp::Jump(remap(t)),
                TestOp::CondJump(t) => TestOp::CondJump(remap(t)),
                other => other,
            }
        }
    }

    #[test]
    fn empty() {
        struct TestSchedule;
        impl GasSchedule<TestOp> for TestSchedule {
            fn cost(&self, _: &TestOp) -> InstrCost<TestOp> {
                InstrCost::constant(0)
            }
        }
        let r: Vec<TestOp> = GasInstrumentor::new(TestSchedule).run(vec![]);
        assert!(r.is_empty());
    }

    /// Input:
    ///   0: Nop    — cost 2
    ///   1: Nop    — cost 2
    ///   2: Return — cost 2
    ///
    /// Output:
    ///   0: Charge(6)
    ///   1: Nop
    ///   2: Nop
    ///   3: Return
    #[test]
    fn single_block_cost_is_sum() {
        struct TestSchedule;
        impl GasSchedule<TestOp> for TestSchedule {
            fn cost(&self, _: &TestOp) -> InstrCost<TestOp> {
                InstrCost::constant(2)
            }
        }
        let ops = vec![TestOp::Nop, TestOp::Nop, TestOp::Return];
        let r: Vec<TestOp> = GasInstrumentor::new(TestSchedule).run(ops);
        assert_eq!(r, vec![
            TestOp::Charge(6),
            TestOp::Nop,
            TestOp::Nop,
            TestOp::Return,
        ]);
    }

    /// Input:
    ///   0: Jump(2) — cost 1
    ///   1: Nop     — cost 1
    ///   2: Return  — cost 1
    ///
    /// Output:
    ///   0: Charge(1)
    ///   1: Jump(4)
    ///   2: Charge(1)
    ///   3: Nop
    ///   4: Charge(1)
    ///   5: Return
    #[test]
    fn jump_target_remapped_to_charge() {
        struct TestSchedule;
        impl GasSchedule<TestOp> for TestSchedule {
            fn cost(&self, _: &TestOp) -> InstrCost<TestOp> {
                InstrCost::constant(1)
            }
        }
        let ops = vec![TestOp::Jump(2), TestOp::Nop, TestOp::Return];
        let r: Vec<TestOp> = GasInstrumentor::new(TestSchedule).run(ops);
        assert_eq!(r, vec![
            TestOp::Charge(1),
            TestOp::Jump(4),
            TestOp::Charge(1),
            TestOp::Nop,
            TestOp::Charge(1),
            TestOp::Return,
        ]);
    }

    /// Input:
    ///   0: CondJump(3) — cost 1
    ///   1: Nop         — cost 1
    ///   2: Jump(0)     — cost 1
    ///   3: Return      — cost 1
    ///
    /// Output:
    ///   0: Charge(1)
    ///   1: CondJump(5)
    ///   2: Charge(2)
    ///   3: Nop
    ///   4: Jump(0)
    ///   5: Charge(1)
    ///   6: Return
    #[test]
    fn back_edge_remapped_to_charge() {
        struct TestSchedule;
        impl GasSchedule<TestOp> for TestSchedule {
            fn cost(&self, _: &TestOp) -> InstrCost<TestOp> {
                InstrCost::constant(1)
            }
        }
        let ops = vec![
            TestOp::CondJump(3),
            TestOp::Nop,
            TestOp::Jump(0),
            TestOp::Return,
        ];
        let r: Vec<TestOp> = GasInstrumentor::new(TestSchedule).run(ops);
        assert_eq!(r, vec![
            TestOp::Charge(1),
            TestOp::CondJump(5),
            TestOp::Charge(2),
            TestOp::Nop,
            TestOp::Jump(0),
            TestOp::Charge(1),
            TestOp::Return,
        ]);
    }

    /// Input:
    ///   0: Nop         — cost 1
    ///   1: CondJump(2) — cost 2
    ///   2: Nop         — cost 1
    ///   3: Return      — cost 3
    ///
    /// Output:
    ///   0: Charge(3)
    ///   1: Nop
    ///   2: CondJump(3)
    ///   3: Charge(4)
    ///   4: Nop
    ///   5: Return
    #[test]
    fn blocks_have_correct_costs() {
        struct TestSchedule;
        impl GasSchedule<TestOp> for TestSchedule {
            fn cost(&self, i: &TestOp) -> InstrCost<TestOp> {
                InstrCost::constant(match i {
                    TestOp::Nop => 1,
                    TestOp::Jump(_) | TestOp::CondJump(_) => 2,
                    TestOp::Return => 3,
                    _ => 0,
                })
            }
        }
        let ops = vec![
            TestOp::Nop,
            TestOp::CondJump(2),
            TestOp::Nop,
            TestOp::Return,
        ];
        let r: Vec<TestOp> = GasInstrumentor::new(TestSchedule).run(ops);
        assert_eq!(r, vec![
            TestOp::Charge(3),
            TestOp::Nop,
            TestOp::CondJump(3),
            TestOp::Charge(4),
            TestOp::Nop,
            TestOp::Return,
        ]);
    }

    /// Input:
    ///   0: Nop    — base: 5, dynamic: Some(ChargeDynamic(3))
    ///   1: Return — base: 5, dynamic: None
    ///
    /// Output:
    ///   0: Charge(10)
    ///   1: Nop
    ///   2: ChargeDynamic(3)
    ///   3: Return
    #[test]
    fn dynamic_charges_inserted_after_instruction() {
        struct TestSchedule;
        impl GasSchedule<TestOp> for TestSchedule {
            fn cost(&self, op: &TestOp) -> InstrCost<TestOp> {
                match op {
                    TestOp::Nop => InstrCost {
                        base: 5,
                        dynamic: Some(TestOp::ChargeDynamic(3)),
                    },
                    _ => InstrCost::constant(5),
                }
            }
        }
        let ops = vec![TestOp::Nop, TestOp::Return];
        let r: Vec<TestOp> = GasInstrumentor::new(TestSchedule).run(ops);
        assert_eq!(r, vec![
            TestOp::Charge(10),
            TestOp::Nop,
            TestOp::ChargeDynamic(3),
            TestOp::Return,
        ]);
    }

    /// Dead code: Nop at index 1 is unreachable but still gets a Charge op.
    ///
    /// Input:
    ///   0: Jump(2) — cost 1
    ///   1: Nop     — cost 1 (dead)
    ///   2: Return  — cost 1
    ///
    /// Output:
    ///   0: Charge(1)
    ///   1: Jump(4)
    ///   2: Charge(1)
    ///   3: Nop
    ///   4: Charge(1)
    ///   5: Return
    #[test]
    fn dead_code_block_still_instrumented() {
        struct TestSchedule;
        impl GasSchedule<TestOp> for TestSchedule {
            fn cost(&self, _: &TestOp) -> InstrCost<TestOp> {
                InstrCost::constant(1)
            }
        }
        let ops = vec![TestOp::Jump(2), TestOp::Nop, TestOp::Return];
        let r: Vec<TestOp> = GasInstrumentor::new(TestSchedule).run(ops);
        assert_eq!(r, vec![
            TestOp::Charge(1),
            TestOp::Jump(4),
            TestOp::Charge(1),
            TestOp::Nop,
            TestOp::Charge(1),
            TestOp::Return,
        ]);
    }
}
