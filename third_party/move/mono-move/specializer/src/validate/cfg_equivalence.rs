// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! CFG equivalence between the final IR and the original bytecode, checked
//! under the carried [`TranslationWitness`].
//!
//! The check is linear: the witness supplies the block bijection, so no
//! isomorphism search is needed. Every edge is checked on both sides — the
//! IR exit kind against the bytecode opcode at the block's exit — so a
//! dropped, inserted, redirected, or kind-swapped edge fails even when the
//! witness is corrupted consistently with the bug.
//!
//! The pass is structural: it certifies edges, not the semantics that
//! select them.
//!
//! The bytecode-side oracle is [`VMControlFlowGraph`] from
//! `move-binary-format`, not the specializer's own block splitter.

use super::{FuncCtx, TranslationWitness};
use crate::stackless_exec_ir::{
    instr_utils::{classify_exit, exit_of_instr, BlockExit, CondJumpKind, ExitKind},
    BasicBlock, NamedSlot,
};
use mono_move_core::{ExecutionErrorKind, IntoExecutionError};
use move_binary_format::{
    control_flow_graph::{ControlFlowGraph, VMControlFlowGraph},
    file_format::{Bytecode, CodeOffset},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum CfgEquivalenceError {
    #[error("witness has {witness_len} entries for {num_blocks} IR blocks")]
    WitnessLengthMismatch {
        witness_len: usize,
        num_blocks: usize,
    },

    #[error("block L{label}: terminator {opcode} at non-final position {position}")]
    InteriorTerminator {
        label: u16,
        position: usize,
        opcode: &'static str,
    },

    #[error("{num_ir_blocks} IR blocks vs {num_bytecode_blocks} bytecode blocks")]
    BlockCountMismatch {
        num_ir_blocks: usize,
        num_bytecode_blocks: usize,
    },

    #[error("label L{label} is out of witness range")]
    LabelOutOfRange { label: u16 },

    #[error(
        "block {block_idx} maps to offset {offset}, not after predecessor's offset {prev} \
         (blocks must map to strictly increasing bytecode offsets)"
    )]
    WitnessNotMonotone {
        block_idx: usize,
        offset: CodeOffset,
        prev: CodeOffset,
    },

    #[error("block L{label} maps to bytecode offset {offset}, expected block start {expected}")]
    BlockStartMismatch {
        label: u16,
        offset: CodeOffset,
        expected: CodeOffset,
    },

    #[error(
        "block L{label} (bytecode block at {start}): IR exits via {ir_exit} but bytecode \
         ends with {bytecode_exit}"
    )]
    ExitKindMismatch {
        label: u16,
        start: CodeOffset,
        ir_exit: &'static str,
        bytecode_exit: String,
    },

    #[error(
        "block L{label} (bytecode block at {start}): IR branch target maps to offset {got}, \
         bytecode branch operand is {expected}"
    )]
    TargetMismatch {
        label: u16,
        start: CodeOffset,
        got: CodeOffset,
        expected: CodeOffset,
    },

    #[error(
        "block L{label} (bytecode block at {start}): next block in layout order maps to \
         offset {got}, bytecode falls through to {exit_pc} + 1"
    )]
    FallthroughMismatch {
        label: u16,
        start: CodeOffset,
        got: CodeOffset,
        exit_pc: CodeOffset,
    },

    #[error("last block L{label} must end in an unconditional jump or a function exit")]
    LastBlockFallsThrough { label: u16 },
}

impl IntoExecutionError for CfgEquivalenceError {
    fn kind(&self) -> ExecutionErrorKind {
        match self {
            CfgEquivalenceError::WitnessLengthMismatch { .. }
            | CfgEquivalenceError::InteriorTerminator { .. }
            | CfgEquivalenceError::BlockCountMismatch { .. }
            | CfgEquivalenceError::LabelOutOfRange { .. }
            | CfgEquivalenceError::WitnessNotMonotone { .. }
            | CfgEquivalenceError::BlockStartMismatch { .. }
            | CfgEquivalenceError::ExitKindMismatch { .. }
            | CfgEquivalenceError::TargetMismatch { .. }
            | CfgEquivalenceError::FallthroughMismatch { .. }
            | CfgEquivalenceError::LastBlockFallsThrough { .. } => {
                ExecutionErrorKind::InvariantViolation
            },
        }
    }
}

