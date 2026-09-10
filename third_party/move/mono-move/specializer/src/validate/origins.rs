// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Bytecode-offset provenance checks on the final IR.
//!
//! Consumes the certified [`BlockCorrespondence`], so block ranges are trusted.
//! Two properties:
//!
//! - Containment: every instruction's origin lies inside its block's
//!   corresponding bytecode block.
//! - Monotonicity: origins are non-decreasing within a block. This holds
//!   only because no pass reorders instructions; under that assumption, a
//!   decrease means an instruction was separated from its origin. A pass
//!   that deliberately reorders (moving instructions and origins together)
//!   would be correct yet non-monotone — adding one requires retiring this
//!   check.

use super::{cfg_equivalence::BlockCorrespondence, FuncCtx};
use mono_move_core::{BytecodeOffset, ExecutionErrorKind, IntoExecutionError};
use move_binary_format::control_flow_graph::ControlFlowGraph;
use thiserror::Error;

/// Check containment and monotonicity of every instruction origin against
/// the certified block correspondence.
pub(crate) fn verify(
    ctx: &FuncCtx,
    correspondence: &BlockCorrespondence,
) -> Result<(), OriginsError> {
    for (block_idx, block) in ctx.func_ir.blocks.iter().enumerate() {
        let block_start = correspondence.bytecode_block_start(block_idx).ok_or(
            OriginsError::MissingCorrespondence {
                label: block.label.0,
            },
        )?;
        let block_end = ctx.bytecode_cfg.block_end(block_start);

        let mut previous: Option<BytecodeOffset> = None;
        for (position, (instr, origin)) in block.instrs.iter_with_origins().enumerate() {
            if origin < block_start || origin > block_end {
                return Err(OriginsError::OriginOutsideBlock {
                    label: block.label.0,
                    position,
                    opcode: instr.opcode_name(),
                    origin,
                    block_start,
                    block_end,
                });
            }
            if let Some(previous) = previous
                && origin < previous
            {
                return Err(OriginsError::OriginsNotMonotone {
                    label: block.label.0,
                    position,
                    opcode: instr.opcode_name(),
                    origin,
                    previous,
                });
            }
            previous = Some(origin);
        }
    }
    Ok(())
}

#[derive(Debug, Error)]
pub(crate) enum OriginsError {
    #[error("block L{label} has no bytecode correspondence entry")]
    MissingCorrespondence { label: u16 },

    #[error(
        "block L{label}, instruction {position} ({opcode}): origin {origin} outside its \
         bytecode block {block_start}..={block_end}"
    )]
    OriginOutsideBlock {
        label: u16,
        position: usize,
        opcode: &'static str,
        origin: BytecodeOffset,
        block_start: BytecodeOffset,
        block_end: BytecodeOffset,
    },

    #[error(
        "block L{label}, instruction {position} ({opcode}): origin {origin} decreases below \
         predecessor's {previous}"
    )]
    OriginsNotMonotone {
        label: u16,
        position: usize,
        opcode: &'static str,
        origin: BytecodeOffset,
        previous: BytecodeOffset,
    },
}

impl IntoExecutionError for OriginsError {
    fn kind(&self) -> ExecutionErrorKind {
        use OriginsError::*;
        match self {
            MissingCorrespondence { .. }
            | OriginOutsideBlock { .. }
            | OriginsNotMonotone { .. } => ExecutionErrorKind::InvariantViolation,
        }
    }
}
