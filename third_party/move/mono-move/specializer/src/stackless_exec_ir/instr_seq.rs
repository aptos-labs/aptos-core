// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Instruction sequence with per-instruction bytecode provenance.

use super::Instr;
use mono_move_core::{
    BytecodeOffset, ExecutionErrorKind, IntoExecutionError, VMInternalError, VMResult,
};
use std::ops::Deref;
use thiserror::Error;

/// An instruction sequence with per-instruction bytecode provenance. Two
/// parallel arrays: instructions and their originating bytecode offsets.
/// Transformations are alignment preserving, so `origins.len() == instrs.len()`
/// holds by construction.
///
/// When an instruction aborts, its origin is the offset reported for the abort.
/// A fused instruction keeps its constituents' semantics but only one of their
/// origins, so the fusion methods refuse to discard the origin of a constituent
/// that can abort. Deleting an instruction discards its origin too, but that is
/// safe: an instruction that no longer exists cannot abort. A synthesized
/// instruction (one with no bytecode ancestor) must not be abortable — its
/// placeholder origin, the next real instruction's offset, would misattribute
/// an abort.
pub struct InstrSeq<SlotForm> {
    instrs: Vec<Instr<SlotForm>>,
    origins: Vec<BytecodeOffset>,
}

impl<SlotForm> Default for InstrSeq<SlotForm> {
    fn default() -> Self {
        Self {
            instrs: Vec::new(),
            origins: Vec::new(),
        }
    }
}

impl<SlotForm> Deref for InstrSeq<SlotForm> {
    type Target = [Instr<SlotForm>];

    fn deref(&self) -> &[Instr<SlotForm>] {
        &self.instrs
    }
}

impl<'a, SlotForm> IntoIterator for &'a InstrSeq<SlotForm> {
    type IntoIter = std::slice::Iter<'a, Instr<SlotForm>>;
    type Item = &'a Instr<SlotForm>;

    fn into_iter(self) -> Self::IntoIter {
        self.instrs.iter()
    }
}

