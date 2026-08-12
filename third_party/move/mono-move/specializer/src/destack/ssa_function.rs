// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Intermediate SSA representation and pre-slot-allocation fusion passes.
//!
//! SSA is intra-block and applies only to value IDs, not to params or locals
//! (which are mutable across blocks). Because the operand stack is empty at
//! block boundaries, no phi nodes are needed — each value ID is defined exactly
//! once within its block and never crosses a block boundary.

use crate::stackless_exec_ir::{
    instr_utils::{for_each_value_use, is_commutative},
    BasicBlock, BinaryOp, FieldPath, Instr, SsaSlot,
};
use mono_move_core::types::InternedType;
use shared_dsa::UnorderedMap;

/// Intermediate SSA representation of a single function, before slot allocation.
pub(crate) struct SSAFunction {
    /// Basic blocks in SSA form.
    pub blocks: Vec<BasicBlock<SsaSlot>>,
    /// Type of each value ID, indexed directly by the value ID number.
    pub value_id_types: Vec<InternedType>,
    /// Types of all locals (params ++ declared locals).
    pub local_types: Vec<InternedType>,
}

impl SSAFunction {
    /// Run all instruction fusion passes; they precede slot allocation, so
    /// fused-away value IDs never receive frame slots.
    pub(crate) fn with_fusion_passes(mut self) -> Self {
        // TODO(perf): right now, we have each different fusion operation to be a separate pass.
        // This is easier to reason about, but we could make it more efficient by
        // combining the passes.
        for block in &mut self.blocks {
            // Collapse depth-N inline-field chains first, on the raw borrow chain.
            fuse_field_chains(&mut block.instrs);
            fuse_pairs(&mut block.instrs, try_fuse_field_access);
            // Consumes ReadField/WriteField produced above, maintain this
            // ordering between fusion passes.
            fuse_pairs(&mut block.instrs, try_fuse_local_field_access);
            fuse_pairs(&mut block.instrs, try_fuse_immediate_binop);
            // Must run after try_fuse_immediate_binop so that BinaryOpImm is
            // available for the BrCmpImm variant.
            fuse_pairs(&mut block.instrs, try_fuse_compare_branch);
        }
        self
    }
}

/// In-place compaction that fuses consecutive instruction pairs.
///
/// For each position, calls `try_fuse(&instrs[r], &instrs[r+1])`. If it returns
/// `Some(fused)`, the pair is replaced by the single fused instruction. Otherwise
/// the instruction is kept as-is. Uses a write-cursor so no allocation is needed.
fn fuse_pairs(
    instrs: &mut Vec<Instr<SsaSlot>>,
    try_fuse: fn(&Instr<SsaSlot>, &Instr<SsaSlot>) -> Option<Instr<SsaSlot>>,
) {
    let mut write = 0;
    let mut read = 0;
    while read < instrs.len() {
        let fused = instrs
            .get(read + 1)
            .and_then(|next| try_fuse(&instrs[read], next));

        match fused {
            Some(fused_instr) => {
                instrs[write] = fused_instr;
                read += 2;
            },
            None => {
                if write != read {
                    instrs.swap(write, read);
                }
                read += 1;
            },
        }
        write += 1;
    }
    instrs.truncate(write);
}

/// Try to fuse a borrow+deref pair into a combined field access instruction.
fn try_fuse_field_access(
    first: &Instr<SsaSlot>,
    second: &Instr<SsaSlot>,
) -> Option<Instr<SsaSlot>> {
    match (first, second) {
        (
            Instr::ImmBorrowField {
                dst: ref_r,
                owner_ty,
                field,
                src,
            },
            Instr::ReadRef { dst, src: read_src },
        ) if *ref_r == *read_src => Some(Instr::ReadField {
            dst: *dst,
            owner_ty: *owner_ty,
            field: *field,
            src: *src,
        }),
        (
            Instr::MutBorrowField {
                dst: ref_r,
                owner_ty,
                field,
                src: dst_ref,
            },
            Instr::WriteRef {
                dst_ref: write_ref,
                val,
            },
        ) if *ref_r == *write_ref => Some(Instr::WriteField {
            dst_ref: *dst_ref,
            owner_ty: *owner_ty,
            field: *field,
            val: *val,
        }),
        (
            Instr::ImmBorrowVariantField {
                dst: ref_r,
                owner_ty,
                field,
                src,
            },
            Instr::ReadRef { dst, src: read_src },
        ) if *ref_r == *read_src => Some(Instr::ReadVariantField {
            dst: *dst,
            owner_ty: *owner_ty,
            field: *field,
            src: *src,
        }),
        (
            Instr::MutBorrowVariantField {
                dst: ref_r,
                owner_ty,
                field,
                src: dst_ref,
            },
            Instr::WriteRef {
                dst_ref: write_ref,
                val,
            },
        ) if *ref_r == *write_ref => Some(Instr::WriteVariantField {
            dst_ref: *dst_ref,
            owner_ty: *owner_ty,
            field: *field,
            val: *val,
        }),
        _ => None,
    }
}