/// Certified block bijection between IR blocks and bytecode blocks. Only
/// [`verify`] constructs one, so holding it is proof that CFG equivalence
/// passed; later passes consume it instead of re-trusting the raw witness.
pub(crate) struct BlockCorrespondence {
    /// For each IR block in layout order, the start offset of its bytecode
    /// block. Strictly increasing; exactly the oracle's block starts.
    block_starts: Vec<CodeOffset>,
}

impl BlockCorrespondence {
    /// Start offset of the bytecode block corresponding to IR block
    /// `block_idx`. Pair with the bytecode CFG's `block_end` to recover the
    /// block's full instruction range.
    pub(crate) fn bytecode_block_start(&self, block_idx: usize) -> Option<CodeOffset> {
        self.block_starts.get(block_idx).copied()
    }
}

/// Verify that the IR's control-flow graph is isomorphic to the bytecode's,
/// edge-by-edge and kind-by-kind, under the carried witness.
///
/// Three steps, each O(code length):
///
/// 1. **Shape**: one witness entry per block, and terminators only in final
///    position (the exit classification reads only the last instruction).
/// 2. **Bijection**: `witness[blocks[i].label]` strictly increases and
///    equals the oracle's ascending block-start list. This makes the
///    witness a bijection onto bytecode blocks, pins IR layout order to
///    bytecode order, and makes all later oracle lookups safe.
/// 3. **Edges**: each exit kind pins both the bytecode opcode at the
///    block's exit and the mapped target labels.
pub(crate) fn verify(ctx: &FuncCtx) -> Result<BlockCorrespondence, CfgEquivalenceError> {
    let blocks = &ctx.func_ir.blocks;
    let witness = &ctx.func_ir.witness;

    check_shape(blocks, witness)?;
    let block_starts = check_bijection(blocks, witness, &ctx.bytecode_cfg)?;
    check_edges(blocks, witness, &block_starts, ctx)?;

    Ok(BlockCorrespondence { block_starts })
}

fn check_shape(
    blocks: &[BasicBlock<NamedSlot>],
    witness: &TranslationWitness,
) -> Result<(), CfgEquivalenceError> {
    if witness.label_to_offset.len() != blocks.len() {
        return Err(CfgEquivalenceError::WitnessLengthMismatch {
            witness_len: witness.label_to_offset.len(),
            num_blocks: blocks.len(),
        });
    }
    for block in blocks {
        let interior_len = block.instrs.len().saturating_sub(1);
        for (position, instr) in block.instrs[..interior_len].iter().enumerate() {
            if exit_of_instr(instr).is_some() {
                return Err(CfgEquivalenceError::InteriorTerminator {
                    label: block.label.0,
                    position,
                    opcode: instr.opcode_name(),
                });
            }
        }
    }
    Ok(())
}

fn check_bijection(
    blocks: &[BasicBlock<NamedSlot>],
    witness: &TranslationWitness,
    bytecode_cfg: &VMControlFlowGraph,
) -> Result<Vec<CodeOffset>, CfgEquivalenceError> {
    // Sorted defensively: `VMControlFlowGraph::new` inserts blocks in
    // ascending start order today, but nothing here should depend on that.
    let mut oracle_starts = bytecode_cfg.blocks();
    oracle_starts.sort_unstable();

    if blocks.len() != oracle_starts.len() {
        return Err(CfgEquivalenceError::BlockCountMismatch {
            num_ir_blocks: blocks.len(),
            num_bytecode_blocks: oracle_starts.len(),
        });
    }

    // Strict monotonicity first (a full pass, so a permuted witness or a
    // duplicated label is diagnosed as such), then elementwise equality
    // with the oracle's ascending block starts.
    let mut block_starts: Vec<CodeOffset> = Vec::with_capacity(blocks.len());
    for (block_idx, block) in blocks.iter().enumerate() {
        let offset = lookup(witness, block.label.0)?;
        if let Some(&prev) = block_starts.last()
            && offset <= prev
        {
            return Err(CfgEquivalenceError::WitnessNotMonotone {
                block_idx,
                offset,
                prev,
            });
        }
        block_starts.push(offset);
    }
    for (block_idx, block) in blocks.iter().enumerate() {
        if block_starts[block_idx] != oracle_starts[block_idx] {
            return Err(CfgEquivalenceError::BlockStartMismatch {
                label: block.label.0,
                offset: block_starts[block_idx],
                expected: oracle_starts[block_idx],
            });
        }
    }
    Ok(block_starts)
}