impl<SlotForm> InstrSeq<SlotForm> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Test-only construction from bare instructions, all with origin 0.
    #[cfg(test)]
    pub(crate) fn for_tests(instrs: Vec<Instr<SlotForm>>) -> Self {
        let origins = vec![0; instrs.len()];
        Self { instrs, origins }
    }

    /// Append `instr`, recording the bytecode offset it originates from.
    pub fn push(&mut self, instr: Instr<SlotForm>, origin: BytecodeOffset) {
        self.instrs.push(instr);
        self.origins.push(origin);
    }

    /// Origin of each instruction, parallel to the instruction slice.
    pub fn origins(&self) -> &[BytecodeOffset] {
        &self.origins
    }

    /// In-place instruction rewrites. Each position keeps its origin, which
    /// obligates the caller on two points that `&mut` access cannot enforce:
    ///
    /// - Rewrite in place only. Moving an instruction's payload to another
    ///   position pairs it with that position's origin.
    /// - Do not turn a non-abortable instruction into an abortable one: the
    ///   kept origin may be fusion-inherited from a different bytecode. The
    ///   reverse direction is fine.
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, Instr<SlotForm>> {
        self.instrs.iter_mut()
    }

    /// Iterate instructions paired with their origins.
    pub fn iter_with_origins(
        &self,
    ) -> impl Iterator<Item = (&Instr<SlotForm>, BytecodeOffset)> + '_ {
        self.instrs.iter().zip(self.origins.iter().copied())
    }

    /// Keep only instructions satisfying `pred`, dropping origins in
    /// lockstep. Sound regardless of abortability: a deleted instruction
    /// cannot execute.
    pub fn retain(&mut self, mut pred: impl FnMut(&Instr<SlotForm>) -> bool) {
        self.retain_indexed(|_, instr| pred(instr));
    }

    /// [`Self::retain`] with the instruction's pre-retain index, for
    /// callers that plan deletions by position.
    pub fn retain_indexed(&mut self, mut pred: impl FnMut(usize, &Instr<SlotForm>) -> bool) {
        let mut write = 0;
        for read in 0..self.instrs.len() {
            if pred(read, &self.instrs[read]) {
                if write != read {
                    self.instrs.swap(write, read);
                    self.origins.swap(write, read);
                }
                write += 1;
            }
        }
        self.truncate_both(write);
    }

    /// Convert every instruction with a fallible mapping, carrying origins
    /// verbatim.
    pub fn try_map<NewSlotForm, MapError>(
        self,
        mut map_instr: impl FnMut(usize, Instr<SlotForm>) -> Result<Instr<NewSlotForm>, MapError>,
    ) -> Result<InstrSeq<NewSlotForm>, MapError> {
        let instrs = self
            .instrs
            .into_iter()
            .enumerate()
            .map(|(index, instr)| map_instr(index, instr))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(InstrSeq {
            instrs,
            origins: self.origins,
        })
    }

    /// In-place compaction that fuses consecutive instruction pairs.
    ///
    /// For each position, calls `try_fuse(&instrs[r], &instrs[r+1])`. If it
    /// returns `Some(fused)`, the pair is replaced by the single fused
    /// instruction. Otherwise the instruction is kept as-is. Uses a
    /// write-cursor so no allocation is needed.
    ///
    /// A fused instruction keeps its abortable constituent's origin (the
    /// first constituent's when neither can abort). Fusing two abortable
    /// instructions is an error: no single origin could attribute both.
    pub(crate) fn fuse_pairs(
        &mut self,
        try_fuse: fn(&Instr<SlotForm>, &Instr<SlotForm>) -> Option<Instr<SlotForm>>,
    ) -> VMResult<()> {
        let mut write = 0;
        let mut read = 0;
        while read < self.instrs.len() {
            let fused = self
                .instrs
                .get(read + 1)
                .and_then(|next| try_fuse(&self.instrs[read], next));

            match fused {
                Some(fused_instr) => {
                    let origin = match (
                        self.instrs[read].can_abort(),
                        self.instrs[read + 1].can_abort(),
                    ) {
                        (true, true) => {
                            return Err(VMInternalError::new(
                                ProvenanceError::TwoAbortableConstituents {
                                    first: self.instrs[read].opcode_name(),
                                    second: self.instrs[read + 1].opcode_name(),
                                },
                            ))
                        },
                        (false, true) => self.origins[read + 1],
                        (true, false) | (false, false) => self.origins[read],
                    };
                    self.instrs[write] = fused_instr;
                    self.origins[write] = origin;
                    read += 2;
                },
                None => {
                    if write != read {
                        self.instrs.swap(write, read);
                        self.origins.swap(write, read);
                    }
                    read += 1;
                },
            }
            write += 1;
        }
        self.truncate_both(write);
        Ok(())
    }

    /// Apply a rewrite plan: two vectors parallel to the sequence, where
    /// `removed[i]` deletes the instruction at position `i` and `placed[i]`
    /// replaces it with a fused instruction that keeps the position's
    /// origin.
    ///
    /// A removed instruction's origin is dropped, so it must be
    /// non-abortable; a position may not be both removed and placed. Both
    /// are errors.
    ///
    /// A placed instruction must subsume the semantics of the one it
    /// overwrites — the kept origin then attributes any abort of the
    /// replaced instruction correctly, which is why placing over an
    /// abortable instruction is legal.
    pub(crate) fn apply_rewrite_plan(
        &mut self,
        mut placed: Vec<Option<Instr<SlotForm>>>,
        removed: Vec<bool>,
    ) -> VMResult<()> {
        if placed.len() != self.instrs.len() || removed.len() != self.instrs.len() {
            return Err(VMInternalError::new(ProvenanceError::PlanShapeMismatch {
                detail: "plan length differs from instruction count",
            }));
        }
        for (index, &is_removed) in removed.iter().enumerate() {
            if !is_removed {
                continue;
            }
            if placed[index].is_some() {
                return Err(VMInternalError::new(ProvenanceError::PlanShapeMismatch {
                    detail: "position both removed and placed",
                }));
            }
            if self.instrs[index].can_abort() {
                return Err(VMInternalError::new(
                    ProvenanceError::DropsAbortableInstruction {
                        opcode: self.instrs[index].opcode_name(),
                    },
                ));
            }
        }

        // Placement is length-preserving (each placed position keeps its own
        // origin); the removals then compact both arrays.
        for (index, slot) in placed.iter_mut().enumerate() {
            if let Some(fused) = slot.take() {
                self.instrs[index] = fused;
            }
        }
        self.retain_indexed(|index, _| !removed[index]);
        Ok(())
    }

    fn truncate_both(&mut self, len: usize) {
        self.instrs.truncate(len);
        self.origins.truncate(len);
    }
}

#[derive(Debug, Error)]
enum ProvenanceError {
    #[error("fusing two abortable instructions ({first} + {second}); a fused op keeps one origin")]
    TwoAbortableConstituents {
        first: &'static str,
        second: &'static str,
    },