/// Try to fuse a `borrow_loc` followed by a field op on its result into a
/// single local-field op, eliding the intermediate fat pointer.
fn try_fuse_local_field_access(
    first: &Instr<SsaSlot>,
    second: &Instr<SsaSlot>,
) -> Option<Instr<SsaSlot>> {
    match (first, second) {
        (
            Instr::ImmBorrowLoc { dst: ref_r, local },
            Instr::ImmBorrowField {
                dst,
                owner_ty,
                field,
                src,
            },
        ) if *ref_r == *src => Some(Instr::ImmBorrowLocField {
            dst: *dst,
            owner_ty: *owner_ty,
            field: *field,
            local: *local,
        }),
        (
            Instr::MutBorrowLoc { dst: ref_r, local },
            Instr::MutBorrowField {
                dst,
                owner_ty,
                field,
                src,
            },
        ) if *ref_r == *src => Some(Instr::MutBorrowLocField {
            dst: *dst,
            owner_ty: *owner_ty,
            field: *field,
            local: *local,
        }),
        (
            Instr::ImmBorrowLoc { dst: ref_r, local },
            Instr::ReadField {
                dst,
                owner_ty,
                field,
                src,
            },
        ) if *ref_r == *src => Some(Instr::ReadLocField {
            dst: *dst,
            owner_ty: *owner_ty,
            field: *field,
            local: *local,
        }),
        (
            Instr::MutBorrowLoc { dst: ref_r, local },
            Instr::WriteField {
                dst_ref,
                owner_ty,
                field,
                val,
            },
        ) if *ref_r == *dst_ref => Some(Instr::WriteLocField {
            local: *local,
            owner_ty: *owner_ty,
            field: *field,
            val: *val,
        }),
        _ => None,
    }
}

/// Try to fuse a comparison + conditional branch pair into a single `BrCmp`/`BrCmpImm`.
///
/// Handles both `BrTrue` (keeps the comparison operator) and `BrFalse` (negates it).
fn try_fuse_compare_branch(
    first: &Instr<SsaSlot>,
    second: &Instr<SsaSlot>,
) -> Option<Instr<SsaSlot>> {
    match (first, second) {
        (
            Instr::BinaryOp {
                dst,
                op: BinaryOp::Cmp(cmp),
                lhs,
                rhs,
            },
            Instr::BrTrue { target, cond },
        ) if *dst == *cond => Some(Instr::BrCmp {
            target: *target,
            op: *cmp,
            lhs: *lhs,
            rhs: *rhs,
        }),
        (
            Instr::BinaryOp {
                dst,
                op: BinaryOp::Cmp(cmp),
                lhs,
                rhs,
            },
            Instr::BrFalse { target, cond },
        ) if *dst == *cond => Some(Instr::BrCmp {
            target: *target,
            op: cmp.negate(),
            lhs: *lhs,
            rhs: *rhs,
        }),
        (
            Instr::BinaryOpImm {
                dst,
                op: BinaryOp::Cmp(cmp),
                lhs,
                imm,
            },
            Instr::BrTrue { target, cond },
        ) if *dst == *cond => Some(Instr::BrCmpImm {
            target: *target,
            op: *cmp,
            lhs: *lhs,
            imm: imm.clone(),
        }),
        (
            Instr::BinaryOpImm {
                dst,
                op: BinaryOp::Cmp(cmp),
                lhs,
                imm,
            },
            Instr::BrFalse { target, cond },
        ) if *dst == *cond => Some(Instr::BrCmpImm {
            target: *target,
            op: cmp.negate(),
            lhs: *lhs,
            imm: imm.clone(),
        }),
        _ => None,
    }
}