fn check_edges(
    blocks: &[BasicBlock<NamedSlot>],
    witness: &TranslationWitness,
    block_starts: &[CodeOffset],
    ctx: &FuncCtx,
) -> Result<(), CfgEquivalenceError> {
    for (block_idx, block) in blocks.iter().enumerate() {
        let start = block_starts[block_idx];
        // Safe: `check_bijection` established `start` is an oracle block id.
        let exit_pc = ctx.bytecode_cfg.block_end(start);
        let exit_instr = &ctx.code[exit_pc as usize];
        let label = block.label.0;

        let exit = classify_exit(block);
        // For a non-fallthrough exit the last instruction is the terminator
        // the classification was derived from, so its opcode names the exit.
        let ir_exit = match block.instrs.last() {
            Some(instr) if !matches!(exit, BlockExit::FallThrough) => instr.opcode_name(),
            _ => "fallthrough (no terminator)",
        };
        let kind_mismatch = || CfgEquivalenceError::ExitKindMismatch {
            label,
            start,
            ir_exit,
            bytecode_exit: format!("{exit_instr:?}"),
        };

        match exit {
            // Unconditional jump: the bytecode block must end in `Branch`,
            // and the mapped target must equal its operand.
            BlockExit::Jump { target } => {
                let Bytecode::Branch(operand) = exit_instr else {
                    return Err(kind_mismatch());
                };
                check_target(witness, label, start, target.0, *operand)?;
            },
            // Conditional jump: `BrTrue`/`BrFalse` must match the same
            // bytecode variant (conversion preserves it); `BrCmp`/`BrCmpImm`
            // accept either, since fusion may negate the comparison operator
            // (the operator itself is unchecked). The taken edge is pinned by
            // the branch operand, the fallthrough edge positionally.
            BlockExit::CondJump { taken, kind } => {
                let operand = match (kind, exit_instr) {
                    (CondJumpKind::BrTrue, Bytecode::BrTrue(operand))
                    | (CondJumpKind::BrFalse, Bytecode::BrFalse(operand))
                    | (
                        CondJumpKind::BrCmp | CondJumpKind::BrCmpImm,
                        Bytecode::BrTrue(operand) | Bytecode::BrFalse(operand),
                    ) => *operand,
                    _ => return Err(kind_mismatch()),
                };
                check_target(witness, label, start, taken.0, operand)?;
                check_fallthrough(block_starts, block_idx, label, start, exit_pc)?;
            },
            // Plain fallthrough (including an empty block): the bytecode
            // block must end in an ordinary instruction — a branch here is
            // an edge the IR dropped. `is_branch` is the same predicate
            // `VMControlFlowGraph` splits blocks on.
            BlockExit::FallThrough => {
                if exit_instr.is_branch() {
                    return Err(kind_mismatch());
                }
                check_fallthrough(block_starts, block_idx, label, start, exit_pc)?;
            },
            // Function exit: exact opcode correspondence. Exits translate
            // 1:1 with no fusion.
            BlockExit::Exit(kind) => {
                let opcode_matches = matches!(
                    (kind, exit_instr),
                    (ExitKind::Ret, Bytecode::Ret)
                        | (ExitKind::Abort, Bytecode::Abort)
                        | (ExitKind::AbortMsg, Bytecode::AbortMsg)
                );
                if !opcode_matches {
                    return Err(kind_mismatch());
                }
            },
        }
    }
    Ok(())
}

/// Fallible witness lookup: a corrupted label must surface as an error,
/// never an index panic.
fn lookup(witness: &TranslationWitness, label: u16) -> Result<CodeOffset, CfgEquivalenceError> {
    witness
        .label_to_offset
        .get(label as usize)
        .copied()
        .ok_or(CfgEquivalenceError::LabelOutOfRange { label })
}

