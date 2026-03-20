// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Basic-block identification for flat instruction sequences.
//!
//! Currently this only partitions instructions into basic blocks, but in the
//! future it may evolve to compute a full control-flow graph.
//!
//! A *basic block* is a maximal straight-line sequence of instructions with a
//! single entry point (its *leader*) and no internal branches. The
//! [`compute_basic_blocks`] function partitions a flat instruction array into
//! such blocks by collecting leaders: instruction `0`, every branch target,
//! and every instruction immediately following a branch.
//!
//! The analysis is generic over the instruction type via [`HasCfgInfo`], so
//! the same code works for any instruction type (micro-ops, stackless IR, …).

use std::collections::BTreeSet;

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Identifies basic-block boundaries in a flat instruction sequence.
///
/// Implement this trait for an instruction type to make it usable with
/// [`compute_basic_blocks`] and [`GasInstrumentor`][crate::instrument::GasInstrumentor].
/// The only required information is whether an instruction branches and, if so,
/// where.
pub trait HasCfgInfo {
    /// Returns the explicit branch target of this instruction, if any.
    ///
    /// - Jumps (unconditional or conditional) return `Some(target)`.
    /// - All other instructions, including `Return`, return `None`.
    fn branch_target(&self) -> Option<usize>;
}

// ---------------------------------------------------------------------------
// BasicBlock
// ---------------------------------------------------------------------------

/// A contiguous range `instrs[start..end]` forming a basic block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BasicBlock {
    /// Index of the first instruction (the *leader*).
    pub start: usize,
    /// One past the index of the last instruction.
    pub end: usize,
}

