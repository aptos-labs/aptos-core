// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Transfer-slot discipline on the final IR.
//!
//! Verifies the subset of the transfer invariants (see the header of
//! `destack/analysis.rs`) that survives slot allocation:
//!
//! 1. Every use of `Transfer(j)` is preceded by a def in the same block.
//! 2. At a call boundary, every bound Transfer slot is consumed by the
//!    call's args (orphans signal an upstream regression).
//! 3. Calls clobber every Transfer slot; ret defs re-bind on the same
//!    instruction.
//! 4. No Transfer binding leaks across a block boundary.
//! 5. Per-call structural invariants — arg positionality, return
//!    Transfer prefix, return monotonicity — via
//!    [`check_call_structural_invariants`], the same helper the
//!    SSA-level `assert_transfer_invariants` uses on the ValueId-level
//!    maps.
//!
//! In addition, every slot operand is range-checked against the function's
//! declared Transfer/Home counts, and duplicate uses of one Transfer
//! position within a single instruction are rejected.

use super::FuncCtx;
use crate::stackless_exec_ir::{
    instr_utils::{call_boundary_rets_and_args, for_each_def, for_each_slot, for_each_value_use},
    NamedSlot, TransferPosition,
};
use mono_move_core::{ExecutionErrorKind, IntoExecutionError};
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum TransferVerifierError {
    #[error("block {block}, instr {instr}: {inner}")]
    TransferCallStructural {
        block: usize,
        instr: usize,
        inner: Box<TransferVerifierError>,
    },

    #[error(
        "arg positionality: args[{arg_idx}] resolves to Transfer({got}), expected Transfer({arg_idx})"
    )]
    TransferArgPositionality { arg_idx: usize, got: u16 },

    #[error(
        "return Transfer prefix: rets[{ret_idx}] resolves to Transfer({got}) after a non-Transfer ret"
    )]
    TransferReturnPrefix { ret_idx: usize, got: u16 },

    #[error("return monotonicity: rets[{ret_idx}] = Transfer({got}) <= prev Transfer({prev})")]
    TransferReturnNotMonotonic { ret_idx: usize, got: u16, prev: u16 },

    #[error("block {block}, instr {instr}: use of Transfer({transfer}) with no live def earlier in this block")]
    TransferUseWithoutLiveDef {
        block: usize,
        instr: usize,
        transfer: u16,
    },

    #[error("block {block}, instr {instr}: Transfer({transfer}) bound at call boundary but not consumed as args[{transfer}]")]
    TransferBoundNotConsumed {
        block: usize,
        instr: usize,
        transfer: u16,
    },

    #[error("block {block}: Transfer({transfer}) bound at block end (Transfer lifetimes must be block-local)")]
    TransferBoundAtBlockEnd { block: usize, transfer: u16 },

    #[error("block {block}, instr {instr}: Transfer({transfer}) out of range (function declares {num_transfer} positions)")]
    TransferPositionOutOfRange {
        block: usize,
        instr: usize,
        transfer: u16,
        num_transfer: usize,
    },

    #[error("block {block}, instr {instr}: Home({home}) out of range (function declares {num_home_slots} home slots)")]
    HomeSlotOutOfRange {
        block: usize,
        instr: usize,
        home: u16,
        num_home_slots: usize,
    },

    #[error(
        "block {block}, instr {instr}: duplicate use of Transfer({transfer}) in one instruction"
    )]
    DuplicateTransferUse {
        block: usize,
        instr: usize,
        transfer: u16,
    },
}

impl IntoExecutionError for TransferVerifierError {
    fn kind(&self) -> ExecutionErrorKind {
        match self {
            TransferVerifierError::TransferCallStructural { .. }
            | TransferVerifierError::TransferArgPositionality { .. }
            | TransferVerifierError::TransferReturnPrefix { .. }
            | TransferVerifierError::TransferReturnNotMonotonic { .. }
            | TransferVerifierError::TransferUseWithoutLiveDef { .. }
            | TransferVerifierError::TransferBoundNotConsumed { .. }
            | TransferVerifierError::TransferBoundAtBlockEnd { .. }
            | TransferVerifierError::TransferPositionOutOfRange { .. }
            | TransferVerifierError::HomeSlotOutOfRange { .. }
            | TransferVerifierError::DuplicateTransferUse { .. } => {
                ExecutionErrorKind::InvariantViolation
            },
        }
    }
}