/// Where a fused field chain is rooted.
enum ChainRoot {
    /// A by-value inline-struct local (the `*BorrowLoc` is absorbed).
    Local(SsaSlot),
    /// A reference (any non-chain ref value).
    Ref(SsaSlot),
}

/// How a chain ends.
enum ChainTerminal {
    /// `ReadRef` of the deepest reference — a by-value field read. `dst` is
    /// the `ReadRef`'s destination, receiving the field's bytes.
    Read { dst: SsaSlot },
    /// `WriteRef` through the deepest reference — a field write. `val` is the
    /// `WriteRef`'s value operand, whose bytes are stored into the field.
    Write { val: SsaSlot },
    /// The deepest reference escapes (returned, passed, frozen, ...) — a
    /// borrow. `dst` is that reference itself, which the fused instruction
    /// takes over defining; its consumers stay unchanged.
    Borrow { dst: SsaSlot },
}

/// A fused chain and how it rewrites the instruction stream.
struct FusedChain {
    /// The fused instruction.
    instr: Instr<SsaSlot>,
    /// The single position that survives the rewrite: `instr` overwrites the
    /// original instruction here (the terminal read/write, or the last borrow
    /// for a borrow chain); every other absorbed position is deleted.
    place_at: usize,
    /// End (exclusive) of the contiguous borrow run starting at the scan
    /// position; the run's positions other than `place_at` are deleted, and
    /// scanning resumes here.
    borrow_end: usize,
}

/// Minimum number of field borrows a fused chain collapses; shorter runs are
/// left to the pairwise passes. Shared by `fuse_field_chains`'s pre-scan gate
/// and `try_build_chain`'s depth check.
const MIN_CHAIN_DEPTH: usize = 2;

/// Pre-slot-allocation pass: collapse a run of inline-struct field selections
/// whose intermediate references are dead after the chain into one fused chain
/// instruction (the erased references never receive frame slots). Runs on the
/// raw borrow chain before the pairwise field fusions.
///
/// A run is fused only when every erased intermediate reference is single-use
/// (so removing its def is sound). The borrow links are contiguous; the
/// terminal read/write may sit a few instructions later (e.g. after a write's
/// right-hand side is evaluated). Sinking the borrow chain's address compute to
/// that terminal is sound because a reference is an address compute
/// (`base + offset`), not a value snapshot, and Move forbids modifying the root
/// while it stays borrowed. Depth-1 runs are left to the pairwise passes.
fn fuse_field_chains(instrs: &mut Vec<Instr<SsaSlot>>) {
    // Every fusable chain contains at least `MIN_CHAIN_DEPTH` struct field
    // borrows; skip the analysis allocations for blocks that cannot hold one
    // (the common case).
    let field_borrows = instrs
        .iter()
        .filter(|instr| {
            matches!(
                instr,
                Instr::ImmBorrowField { .. } | Instr::MutBorrowField { .. }
            )
        })
        .take(MIN_CHAIN_DEPTH)
        .count();
    if field_borrows < MIN_CHAIN_DEPTH {
        return;
    }

    let use_info = value_use_info(instrs);
    let len = instrs.len();
    // Rewrite plan, allocated lazily on the first fused chain and applied by
    // the compaction loop below: `placed[pos]` replaces the instruction at
    // `pos` with the fused one, `removed[pos]` deletes it. `placed` stays
    // empty when nothing fuses.
    let mut placed: Vec<Option<Instr<SsaSlot>>> = Vec::new();
    let mut removed: Vec<bool> = Vec::new();

    let mut start = 0;
    while start < len {
        // Skip a position holding an earlier chain's sunk terminal.
        if !placed.is_empty() && placed[start].is_some() {
            start += 1;
            continue;
        }
        match try_build_chain(instrs, start, &use_info) {
            Some(chain) => {
                if placed.is_empty() {
                    placed = vec![None; len];
                    removed = vec![false; len];
                }
                // The absorbed positions are always free: the cursor never
                // re-enters a fused run (it resumes at `borrow_end`), and an
                // earlier chain's sunk terminal cannot lie inside this run (a
                // run is contiguous borrows; a terminal is a read/write).
                debug_assert!((start..chain.borrow_end).all(|idx| !removed[idx]));
                debug_assert!(!removed[chain.place_at] && placed[chain.place_at].is_none());
                removed[start..chain.borrow_end].fill(true);
                // No-op for a sunk terminal (`place_at >= borrow_end`).
                removed[chain.place_at] = false;
                placed[chain.place_at] = Some(chain.instr);
                start = chain.borrow_end;
            },
            None => start += 1,
        }
    }
    if placed.is_empty() {
        return;
    }

    // In-place write-cursor compaction (same shape as `fuse_pairs`): fused
    // instructions overwrite their `place_at` position, removed ones drop out.
    let mut write = 0;
    for read in 0..len {
        if removed[read] {
            continue;
        }
        if let Some(fused) = placed[read].take() {
            instrs[write] = fused;
        } else if write != read {
            instrs.swap(write, read);
        }
        write += 1;
    }
    instrs.truncate(write);
}