impl BasicBlock {
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

// ---------------------------------------------------------------------------
// Algorithm
// ---------------------------------------------------------------------------

/// Partition a flat instruction slice into basic blocks, returned in order.
pub fn compute_basic_blocks<I: HasCfgInfo>(instrs: &[I]) -> Vec<BasicBlock> {
    if instrs.is_empty() {
        return vec![];
    }

    let mut leaders: BTreeSet<usize> = BTreeSet::new();
    leaders.insert(0);

    for (i, instr) in instrs.iter().enumerate() {
        if let Some(target) = instr.branch_target() {
            if target < instrs.len() {
                leaders.insert(target);
            }
            if i + 1 < instrs.len() {
                leaders.insert(i + 1);
            }
        }
    }

    let leaders: Vec<usize> = leaders.into_iter().collect();

    let ends = leaders[1..]
        .iter()
        .copied()
        .chain(std::iter::once(instrs.len()));
    leaders
        .iter()
        .copied()
        .zip(ends)
        .map(|(start, end)| BasicBlock { start, end })
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal stub instruction for testing.
    enum TestOp {
        Nop,
        Jump(usize),
        CondJump(usize),
        Return,
    }

    impl HasCfgInfo for TestOp {
        fn branch_target(&self) -> Option<usize> {
            match self {
                TestOp::Jump(t) | TestOp::CondJump(t) => Some(*t),
                _ => None,
            }
        }
    }

    #[test]
    fn empty() {
        assert!(compute_basic_blocks::<TestOp>(&[]).is_empty());
    }

    /// 0: Nop
    /// 1: Nop
    /// 2: Return
    ///
    /// Leaders: 0
    #[test]
    fn single_block_no_branches() {
        let ops = vec![TestOp::Nop, TestOp::Nop, TestOp::Return];
        let blocks = compute_basic_blocks(&ops);
        assert_eq!(blocks, vec![BasicBlock { start: 0, end: 3 }]);
    }

    /// 0: Jump(2)
    /// 1: Nop
    /// 2: Return
    ///
    /// Leaders: 0, 1 (fallthrough), 2 (branch target)
    #[test]
    fn jump_adds_fallthrough_leader() {
        let ops = vec![TestOp::Jump(2), TestOp::Nop, TestOp::Return];
        let blocks = compute_basic_blocks(&ops);
        assert_eq!(blocks, vec![
            BasicBlock { start: 0, end: 1 },
            BasicBlock { start: 1, end: 2 },
            BasicBlock { start: 2, end: 3 },
        ]);
    }

    /// 0: CondJump(4) — loop header
    /// 1: Nop         — loop body
    /// 2: Nop
    /// 3: Jump(0)     — back edge
    /// 4: Return
    ///
    /// Leaders: 0, 1 (fallthrough), 4 (branch target)
    #[test]
    fn back_edge_loop() {
        let ops = vec![
            TestOp::CondJump(4),
            TestOp::Nop,
            TestOp::Nop,
            TestOp::Jump(0),
            TestOp::Return,
        ];
        let blocks = compute_basic_blocks(&ops);
        assert_eq!(blocks, vec![
            BasicBlock { start: 0, end: 1 }, // loop header
            BasicBlock { start: 1, end: 4 }, // loop body
            BasicBlock { start: 4, end: 5 }, // exit
        ]);
    }

    /// 0: Jump(0)
    ///
    /// Leaders: 0
    #[test]
    fn branch_to_self() {
        let ops = vec![TestOp::Jump(0)];
        let blocks = compute_basic_blocks(&ops);
        assert_eq!(blocks, vec![BasicBlock { start: 0, end: 1 }]);
    }

    /// 0: CondJump(4)  — if condition
    /// 1: Nop          — then body
    /// 2: Nop
    /// 3: Jump(6)      — skip else
    /// 4: Nop          — else body
    /// 5: Nop
    /// 6: CondJump(9)  — loop header
    /// 7: Nop          — loop body
    /// 8: Jump(6)      — back edge
    /// 9: Return
    ///
    /// Leaders: 0, 1 (fallthrough), 4 (branch target), 6 (branch target),
    ///          7 (fallthrough), 9 (branch target)
    #[test]
    fn if_else_then_loop() {
        let ops = vec![
            TestOp::CondJump(4),
            TestOp::Nop,
            TestOp::Nop,
            TestOp::Jump(6),
            TestOp::Nop,
            TestOp::Nop,
            TestOp::CondJump(9),
            TestOp::Nop,
            TestOp::Jump(6),
            TestOp::Return,
        ];
        let blocks = compute_basic_blocks(&ops);
        assert_eq!(blocks, vec![
            BasicBlock { start: 0, end: 1 },  // if header
            BasicBlock { start: 1, end: 4 },  // then body
            BasicBlock { start: 4, end: 6 },  // else body
            BasicBlock { start: 6, end: 7 },  // loop header
            BasicBlock { start: 7, end: 9 },  // loop body
            BasicBlock { start: 9, end: 10 }, // exit
        ]);
    }

    /// 0: Nop
    /// 1: Nop
    /// 2: Nop
    /// 3: CondJump(1)
    ///
    /// Leaders: 0, 1 (branch target)
    #[test]
    fn cond_jump_back_edge_no_fallthrough() {
        let ops = vec![TestOp::Nop, TestOp::Nop, TestOp::Nop, TestOp::CondJump(1)];
        let blocks = compute_basic_blocks(&ops);
        assert_eq!(blocks, vec![BasicBlock { start: 0, end: 1 }, BasicBlock {
            start: 1,
            end: 4
        },]);
    }

    /// 0: Jump(2)
    /// 1: Jump(2)
    /// 2: Return
    ///
    /// Leaders: 0, 1 (fallthrough), 2 (branch target)
    #[test]
    fn multiple_branches_to_same_leader() {
        let ops = vec![TestOp::Jump(2), TestOp::Jump(2), TestOp::Return];
        let blocks = compute_basic_blocks(&ops);
        assert_eq!(blocks, vec![
            BasicBlock { start: 0, end: 1 },
            BasicBlock { start: 1, end: 2 },
            BasicBlock { start: 2, end: 3 },
        ]);
    }
}