/// Check the per-call structural Transfer invariants:
///
/// - **Arg positionality** (invariant 2): if `args[j]` resolves to
///   `Transfer(i)`, then `i == j`.
/// - **Return Transfer prefix** (invariant 5): the rets list is a
///   (possibly empty) Transfer-resolved prefix followed by a non-Transfer
///   suffix.
/// - **Return monotonicity** (invariant 3): within the Transfer prefix,
///   resolved Transfer indices strictly increase.
pub(crate) fn check_call_structural_invariants<SlotForm, F>(
    args: &[SlotForm],
    rets: &[SlotForm],
    transfer_pos: F,
) -> Result<(), TransferVerifierError>
where
    F: Fn(&SlotForm) -> Option<u16>,
{
    for (j, slot) in args.iter().enumerate() {
        if let Some(i) = transfer_pos(slot)
            && i as usize != j
        {
            return Err(TransferVerifierError::TransferArgPositionality { arg_idx: j, got: i });
        }
    }

    let mut seen_non_transfer = false;
    let mut last_transfer: Option<u16> = None;
    for (k, slot) in rets.iter().enumerate() {
        match transfer_pos(slot) {
            Some(i) => {
                if seen_non_transfer {
                    return Err(TransferVerifierError::TransferReturnPrefix { ret_idx: k, got: i });
                }
                if let Some(prev) = last_transfer
                    && i <= prev
                {
                    return Err(TransferVerifierError::TransferReturnNotMonotonic {
                        ret_idx: k,
                        got: i,
                        prev,
                    });
                }
                last_transfer = Some(i);
            },
            None => seen_non_transfer = true,
        }
    }

    Ok(())
}