    #[error("rewrite plan drops the origin of abortable instruction {opcode}")]
    DropsAbortableInstruction { opcode: &'static str },

    #[error("rewrite plan shape mismatch: {detail}")]
    PlanShapeMismatch { detail: &'static str },
}

impl IntoExecutionError for ProvenanceError {
    fn kind(&self) -> ExecutionErrorKind {
        use ProvenanceError::*;
        match self {
            TwoAbortableConstituents { .. }
            | DropsAbortableInstruction { .. }
            | PlanShapeMismatch { .. } => ExecutionErrorKind::InvariantViolation,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stackless_exec_ir::{BinaryOp, ImmValue, Instr, SsaSlot};

    fn value(id: u16) -> SsaSlot {
        SsaSlot::ValueId(id)
    }

    /// Non-abortable.
    fn ld(dst: u16) -> Instr<SsaSlot> {
        Instr::LdImm {
            dst: value(dst),
            imm: ImmValue::U8(1),
        }
    }

    /// Abortable (arithmetic overflow).
    fn add(dst: u16, lhs: u16, rhs: u16) -> Instr<SsaSlot> {
        Instr::BinaryOp {
            dst: value(dst),
            op: BinaryOp::Add,
            lhs: value(lhs),
            rhs: value(rhs),
        }
    }

    fn opcodes(seq: &InstrSeq<SsaSlot>) -> Vec<&'static str> {
        seq.iter().map(Instr::opcode_name).collect()
    }

    #[test]
    fn retain_drops_origins_in_lockstep() {
        let mut seq = InstrSeq::new();
        seq.push(ld(0), 0);
        seq.push(add(1, 0, 0), 1);
        seq.push(ld(2), 2);
        seq.retain(|instr| !matches!(instr, Instr::LdImm { .. }));
        assert_eq!(opcodes(&seq), ["BinaryOp"]);
        assert_eq!(seq.origins(), [1]);
    }

    #[test]
    fn retain_indexed_sees_pre_retain_positions() {
        let mut seq = InstrSeq::new();
        seq.push(ld(0), 4);
        seq.push(ld(1), 5);
        seq.push(ld(2), 6);
        seq.retain_indexed(|index, _| index != 1);
        assert_eq!(seq.origins(), [4, 6]);
    }

    #[test]
    fn try_map_preserves_origins() {
        let mut seq = InstrSeq::new();
        seq.push(ld(0), 7);
        seq.push(ld(1), 9);
        let mapped = seq
            .try_map(|_, instr| Ok::<_, ()>(instr))
            .expect("mapping is infallible");
        assert_eq!(mapped.origins(), [7, 9]);
    }

    /// Fuses any pair into (a copy of) the second instruction — enough to
    /// drive `fuse_pairs`' origin selection from the constituents alone.
    fn fuse_to_second(_first: &Instr<SsaSlot>, second: &Instr<SsaSlot>) -> Option<Instr<SsaSlot>> {
        Some(second.clone())
    }

    #[test]
    fn fuse_pairs_takes_first_origin_when_neither_aborts() {
        let mut seq = InstrSeq::new();
        seq.push(ld(0), 3);
        seq.push(ld(1), 5);
        seq.fuse_pairs(fuse_to_second).expect("fusable");
        assert_eq!(seq.origins(), [3]);
    }

    #[test]
    fn fuse_pairs_takes_the_abortable_constituents_origin() {
        // Abortable second: LdImm + BinaryOp keeps the arithmetic origin.
        let mut seq = InstrSeq::new();
        seq.push(ld(0), 3);
        seq.push(add(1, 0, 0), 8);
        seq.fuse_pairs(fuse_to_second).expect("fusable");
        assert_eq!(seq.origins(), [8]);

        // Abortable first: the borrow-side origin survives.
        let mut seq = InstrSeq::new();
        seq.push(add(0, 0, 0), 2);
        seq.push(ld(1), 6);
        seq.fuse_pairs(fuse_to_second).expect("fusable");
        assert_eq!(seq.origins(), [2]);
    }

    #[test]
    fn fuse_pairs_rejects_two_abortable_constituents() {
        let mut seq = InstrSeq::new();
        seq.push(add(0, 0, 0), 1);
        seq.push(add(1, 0, 0), 2);
        assert!(seq.fuse_pairs(fuse_to_second).is_err());
    }

    #[test]
    fn apply_rewrite_plan_keeps_the_replaced_positions_origin() {
        let mut seq = InstrSeq::new();
        seq.push(ld(0), 0);
        seq.push(ld(1), 1);
        seq.push(ld(2), 2);
        // Remove position 0, replace position 2.
        let placed = vec![None, None, Some(add(3, 1, 2))];
        let removed = vec![true, false, false];
        seq.apply_rewrite_plan(placed, removed).expect("valid plan");
        assert_eq!(opcodes(&seq), ["LdImm", "BinaryOp"]);
        assert_eq!(seq.origins(), [1, 2]);
    }

    #[test]
    fn apply_rewrite_plan_rejects_dropping_an_abortable_instruction() {
        let mut seq = InstrSeq::new();
        seq.push(add(0, 0, 0), 0);
        seq.push(ld(1), 1);
        let placed = vec![None, Some(ld(2))];
        let removed = vec![true, false];
        assert!(seq.apply_rewrite_plan(placed, removed).is_err());
    }

    #[test]
    fn apply_rewrite_plan_rejects_malformed_plans() {
        let mut seq = InstrSeq::new();
        seq.push(ld(0), 0);
        // Length mismatch.
        assert!(seq.apply_rewrite_plan(vec![], vec![]).is_err());
        // Removed and placed at once.
        assert!(seq
            .apply_rewrite_plan(vec![Some(ld(1))], vec![true])
            .is_err());
    }
}
