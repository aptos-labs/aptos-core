// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Generic instrumentation pass: insert gas charge ops into a flat
//! instruction sequence.
//!
//! Two kinds of charge ops are inserted:
//!
//! - **`ChargeBlock`** — for each basic block, prepended at the block entry
//!   with the summed static costs of all instructions in the block.
//! - **`ChargeVariable`** — for each instruction with a runtime-variable
//!   cost, appended immediately after the instruction.
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
pub trait GasInstr: Sized {
    /// The slot type used to address runtime values for `ChargeVariable`.
    type Slot;

    /// Construct a static gas charge for a basic block.
    ///
    /// `cost` is the pre-summed static cost of every instruction in the
    /// block, computed at instrumentation time.
    fn charge_block(cost: u64) -> Self;

    /// Construct a runtime-variable gas charge.
    ///
    /// The interpreter evaluates `per_unit * frame[slot]` at runtime. `slot`
    /// holds the runtime quantity (e.g. the number of bytes to copy).
    fn charge_variable(per_unit: u64, slot: Self::Slot) -> Self;
}

/// Rewrites branch-target instruction indices inside an instruction.
///
/// Implemented alongside [`HasCfgInfo`]. [`GasInstrumentation::run`] calls
/// this on every instruction to fix up branch targets after inserting charge
/// ops. Non-branching instructions return `self` unchanged.
pub trait RemapTargets: Sized {
    /// Return a copy of `self` with every branch target index `t` replaced
    /// by `remap(t)`.
    fn remap_targets(self, remap: impl Fn(usize) -> usize) -> Self;
}

// ---------------------------------------------------------------------------
// GasInstrumentation
// ---------------------------------------------------------------------------

/// Inserts gas charge ops into a flat instruction sequence.
///
/// For each basic block, prepends a static block charge with the summed base
/// costs. For each `Dynamic`-cost instruction, also inserts a runtime charge
/// immediately after it. Branch targets are remapped to account for all
/// inserted ops.
pub struct GasInstrumentation<S> {
    pub schedule: S,
}

impl<S> GasInstrumentation<S> {
    pub fn new(schedule: S) -> Self {
        Self { schedule }
    }