/// Verify the Transfer-slot discipline on the final named-slot IR (see the
/// module header for the exact invariant subset).
pub(crate) fn verify(ctx: &FuncCtx) -> Result<(), TransferVerifierError> {
    let func = ctx.func_ir;
    let num_transfer = func.num_transfer_positions as usize;
    let num_home_slots = func.num_home_slots as usize;
    let mut bound: Vec<bool> = vec![false; num_transfer];
    for (b_idx, block) in func.blocks.iter().enumerate() {
        // Block-local lifetime: a fresh state at every block.
        bound.fill(false);
        for (i, instr) in block.instrs.iter().enumerate() {
            // Every slot operand must be within the function's declared
            // ranges; an out-of-range slot (a slot-allocation bug) must
            // surface as an error, not an index panic here or at lowering.
            let mut out_of_range: Option<TransferVerifierError> = None;
            for_each_slot(instr, |slot| {
                if out_of_range.is_some() {
                    return;
                }
                match slot {
                    NamedSlot::Transfer(position) if position.0 as usize >= num_transfer => {
                        out_of_range = Some(TransferVerifierError::TransferPositionOutOfRange {
                            block: b_idx,
                            instr: i,
                            transfer: position.0,
                            num_transfer,
                        });
                    },
                    NamedSlot::Home(home) if home.0 as usize >= num_home_slots => {
                        out_of_range = Some(TransferVerifierError::HomeSlotOutOfRange {
                            block: b_idx,
                            instr: i,
                            home: home.0,
                            num_home_slots,
                        });
                    },
                    NamedSlot::Transfer(_) | NamedSlot::Home(_) => {},
                }
            });
            if let Some(err) = out_of_range {
                return Err(err);
            }

            // (1) every Transfer value use must be live.
            let mut unbound: Option<u16> = None;
            for_each_value_use(instr, |s| {
                if let NamedSlot::Transfer(j) = s
                    && !bound[j.0 as usize]
                    && unbound.is_none()
                {
                    unbound = Some(j.0);
                }
            });
            if let Some(j) = unbound {
                return Err(TransferVerifierError::TransferUseWithoutLiveDef {
                    block: b_idx,
                    instr: i,
                    transfer: j,
                });
            }

            // (2) at a call boundary, every bound Transfer slot must
            // be consumed by this call's args as args[j] = Transfer(j)
            // (arg positionality).
            if let Some((rets, args)) = call_boundary_rets_and_args(instr) {
                // Structural invariants (arg positionality, return Transfer prefix,
                // return monotonicity).
                check_call_structural_invariants(args, rets, |s| match s {
                    NamedSlot::Transfer(i) => Some(i.0),
                    NamedSlot::Home(_) => None,
                })
                .map_err(|e| TransferVerifierError::TransferCallStructural {
                    block: b_idx,
                    instr: i,
                    inner: Box::new(e),
                })?;
                // Orphan check: every bound slot must be consumed
                // by this call's args. A bound position not in args
                // signals a dead Transfer def from earlier in the block.
                for (j, &b) in bound.iter().enumerate() {
                    if b {
                        let consumed_here = j < args.len()
                            && args[j] == NamedSlot::Transfer(TransferPosition(j as u16));
                        if !consumed_here {
                            return Err(TransferVerifierError::TransferBoundNotConsumed {
                                block: b_idx,
                                instr: i,
                                transfer: j as u16,
                            });
                        }
                    }
                }
                // Clobber: a call reuses the entire callee region;
                // the ret defs below re-bind whatever positions
                // this call returns to.
                bound.fill(false);
            } else {
                // Non-call: value uses release their bindings (single-use).
                // A release that finds the binding already gone was released
                // by this same instruction — a duplicate use (never-bound
                // uses were rejected by the liveness check above).
                let mut duplicate: Option<u16> = None;
                for_each_value_use(instr, |slot| {
                    if let NamedSlot::Transfer(position) = slot {
                        if !bound[position.0 as usize] && duplicate.is_none() {
                            duplicate = Some(position.0);
                        }
                        bound[position.0 as usize] = false;
                    }
                });
                if let Some(transfer) = duplicate {
                    return Err(TransferVerifierError::DuplicateTransferUse {
                        block: b_idx,
                        instr: i,
                        transfer,
                    });
                }
            }
            // Defs bind: ret-Transfers for calls, Move/Copy dst (etc.)
            // for non-calls.
            for_each_def(instr, |s| {
                if let NamedSlot::Transfer(j) = s {
                    bound[j.0 as usize] = true;
                }
            });
        }

        // (3) no Transfer binding may survive past the end of a block.
        for (j, &b) in bound.iter().enumerate() {
            if b {
                return Err(TransferVerifierError::TransferBoundAtBlockEnd {
                    block: b_idx,
                    transfer: j as u16,
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        stackless_exec_ir::{
            BasicBlock, BinaryOp, FunctionIR, HomeIndex, ImmValue, Instr, InstrSeq, Label,
            NamedSlot,
        },
        validate::TranslationWitness,
    };
    use move_binary_format::{
        control_flow_graph::VMControlFlowGraph,
        file_format::{Bytecode, FunctionDefinitionIndex, FunctionHandleIndex, IdentifierIndex},
    };

    fn home(index: u16) -> NamedSlot {
        NamedSlot::Home(HomeIndex(index))
    }

    fn transfer(position: u16) -> NamedSlot {
        NamedSlot::Transfer(TransferPosition(position))
    }

    fn ld(dst: NamedSlot) -> Instr<NamedSlot> {
        Instr::LdImm {
            dst,
            imm: ImmValue::Bool(true),
        }
    }

    fn func_with(
        num_home_slots: u16,
        num_transfer_positions: u16,
        instrs: Vec<Instr<NamedSlot>>,
    ) -> FunctionIR {
        FunctionIR {
            name_idx: IdentifierIndex(0),
            handle_idx: FunctionHandleIndex(0),
            def_idx: FunctionDefinitionIndex(0),
            num_params: 0,
            num_locals: 0,
            num_home_slots,
            num_transfer_positions,
            blocks: vec![BasicBlock {
                label: Label(0),
                instrs: InstrSeq::for_tests(instrs),
            }],
            home_slot_types: Vec::new(),
            block_costs: Vec::new(),
            witness: TranslationWitness {
                label_to_offset: vec![0],
            },
        }
    }

    /// The pass reads only `func_ir`; the bytecode side is a placeholder.
    fn run(func_ir: &FunctionIR) -> Result<(), TransferVerifierError> {
        let code = [Bytecode::Ret];
        let ctx = FuncCtx {
            func_ir,
            code: &code,
            bytecode_cfg: VMControlFlowGraph::new(&code),
        };
        verify(&ctx)
    }

    #[test]
    fn in_range_slots_pass() {
        let func = func_with(1, 1, vec![
            ld(transfer(0)),
            Instr::Copy {
                dst: home(0),
                src: transfer(0),
            },
            Instr::Ret { srcs: Box::new([]) },
        ]);
        run(&func).expect("well-formed slots validate");
    }

    #[test]
    fn out_of_range_home_slot_fails() {
        let func = func_with(1, 0, vec![ld(home(5)), Instr::Ret { srcs: Box::new([]) }]);
        assert!(matches!(
            run(&func),
            Err(TransferVerifierError::HomeSlotOutOfRange { home: 5, .. })
        ));
    }

    #[test]
    fn out_of_range_transfer_position_fails() {
        let func = func_with(1, 0, vec![ld(transfer(0)), Instr::Ret {
            srcs: Box::new([]),
        }]);
        assert!(matches!(
            run(&func),
            Err(TransferVerifierError::TransferPositionOutOfRange { transfer: 0, .. })
        ));
    }

    /// Two uses of the same Transfer position in one instruction violate
    /// the single-use discipline even though both see a live binding.
    #[test]
    fn duplicate_transfer_use_fails() {
        let func = func_with(1, 1, vec![
            ld(transfer(0)),
            Instr::BinaryOp {
                dst: home(0),
                op: BinaryOp::Add,
                lhs: transfer(0),
                rhs: transfer(0),
            },
            Instr::Ret { srcs: Box::new([]) },
        ]);
        assert!(matches!(
            run(&func),
            Err(TransferVerifierError::DuplicateTransferUse { transfer: 0, .. })
        ));
    }
}
