-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Model

/-!
# Basic blocks of the model's control-flow graph

UNTRUSTED (checked by the kernel). Characterizes `Model.blockIds` and
`Model.buildBlocks`: the blocks partition the code into maximal runs that
start at a block id and contain no other block id.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model

/-! ## Block ids -/

theorem recordBlockIds_mem (code : Array Bytecode) (ids : List Nat) (pc x : Nat) :
    x ∈ recordBlockIds code ids pc ↔
      x ∈ ids ∨ ∃ b, code[pc]? = some b ∧
        (offset b = some x ∨ (isBranch b = true ∧ pc + 1 < code.size ∧ x = pc + 1)) := by
  unfold recordBlockIds
  cases hc : code[pc]? with
  | none => simp
  | some b =>
    cases ho : offset b <;> cases hb : isBranch b <;> by_cases hs : pc + 1 < code.size <;>
      simp [hs] <;> grind

theorem foldl_recordBlockIds_mem (code : Array Bytecode) (pcs : List Nat) (ids : List Nat)
    (x : Nat) :
    x ∈ pcs.foldl (recordBlockIds code) ids ↔
      x ∈ ids ∨ ∃ pc ∈ pcs, ∃ b, code[pc]? = some b ∧
        (offset b = some x ∨ (isBranch b = true ∧ pc + 1 < code.size ∧ x = pc + 1)) := by
  induction pcs generalizing ids with
  | nil => simp
  | cons pc pcs ih =>
    simp only [List.foldl_cons, ih, recordBlockIds_mem, List.mem_cons]
    constructor
    · rintro ((h | h) | ⟨q, hq, h⟩)
      · exact Or.inl h
      · exact Or.inr ⟨pc, Or.inl rfl, h⟩
      · exact Or.inr ⟨q, Or.inr hq, h⟩
    · rintro (h | ⟨q, (rfl | hq), h⟩)
      · exact Or.inl (Or.inl h)
      · exact Or.inl (Or.inr h)
      · exact Or.inr ⟨q, hq, h⟩

/-- The block ids: the entry, every jump target, and every instruction after a
branch. -/
theorem mem_blockIds (code : Array Bytecode) (x : Nat) :
    x ∈ blockIds code ↔
      x = 0 ∨ ∃ pc, pc < code.size ∧ ∃ b, code[pc]? = some b ∧
        (offset b = some x ∨ (isBranch b = true ∧ pc + 1 < code.size ∧ x = pc + 1)) := by
  simp only [blockIds, foldl_recordBlockIds_mem, List.mem_singleton, List.mem_range]

theorem zero_mem_blockIds (code : Array Bytecode) : 0 ∈ blockIds code :=
  (mem_blockIds code 0).2 (Or.inl rfl)

/-! ## Blocks -/

/-- What the blocks built so far guarantee: each lies within the code, starts
at a block id, ends at the end of a block, and contains no other block id. -/
structure BlockOk (code : Array Bytecode) (b : BasicBlock) : Prop where
  start_le_exit : b.start ≤ b.exit
  exit_lt : b.exit < code.size
  start_mem : b.start ∈ blockIds code
  exit_end : isEndOfBlock code (blockIds code) b.exit = true
  inner : ∀ p, b.start < p → p ≤ b.exit → p ∉ blockIds code
  successors : b.successors = getSuccessors b.exit code

/-- The fold invariant of `buildBlocks` after the first `k` instructions. -/
structure BuildInv (code : Array Bytecode) (k : Nat) (st : Nat × List BasicBlock) : Prop where
  entry_le : st.1 ≤ k
  entry_mem : st.1 < code.size → st.1 ∈ blockIds code
  blocks_ok : ∀ b ∈ st.2, BlockOk code b
  covered : ∀ pc, pc < st.1 → ∃ b ∈ st.2, b.start ≤ pc ∧ pc ≤ b.exit
  pending : ∀ pc, st.1 ≤ pc → pc < k → isEndOfBlock code (blockIds code) pc = false

theorem isEndOfBlock_iff (code : Array Bytecode) (pc : Nat) :
    isEndOfBlock code (blockIds code) pc = true ↔
      pc + 1 = code.size ∨ pc + 1 ∈ blockIds code := by
  simp [isEndOfBlock]