    pub fn run<I>(&self, ops: Vec<I>) -> Vec<I>
    where
        I: HasCfgInfo + RemapTargets + GasInstr<Slot = S::Slot>,
        S: GasSchedule<I>,
    {
        if ops.is_empty() {
            return vec![];
        }

        let blocks = compute_basic_blocks(&ops);
        let block_starts: Vec<usize> = blocks.iter().map(|bb| bb.start).collect();

        let costs: Vec<InstrCost<S::Slot>> = ops.iter().map(|op| self.schedule.cost(op)).collect();

        let block_costs: Vec<u64> = blocks
            .iter()
            .map(|bb| {
                (bb.start..bb.end)
                    .map(|i| match &costs[i] {
                        InstrCost::Static(c) | InstrCost::Dynamic { base: c, .. } => *c,
                    })
                    .sum()
            })
            .collect();

        // d_before[i] = number of dynamic-cost instructions at indices < i.
        let mut n_dynamic = 0usize;
        let d_before: Vec<usize> = costs
            .iter()
            .map(|c| {
                let d = n_dynamic;
                if matches!(c, InstrCost::Dynamic { .. }) {
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
                result.push(I::charge_block(block_costs[bi]));
                bi += 1;
            }
            result.push(op.remap_targets(remap));
            if let InstrCost::Dynamic { per_unit, slot, .. } = cost {
                result.push(I::charge_variable(per_unit, slot));
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
        ChargeBlock { cost: u64 },
        ChargeVariable { per_unit: u64 },
    }

    impl GasInstr for TestOp {
        type Slot = ();

        fn charge_block(cost: u64) -> Self {
            TestOp::ChargeBlock { cost }
        }

        fn charge_variable(per_unit: u64, _slot: ()) -> Self {
            TestOp::ChargeVariable { per_unit }
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
            type Slot = ();

            fn cost(&self, _: &TestOp) -> InstrCost<()> {
                InstrCost::Static(0)
            }
        }
        let r: Vec<TestOp> = GasInstrumentation::new(TestSchedule).run(vec![]);
        assert!(r.is_empty());
    }

    /// Input:
    ///   0: Nop    — cost 2
    ///   1: Nop    — cost 2
    ///   2: Return — cost 2
    ///
    /// Output:
    ///   0: ChargeBlock(6)
    ///   1: Nop
    ///   2: Nop
    ///   3: Return
    #[test]
    fn single_block_cost_is_sum() {
        struct TestSchedule;
        impl GasSchedule<TestOp> for TestSchedule {
            type Slot = ();

            fn cost(&self, _: &TestOp) -> InstrCost<()> {
                InstrCost::Static(2)
            }
        }
        let ops = vec![TestOp::Nop, TestOp::Nop, TestOp::Return];
        let r: Vec<TestOp> = GasInstrumentation::new(TestSchedule).run(ops);
        assert_eq!(r, vec![
            TestOp::ChargeBlock { cost: 6 },
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
    ///   0: ChargeBlock(1)
    ///   1: Jump(4)
    ///   2: ChargeBlock(1)
    ///   3: Nop
    ///   4: ChargeBlock(1)
    ///   5: Return
    #[test]
    fn jump_target_remapped_to_charge_block() {
        struct TestSchedule;
        impl GasSchedule<TestOp> for TestSchedule {
            type Slot = ();

            fn cost(&self, _: &TestOp) -> InstrCost<()> {
                InstrCost::Static(1)
            }
        }
        let ops = vec![TestOp::Jump(2), TestOp::Nop, TestOp::Return];
        let r: Vec<TestOp> = GasInstrumentation::new(TestSchedule).run(ops);
        assert_eq!(r, vec![
            TestOp::ChargeBlock { cost: 1 },
            TestOp::Jump(4),
            TestOp::ChargeBlock { cost: 1 },
            TestOp::Nop,
            TestOp::ChargeBlock { cost: 1 },
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
    ///   0: ChargeBlock(1)
    ///   1: CondJump(5)
    ///   2: ChargeBlock(2)
    ///   3: Nop
    ///   4: Jump(0)
    ///   5: ChargeBlock(1)
    ///   6: Return
    #[test]
    fn back_edge_remapped_to_charge_block() {
        struct TestSchedule;
        impl GasSchedule<TestOp> for TestSchedule {
            type Slot = ();

            fn cost(&self, _: &TestOp) -> InstrCost<()> {
                InstrCost::Static(1)
            }
        }
        let ops = vec![
            TestOp::CondJump(3),
            TestOp::Nop,
            TestOp::Jump(0),
            TestOp::Return,
        ];
        let r: Vec<TestOp> = GasInstrumentation::new(TestSchedule).run(ops);
        assert_eq!(r, vec![
            TestOp::ChargeBlock { cost: 1 },
            TestOp::CondJump(5),
            TestOp::ChargeBlock { cost: 2 },
            TestOp::Nop,
            TestOp::Jump(0),
            TestOp::ChargeBlock { cost: 1 },
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
    ///   0: ChargeBlock(3)
    ///   1: Nop
    ///   2: CondJump(3)
    ///   3: ChargeBlock(4)
    ///   4: Nop
    ///   5: Return
    #[test]
    fn blocks_have_correct_costs() {
        struct TestSchedule;
        impl GasSchedule<TestOp> for TestSchedule {
            type Slot = ();

            fn cost(&self, i: &TestOp) -> InstrCost<()> {
                match i {
                    TestOp::Nop => InstrCost::Static(1),
                    TestOp::Jump(_) | TestOp::CondJump(_) => InstrCost::Static(2),
                    TestOp::Return => InstrCost::Static(3),
                    _ => InstrCost::Static(0),
                }
            }
        }
        let ops = vec![
            TestOp::Nop,
            TestOp::CondJump(2),
            TestOp::Nop,
            TestOp::Return,
        ];
        let r: Vec<TestOp> = GasInstrumentation::new(TestSchedule).run(ops);
        assert_eq!(r, vec![
            TestOp::ChargeBlock { cost: 3 },
            TestOp::Nop,
            TestOp::CondJump(3),
            TestOp::ChargeBlock { cost: 4 },
            TestOp::Nop,
            TestOp::Return,
        ]);
    }

    /// Input:
    ///   0: Nop    — Dynamic { base: 5, per_unit: 3 }
    ///   1: Return — Static(5)
    ///
    /// Output:
    ///   0: ChargeBlock(10)
    ///   1: Nop
    ///   2: ChargeVariable(3)
    ///   3: Return
    #[test]
    fn dynamic_charges_inserted_after_instruction() {
        struct TestSchedule;
        impl GasSchedule<TestOp> for TestSchedule {
            type Slot = ();

            fn cost(&self, op: &TestOp) -> InstrCost<()> {
                match op {
                    TestOp::Nop => InstrCost::Dynamic {
                        base: 5,
                        per_unit: 3,
                        slot: (),
                    },
                    _ => InstrCost::Static(5),
                }
            }
        }
        let ops = vec![TestOp::Nop, TestOp::Return];
        let r: Vec<TestOp> = GasInstrumentation::new(TestSchedule).run(ops);
        assert_eq!(r, vec![
            TestOp::ChargeBlock { cost: 10 },
            TestOp::Nop,
            TestOp::ChargeVariable { per_unit: 3 },
            TestOp::Return,
        ]);
    }
}