/// The mapped jump target must equal the bytecode branch operand.
fn check_target(
    witness: &TranslationWitness,
    label: u16,
    start: CodeOffset,
    target_label: u16,
    operand: CodeOffset,
) -> Result<(), CfgEquivalenceError> {
    let got = lookup(witness, target_label)?;
    if got != operand {
        return Err(CfgEquivalenceError::TargetMismatch {
            label,
            start,
            got,
            expected: operand,
        });
    }
    Ok(())
}

/// The next block in layout order must map to `exit_pc + 1`; the last
/// block must not fall through. The offset equality is already implied by
/// `check_bijection` (the oracle partition is contiguous) and is re-checked
/// here as defense in depth.
fn check_fallthrough(
    block_starts: &[CodeOffset],
    block_idx: usize,
    label: u16,
    start: CodeOffset,
    exit_pc: CodeOffset,
) -> Result<(), CfgEquivalenceError> {
    let next_start = block_starts
        .get(block_idx + 1)
        .copied()
        .ok_or(CfgEquivalenceError::LastBlockFallsThrough { label })?;
    // Widened: `exit_pc + 1` cannot overflow in u32.
    if next_start as u32 != exit_pc as u32 + 1 {
        return Err(CfgEquivalenceError::FallthroughMismatch {
            label,
            start,
            got: next_start,
            exit_pc,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stackless_exec_ir::{
        BasicBlock, CmpKind, FunctionIR, HomeIndex, ImmValue, Instr, InstrSeq, Label, NamedSlot,
    };
    use move_binary_format::file_format::{
        Bytecode, FunctionDefinitionIndex, FunctionHandleIndex, IdentifierIndex,
    };

    fn slot() -> NamedSlot {
        NamedSlot::Home(HomeIndex(0))
    }

    /// Any ordinary (non-terminator) instruction.
    fn ld() -> Instr<NamedSlot> {
        Instr::LdImm {
            dst: slot(),
            imm: ImmValue::Bool(true),
        }
    }

    fn block(label: u16, instrs: Vec<Instr<NamedSlot>>) -> BasicBlock<NamedSlot> {
        BasicBlock {
            label: Label(label),
            instrs: InstrSeq::for_tests(instrs),
        }
    }

    fn make_func(blocks: Vec<BasicBlock<NamedSlot>>, witness: Vec<CodeOffset>) -> FunctionIR {
        FunctionIR {
            name_idx: IdentifierIndex(0),
            handle_idx: FunctionHandleIndex(0),
            def_idx: FunctionDefinitionIndex(0),
            num_params: 0,
            num_locals: 0,
            num_home_slots: 1,
            num_transfer_positions: 0,
            blocks,
            home_slot_types: Vec::new(),
            block_costs: Vec::new(),
            witness: TranslationWitness {
                label_to_offset: witness,
            },
        }
    }

    fn run(
        func_ir: &FunctionIR,
        code: &[Bytecode],
    ) -> Result<BlockCorrespondence, CfgEquivalenceError> {
        let ctx = FuncCtx {
            func_ir,
            code,
            bytecode_cfg: VMControlFlowGraph::new(code),
        };
        verify(&ctx)
    }

    /// Diamond covering all four exit shapes:
    ///
    /// ```text
    /// 0: LdTrue          \ block A [0..1]: conditional jump to 4
    /// 1: BrTrue(4)       /
    /// 2: LdTrue          \ block B [2..3]: unconditional jump to 5
    /// 3: Branch(5)       /
    /// 4: LdTrue            block C [4..4]: plain fallthrough into 5
    /// 5: Ret               block D [5..5]: function exit
    /// ```
    fn diamond_code() -> Vec<Bytecode> {
        vec![
            Bytecode::LdTrue,
            Bytecode::BrTrue(4),
            Bytecode::LdTrue,
            Bytecode::Branch(5),
            Bytecode::LdTrue,
            Bytecode::Ret,
        ]
    }

    /// Correct IR for [`diamond_code`], with labels equal to positions.
    fn diamond_ir() -> Vec<BasicBlock<NamedSlot>> {
        vec![
            block(0, vec![ld(), Instr::BrTrue {
                target: Label(2),
                cond: slot(),
            }]),
            block(1, vec![ld(), Instr::Branch { target: Label(3) }]),
            block(2, vec![ld()]),
            block(3, vec![Instr::Ret { srcs: Box::new([]) }]),
        ]
    }

    fn diamond_witness() -> Vec<CodeOffset> {
        vec![0, 2, 4, 5]
    }

    #[test]
    fn diamond_passes_and_certifies_correspondence() {
        let func = make_func(diamond_ir(), diamond_witness());
        let correspondence = run(&func, &diamond_code()).expect("correct translation validates");
        for (block_idx, expected) in [0u16, 2, 4, 5].into_iter().enumerate() {
            assert_eq!(
                correspondence.bytecode_block_start(block_idx),
                Some(expected)
            );
        }
        assert_eq!(correspondence.bytecode_block_start(4), None);
    }

    /// Labels are ids, not positions: the converter assigns them in
    /// branch-target-encounter order. A permuted (but truthful) label
    /// assignment must validate.
    #[test]
    fn permuted_labels_pass() {
        // Converter-style assignment for the diamond: offset 4 -> L0
        // (BrTrue target), offset 2 -> L1 (its fallthrough), offset 5 ->
        // L2 (Branch target), offset 0 -> L3 (on demand).
        let blocks = vec![
            block(3, vec![ld(), Instr::BrTrue {
                target: Label(0),
                cond: slot(),
            }]),
            block(1, vec![ld(), Instr::Branch { target: Label(2) }]),
            block(0, vec![ld()]),
            block(2, vec![Instr::Ret { srcs: Box::new([]) }]),
        ];
        let func = make_func(blocks, vec![4, 2, 5, 0]);
        run(&func, &diamond_code()).expect("permuted labels validate");
    }

    /// A `Nop`-only bytecode block converts to an *empty* IR block, which
    /// classifies as fallthrough.
    #[test]
    fn empty_block_falls_through() {
        let code = vec![
            Bytecode::LdTrue,
            Bytecode::BrTrue(3),
            Bytecode::Nop,
            Bytecode::Ret,
        ];
        let blocks = vec![
            block(0, vec![ld(), Instr::BrTrue {
                target: Label(2),
                cond: slot(),
            }]),
            block(1, vec![]),
            block(2, vec![Instr::Ret { srcs: Box::new([]) }]),
        ];
        let func = make_func(blocks, vec![0, 2, 3]);
        run(&func, &code).expect("empty block validates as fallthrough");
    }

    /// A conditional whose target is its own fallthrough is legal and must
    /// not be confused with an unconditional jump (the oracle's successor
    /// list dedups it; the operand-based check does not).
    #[test]
    fn degenerate_conditional_passes() {
        let code = vec![Bytecode::LdTrue, Bytecode::BrTrue(2), Bytecode::Ret];
        let blocks = vec![
            block(0, vec![ld(), Instr::BrTrue {
                target: Label(1),
                cond: slot(),
            }]),
            block(1, vec![Instr::Ret { srcs: Box::new([]) }]),
        ];
        let func = make_func(blocks, vec![0, 2]);
        run(&func, &code).expect("degenerate conditional validates");
    }

    /// Compare-branch fusion keeps the taken target and negates the
    /// operator for `BrFalse`, so `BrCmp` corresponds to either variant.
    #[test]
    fn fused_compare_branch_accepts_either_bytecode_polarity() {
        for conditional in [Bytecode::BrTrue(2), Bytecode::BrFalse(2)] {
            let code = vec![Bytecode::LdTrue, conditional, Bytecode::Ret];
            let blocks = vec![
                block(0, vec![Instr::BrCmp {
                    target: Label(1),
                    op: CmpKind::Eq,
                    lhs: slot(),
                    rhs: slot(),
                }]),
                block(1, vec![Instr::Ret { srcs: Box::new([]) }]),
            ];
            let func = make_func(blocks, vec![0, 2]);
            run(&func, &code).expect("fused compare-branch validates");
        }
    }

    /// Dropped conditional edge: the IR conditional was deleted, leaving a
    /// fallthrough block. The fallthrough arm pins the bytecode exit
    /// opcode, so this fails even though the positional check would pass.
    #[test]
    fn dropped_edge_fails() {
        let mut blocks = diamond_ir();
        blocks[0].instrs = InstrSeq::for_tests(vec![ld()]);
        let func = make_func(blocks, diamond_witness());
        assert!(matches!(
            run(&func, &diamond_code()),
            Err(CfgEquivalenceError::ExitKindMismatch { .. })
        ));
    }

    /// Inserted conditional edge: `Branch` miscompiled to `BrCmp` with the
    /// same target adds a fallthrough edge the bytecode does not have.
    #[test]
    fn inserted_edge_fails() {
        let mut blocks = diamond_ir();
        blocks[1].instrs = InstrSeq::for_tests(vec![ld(), Instr::BrCmp {
            target: Label(3),
            op: CmpKind::Eq,
            lhs: slot(),
            rhs: slot(),
        }]);
        let func = make_func(blocks, diamond_witness());
        assert!(matches!(
            run(&func, &diamond_code()),
            Err(CfgEquivalenceError::ExitKindMismatch { .. })
        ));
    }

    /// Conditional degenerating to an unconditional: IR `Branch` where the
    /// bytecode has `BrTrue` — caught by the Jump arm's opcode pin.
    #[test]
    fn conditional_to_unconditional_degeneration_fails() {
        let mut blocks = diamond_ir();
        blocks[0].instrs = InstrSeq::for_tests(vec![ld(), Instr::Branch { target: Label(2) }]);
        let func = make_func(blocks, diamond_witness());
        assert!(matches!(
            run(&func, &diamond_code()),
            Err(CfgEquivalenceError::ExitKindMismatch { .. })
        ));
    }

    /// Taken/fallthrough swap with set-equal successors: the taken edge
    /// points at the fallthrough block. Successor-set comparison would
    /// accept this; the operand-pinned taken edge does not.
    #[test]
    fn swapped_conditional_targets_fail() {
        let mut blocks = diamond_ir();
        blocks[0].instrs = InstrSeq::for_tests(vec![ld(), Instr::BrTrue {
            target: Label(1),
            cond: slot(),
        }]);
        let func = make_func(blocks, diamond_witness());
        assert!(matches!(
            run(&func, &diamond_code()),
            Err(CfgEquivalenceError::TargetMismatch { .. })
        ));

        // The layout-order variant of the same swap: blocks reordered so
        // the taken block sits in the fallthrough position — caught by the
        // monotonicity clause that pins layout order to bytecode order.
        let mut blocks = diamond_ir();
        blocks.swap(1, 2);
        let func = make_func(blocks, diamond_witness());
        assert!(matches!(
            run(&func, &diamond_code()),
            Err(CfgEquivalenceError::WitnessNotMonotone { .. })
        ));
    }

    #[test]
    fn redirected_target_fails() {
        let mut blocks = diamond_ir();
        blocks[0].instrs = InstrSeq::for_tests(vec![ld(), Instr::BrTrue {
            target: Label(3),
            cond: slot(),
        }]);
        let func = make_func(blocks, diamond_witness());
        assert!(matches!(
            run(&func, &diamond_code()),
            Err(CfgEquivalenceError::TargetMismatch { .. })
        ));
    }

    /// Unfused conditionals are variant-preserving 1:1, so a
    /// `BrTrue`/`BrFalse` swap (same target, inverted branch) is caught.
    #[test]
    fn conditional_variant_swap_fails() {
        let mut blocks = diamond_ir();
        blocks[0].instrs = InstrSeq::for_tests(vec![ld(), Instr::BrFalse {
            target: Label(2),
            cond: slot(),
        }]);
        let func = make_func(blocks, diamond_witness());
        assert!(matches!(
            run(&func, &diamond_code()),
            Err(CfgEquivalenceError::ExitKindMismatch { .. })
        ));
    }

    /// IR `Abort` where the bytecode returns: exits must match exactly.
    #[test]
    fn exit_kind_swap_fails() {
        let mut blocks = diamond_ir();
        blocks[3].instrs = InstrSeq::for_tests(vec![Instr::Abort { code: slot() }]);
        let func = make_func(blocks, diamond_witness());
        assert!(matches!(
            run(&func, &diamond_code()),
            Err(CfgEquivalenceError::ExitKindMismatch { .. })
        ));
    }

    /// A terminator at a non-final position would execute before the
    /// block's nominal exit; the classification may only trust the last
    /// instruction after this scan.
    #[test]
    fn interior_terminator_fails() {
        let mut blocks = diamond_ir();
        blocks[2].instrs = InstrSeq::for_tests(vec![Instr::Branch { target: Label(3) }, ld()]);
        let func = make_func(blocks, diamond_witness());
        assert!(matches!(
            run(&func, &diamond_code()),
            Err(CfgEquivalenceError::InteriorTerminator { .. })
        ));
    }

    /// Two blocks sharing a label alias onto one bytecode block (and
    /// lowering's label table is last-write-wins) — caught because the
    /// composed offsets repeat, breaking strict monotonicity.
    #[test]
    fn duplicated_label_fails() {
        let mut blocks = diamond_ir();
        blocks[2].label = Label(1);
        let func = make_func(blocks, diamond_witness());
        assert!(matches!(
            run(&func, &diamond_code()),
            Err(CfgEquivalenceError::WitnessNotMonotone { .. })
        ));
    }

    #[test]
    fn label_out_of_range_fails() {
        let mut blocks = diamond_ir();
        blocks[0].label = Label(9);
        let func = make_func(blocks, diamond_witness());
        assert!(matches!(
            run(&func, &diamond_code()),
            Err(CfgEquivalenceError::LabelOutOfRange { .. })
        ));
    }

    /// A corrupted target label (not a corrupted block label) must also
    /// surface as an error, never an index panic.
    #[test]
    fn out_of_range_target_label_fails() {
        let mut blocks = diamond_ir();
        blocks[1].instrs = InstrSeq::for_tests(vec![ld(), Instr::Branch { target: Label(9) }]);
        let func = make_func(blocks, diamond_witness());
        assert!(matches!(
            run(&func, &diamond_code()),
            Err(CfgEquivalenceError::LabelOutOfRange { .. })
        ));
    }

    /// A witness offset pointing mid-block passes distinctness and
    /// monotonicity but is not an oracle block start.
    #[test]
    fn mid_block_witness_offset_fails() {
        let func = make_func(diamond_ir(), vec![0, 2, 3, 5]);
        assert!(matches!(
            run(&func, &diamond_code()),
            Err(CfgEquivalenceError::BlockStartMismatch { .. })
        ));
    }

    #[test]
    fn permuted_witness_fails() {
        let func = make_func(diamond_ir(), vec![0, 4, 2, 5]);
        assert!(matches!(
            run(&func, &diamond_code()),
            Err(CfgEquivalenceError::WitnessNotMonotone { .. })
        ));
    }

    /// Merged blocks (fewer IR blocks than bytecode blocks) fail totality.
    #[test]
    fn block_count_mismatch_fails() {
        let blocks = vec![
            block(0, vec![ld(), Instr::BrTrue {
                target: Label(2),
                cond: slot(),
            }]),
            block(1, vec![ld(), Instr::Branch { target: Label(2) }]),
            block(2, vec![Instr::Ret { srcs: Box::new([]) }]),
        ];
        let func = make_func(blocks, vec![0, 2, 5]);
        assert!(matches!(
            run(&func, &diamond_code()),
            Err(CfgEquivalenceError::BlockCountMismatch { .. })
        ));
    }

    #[test]
    fn witness_length_mismatch_fails() {
        let func = make_func(diamond_ir(), vec![0, 2, 4]);
        assert!(matches!(
            run(&func, &diamond_code()),
            Err(CfgEquivalenceError::WitnessLengthMismatch { .. })
        ));
    }

    /// The last block may not fall through (verified bytecode ends in an
    /// unconditional exit); the lookup of `blocks[i + 1]` must be an
    /// error, not a panic.
    #[test]
    fn last_block_fallthrough_fails() {
        let code = vec![Bytecode::LdTrue];
        let func = make_func(vec![block(0, vec![ld()])], vec![0]);
        assert!(matches!(
            run(&func, &code),
            Err(CfgEquivalenceError::LastBlockFallsThrough { .. })
        ));
    }
}