/// For each `ValueId`, the `(use count, last consumer position)` pair. A reference
/// `ValueId` is linkable into a chain only when its use count is exactly 1, in which
/// case the recorded position is that sole consumer's.
fn value_use_info(instrs: &[Instr<SsaSlot>]) -> UnorderedMap<SsaSlot, (u32, usize)> {
    let mut info: UnorderedMap<SsaSlot, (u32, usize)> = UnorderedMap::new();
    for (pos, instr) in instrs.iter().enumerate() {
        for_each_value_use(instr, |slot| {
            if slot.is_value_id() {
                let entry = info.entry(slot).or_insert((0, pos));
                entry.0 += 1;
                entry.1 = pos;
            }
        });
    }
    info
}

/// Try to build a fused field chain rooted at `instrs[start]`.
fn try_build_chain(
    instrs: &[Instr<SsaSlot>],
    start: usize,
    use_info: &UnorderedMap<SsaSlot, (u32, usize)>,
) -> Option<FusedChain> {
    let linkable =
        |slot: SsaSlot| slot.is_value_id() && matches!(use_info.get(&slot), Some(&(1, _)));

    // Identify the head: the root, the reference the next struct field-borrow
    // must consume, the next index, and the path so far. The borrow kind (`Imm`
    // vs `Mut`) doesn't change how the head links, so derive it once and merge
    // each Imm/Mut head pair.
    let is_mut = matches!(
        &instrs[start],
        Instr::MutBorrowLoc { .. } | Instr::MutBorrowField { .. }
    );
    let (root, mut cur_ref, mut idx, mut path) = match &instrs[start] {
        Instr::ImmBorrowLoc {
            dst: produced,
            local,
        }
        | Instr::MutBorrowLoc {
            dst: produced,
            local,
        } => (ChainRoot::Local(*local), *produced, start + 1, Vec::new()),
        Instr::ImmBorrowField {
            dst: produced,
            owner_ty,
            field,
            src,
        }
        | Instr::MutBorrowField {
            dst: produced,
            owner_ty,
            field,
            src,
        } => (ChainRoot::Ref(*src), *produced, start + 1, vec![(
            *owner_ty, *field,
        )]),
        _ => return None,
    };

    // Extend through contiguous, single-use struct field-borrows of the matching
    // borrow kind. (Field borrows are emitted back-to-back; only the terminal
    // may be separated, by a write's right-hand-side computation.)
    while linkable(cur_ref) {
        match instrs.get(idx) {
            Some(Instr::ImmBorrowField {
                dst: produced,
                owner_ty,
                field,
                src,
            }) if !is_mut && *src == cur_ref => {
                path.push((*owner_ty, *field));
                cur_ref = *produced;
                idx += 1;
            },
            Some(Instr::MutBorrowField {
                dst: produced,
                owner_ty,
                field,
                src,
            }) if is_mut && *src == cur_ref => {
                path.push((*owner_ty, *field));
                cur_ref = *produced;
                idx += 1;
            },
            _ => break,
        }
    }
    // The absorbed borrows occupy `[start, borrow_end)`.
    let borrow_end = idx;

    // Shorter runs are left to the pairwise passes (see `MIN_CHAIN_DEPTH`,
    // which the pre-scan gate shares).
    if path.len() < MIN_CHAIN_DEPTH {
        return None;
    }

    // Classify the terminal by the deepest reference's sole consumer. It is
    // single-use exactly when it feeds a `ReadRef`/`WriteRef`; otherwise it
    // escapes (returned, passed, frozen, used more than once) and ends a borrow.
    // Field borrows are pure address computes (no abort), so the terminal may
    // sink past intervening instructions freely.
    let sole_use_pos = if cur_ref.is_value_id() {
        use_info
            .get(&cur_ref)
            .and_then(|&(count, pos)| (count == 1).then_some(pos))
    } else {
        None
    };
    let terminal_consumer = sole_use_pos.and_then(|term_idx| match &instrs[term_idx] {
        Instr::ReadRef { dst, src } if *src == cur_ref => {
            Some((ChainTerminal::Read { dst: *dst }, term_idx))
        },
        Instr::WriteRef { dst_ref, val } if *dst_ref == cur_ref => {
            Some((ChainTerminal::Write { val: *val }, term_idx))
        },
        _ => None,
    });

    // The borrows are absorbed; the terminal (when separate) is replaced in
    // place at `place_at` by the fused instruction.
    let (terminal, place_at) = match terminal_consumer {
        Some((terminal, term_idx)) => (terminal, term_idx),
        // No read/write consumer: the deepest reference is the chain's result,
        // produced at the last borrow's position.
        None => (ChainTerminal::Borrow { dst: cur_ref }, borrow_end - 1),
    };

    Some(FusedChain {
        instr: build_chain_instr(root, is_mut, path.into(), terminal),
        place_at,
        borrow_end,
    })
}