theorem buildInv_step (code : Array Bytecode) (k : Nat) (hk : k < code.size)
    (st : Nat × List BasicBlock) (inv : BuildInv code k st) :
    BuildInv code (k + 1)
      (if isEndOfBlock code (blockIds code) k = true then
        (k + 1, st.2 ++ [⟨st.1, k, getSuccessors k code⟩]) else st) := by
  by_cases hend : isEndOfBlock code (blockIds code) k = true
  · rw [if_pos hend]
    have hentry : st.1 ≤ k := inv.entry_le
    have hb : BlockOk code ⟨st.1, k, getSuccessors k code⟩ := by
      refine ⟨hentry, hk, inv.entry_mem (by omega), hend, ?_, rfl⟩
      intro p hp hpk hmem
      have hq : isEndOfBlock code (blockIds code) (p - 1) = false :=
        inv.pending (p - 1) (by simp at hp; omega) (by simp at hpk; omega)
      have : isEndOfBlock code (blockIds code) (p - 1) = true := by
        rw [isEndOfBlock_iff]; right
        simp at hp
        rwa [show p - 1 + 1 = p by omega]
      simp_all
    refine ⟨by simp, ?_, ?_, ?_, ?_⟩
    · intro hlt
      rcases (isEndOfBlock_iff code k).1 hend with h | h
      · simp at hlt; omega
      · simpa using h
    · intro b hb'
      rcases List.mem_append.1 hb' with h | h
      · exact inv.blocks_ok b h
      · simp at h; subst h; exact hb
    · intro pc hpc
      by_cases hlt : pc < st.1
      · obtain ⟨b, hb', h⟩ := inv.covered pc hlt
        exact ⟨b, List.mem_append_left _ hb', h⟩
      · refine ⟨⟨st.1, k, getSuccessors k code⟩, List.mem_append_right _ (by simp), ?_, ?_⟩
        · simp; omega
        · simp at hpc; simp; omega
    · intro pc h1 h2
      simp at h1 h2 ⊢; omega
  · rw [if_neg hend]
    refine ⟨by have := inv.entry_le; omega, inv.entry_mem, inv.blocks_ok, inv.covered, ?_⟩
    intro pc h1 h2
    by_cases hpk : pc = k
    · subst hpk; simpa using hend
    · exact inv.pending pc h1 (by omega)

/-- The fold step of `buildBlocks`. -/
def buildStep (code : Array Bytecode) (acc : Nat × List BasicBlock) (pc : Nat) :
    Nat × List BasicBlock :=
  if isEndOfBlock code (blockIds code) pc = true then
    (pc + 1, acc.2 ++ [⟨acc.1, pc, getSuccessors pc code⟩]) else acc

theorem buildBlocks_eq (code : Array Bytecode) :
    buildBlocks code = ((List.range code.size).foldl (buildStep code) (0, [])).2 := rfl

theorem buildInv_foldl (code : Array Bytecode) (k : Nat) (hk : k ≤ code.size) :
    BuildInv code k ((List.range k).foldl (buildStep code) (0, [])) := by
  induction k with
  | zero =>
    exact ⟨by simp, fun _ => zero_mem_blockIds code, by simp, by simp, by simp⟩
  | succ k ih =>
    rw [List.range_succ, List.foldl_append, List.foldl_cons, List.foldl_nil]
    exact buildInv_step code k (by omega) _ (ih (by omega))

/-- The blocks partition the code: every block is well formed, and every
instruction lies in some block. -/
theorem buildBlocks_spec (code : Array Bytecode) :
    (∀ b ∈ buildBlocks code, BlockOk code b) ∧
    (∀ pc, pc < code.size → ∃ b ∈ buildBlocks code, b.start ≤ pc ∧ pc ≤ b.exit) := by
  have inv := buildInv_foldl code code.size (Nat.le_refl _)
  rw [buildBlocks_eq]
  refine ⟨inv.blocks_ok, ?_⟩
  intro pc hpc
  -- The final entry is the code size, since the last instruction ends a block.
  have hentry : ((List.range code.size).foldl (buildStep code) (0, [])).1 = code.size := by
    have hle := inv.entry_le
    cases Nat.lt_or_ge ((List.range code.size).foldl (buildStep code) (0, [])).1 code.size with
    | inl hlt =>
      have hlast := inv.pending (code.size - 1) (by omega) (by omega)
      have : isEndOfBlock code (blockIds code) (code.size - 1) = true :=
        (isEndOfBlock_iff code _).2 (Or.inl (by omega))
      simp_all
    | inr hge => omega
  exact inv.covered pc (by rw [hentry]; exact hpc)

end MoveBytecode.Proofs