/// Construct the fused instruction for a classified chain.
fn build_chain_instr(
    root: ChainRoot,
    is_mut: bool,
    path: FieldPath,
    terminal: ChainTerminal,
) -> Instr<SsaSlot> {
    match root {
        ChainRoot::Local(local) => match terminal {
            ChainTerminal::Read { dst } => Instr::ReadLocFieldChain { dst, path, local },
            ChainTerminal::Write { val } => Instr::WriteLocFieldChain { local, path, val },
            ChainTerminal::Borrow { dst } if is_mut => {
                Instr::MutBorrowLocFieldChain { dst, path, local }
            },
            ChainTerminal::Borrow { dst } => Instr::ImmBorrowLocFieldChain { dst, path, local },
        },
        ChainRoot::Ref(src) => match terminal {
            ChainTerminal::Read { dst } => Instr::ReadFieldChain { dst, path, src },
            ChainTerminal::Write { val } => Instr::WriteFieldChain {
                dst_ref: src,
                path,
                val,
            },
            ChainTerminal::Borrow { dst } if is_mut => {
                Instr::MutBorrowFieldChain { dst, path, src }
            },
            ChainTerminal::Borrow { dst } => Instr::ImmBorrowFieldChain { dst, path, src },
        },
    }
}

/// Try to fuse a `LdImm` + `BinaryOp` pair into a `BinaryOpImm` instruction.
fn try_fuse_immediate_binop(
    first: &Instr<SsaSlot>,
    second: &Instr<SsaSlot>,
) -> Option<Instr<SsaSlot>> {
    match (first, second) {
        (Instr::LdImm { dst: tmp, imm }, Instr::BinaryOp { dst, op, lhs, rhs }) if *tmp == *rhs => {
            Some(Instr::BinaryOpImm {
                dst: *dst,
                op: *op,
                lhs: *lhs,
                imm: imm.clone(),
            })
        },
        (Instr::LdImm { dst: tmp, imm }, Instr::BinaryOp { dst, op, lhs, rhs })
            if *tmp == *lhs && is_commutative(op) =>
        {
            Some(Instr::BinaryOpImm {
                dst: *dst,
                op: *op,
                lhs: *rhs,
                imm: imm.clone(),
            })
        },
        _ => None,
    }
}
