-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.IR.Syntax
import Std.Data.ExtTreeSet

/-!
# IR liveness analysis

This module contains the backward may-liveness analysis for the IR, together
with its basic transfer and post-fixpoint lemmas.
-/

namespace MoveModel.IR

/-- Finite sets of live local indices, with extensional equality. -/
abbrev LiveSet := Std.ExtTreeSet LocalIndex

namespace LiveSet

/-- Construct a finite live-local set from a list of local indices. -/
def ofList (xs : List LocalIndex) : LiveSet :=
  Std.ExtTreeSet.ofList xs

end LiveSet

/-- The live set before an instruction, from the live set after it. -/
def liveThroughInstr (i : Instr) (after : LiveSet) : LiveSet :=
  (instrDefs i).foldl (fun live x => live.erase x) after
    |>.union (LiveSet.ofList (instrUses i))

/-- The live set at the start of a block, from the block-exit live set. -/
def liveThroughBlock (blk : Block) (out : LiveSet) : LiveSet :=
  blk.instrs.foldr liveThroughInstr
    ((LiveSet.ofList (termReads blk.term)).union out)

/-- Live-in sets per block (may-liveness, union join), by fixpoint. -/
def liveAnalysis (d : FunDecl) : Array LiveSet := Id.run do
  let n := d.body.size
  let mut liveIn : Array LiveSet := Array.replicate n ∅
  let rounds := n * (d.numLocals + 2) + 2
  for _ in [0:rounds] do
    let mut changed := false
    for b in [0:n] do
      match d.body.blocks b with
      | none => pure ()
      | some blk =>
        let out := (termSuccs blk.term).foldl
          (fun acc s => acc.union (liveIn.getD s ∅)) ∅
        let inn := liveThroughBlock blk out
        unless inn == liveIn.getD b ∅ do
          liveIn := liveIn.set! b inn
          changed := true
    unless changed do
      return liveIn
  return liveIn

/-- Is the liveness result a post-fixpoint (`liveIn ⊇` its transfer)? -/
def liveStable (d : FunDecl) (liveIn : Array LiveSet) : Bool :=
  (List.range d.body.size).all fun b =>
    match d.body.blocks b with
    | none => true
    | some blk =>
      let out := (termSuccs blk.term).foldl
        (fun acc s => acc.union (liveIn.getD s ∅)) ∅
      (liveThroughBlock blk out).toList.all
        ((liveIn.getD b ∅).contains ·)

/-- The live-before set of an instruction suffix. -/
def liveBeforeSuffix (lat : LiveSet) (is : List Instr) : LiveSet :=
  is.foldr liveThroughInstr lat

/-- Live-after sets in instruction order.  The head is the liveness before
the tail of the block, hence immediately after the head instruction. -/
def liveAfterEach (lat : LiveSet) : List Instr → List LiveSet
  | [] => []
  | _ :: rest => liveBeforeSuffix lat rest :: liveAfterEach lat rest

/-- There is one live-after set for every instruction. -/
@[simp] theorem liveAfterEach_length (lat : LiveSet) (is : List Instr) :
    (liveAfterEach lat is).length = is.length := by
  induction is with
  | nil => rfl
  | cons _ is ih => simp [liveAfterEach, ih]

/-- The live set at a block terminator. -/
def liveAtTermIn (liveIn : Array LiveSet) (blk : Block) : LiveSet :=
  (LiveSet.ofList (termReads blk.term)).union
    ((termSuccs blk.term).foldl
      (fun acc s => acc.union (liveIn.getD s ∅)) ∅)

/-- The terminator live set computed using the function's liveness analysis. -/
def liveAtTermOf (d : FunDecl) (blk : Block) : LiveSet :=
  liveAtTermIn (liveAnalysis d) blk

/-- Erasing a list of definitions removes exactly those indices from a set. -/
theorem mem_eraseDefs {x : LocalIndex} {xs : List LocalIndex} {s : LiveSet} :
    x ∈ xs.foldl (fun live y => live.erase y) s ↔ x ∈ s ∧ x ∉ xs := by
  induction xs generalizing s with
  | nil => simp
  | cons y ys ih =>
      simp only [List.foldl_cons, ih, Std.ExtTreeSet.mem_erase,
        List.mem_cons]
      constructor
      · rintro ⟨⟨hyx, hxs⟩, hxys⟩
        exact ⟨hxs, fun h => h.elim (fun hxy => hyx (by simp [hxy])) hxys⟩
      · rintro ⟨hxs, hnot⟩
        exact ⟨⟨fun hyx => hnot (Or.inl
          (Nat.compare_eq_eq.mp hyx).symm), hxs⟩,
          fun hxys => hnot (Or.inr hxys)⟩

/-- Every local used by an instruction is live immediately before it. -/
theorem uses_mem_liveThroughInstr {i : Instr} {after : LiveSet}
    {x : LocalIndex} (h : x ∈ instrUses i) :
    x ∈ liveThroughInstr i after := by
  simp [liveThroughInstr, LiveSet.ofList, h]

/-- A live-after local not defined by an instruction remains live before it. -/
theorem live_liveThroughInstr {i : Instr} {after : LiveSet}
    {x : LocalIndex} (hx : x ∈ after) (hd : x ∉ instrDefs i) :
    x ∈ liveThroughInstr i after := by
  simp [liveThroughInstr, mem_eraseDefs, hx, hd]

/-- Every local read by a terminator belongs to its terminator live set. -/
theorem termReads_mem_liveAtTermIn {liveIn : Array LiveSet}
    {blk : Block} {x : LocalIndex} (h : x ∈ termReads blk.term) :
    x ∈ liveAtTermIn liveIn blk := by
  simp [liveAtTermIn, LiveSet.ofList, h]

/-- A member of the initial set or any joined set survives a union fold. -/
theorem mem_foldl_liveUnion {sets : BlockId → LiveSet} {x : LocalIndex} :
    ∀ {ss : List BlockId} {init : LiveSet},
    (x ∈ init ∨ ∃ s ∈ ss, x ∈ sets s) →
    x ∈ ss.foldl (fun acc s => acc.union (sets s)) init
  | [], _, h =>
      h.elim id fun ⟨_, hs, _⟩ => absurd hs List.not_mem_nil
  | t :: ts, init, h => by
      simp only [List.foldl_cons]
      apply mem_foldl_liveUnion (ss := ts)
      rcases h with hx | ⟨s, hs, hx⟩
      · exact Or.inl (by simp [hx])
      · rcases List.mem_cons.mp hs with heq | hmem
        · subst s
          exact Or.inl (by simp [hx])
        · exact Or.inr ⟨s, hmem, hx⟩

/-- A successor's live-in set is included in its predecessor's terminator set. -/
theorem liveIn_subset_liveAtTermIn {liveIn : Array LiveSet}
    {blk : Block} {succ : BlockId} (hsucc : succ ∈ termSuccs blk.term)
    {x : LocalIndex} (hx : x ∈ liveIn.getD succ ∅) :
    x ∈ liveAtTermIn liveIn blk := by
  have hout : x ∈ (termSuccs blk.term).foldl
      (fun (acc : LiveSet) s => acc.union (liveIn.getD s ∅)) ∅ :=
    mem_foldl_liveUnion (sets := fun s => liveIn.getD s ∅)
      (Or.inr ⟨succ, hsucc, hx⟩)
  simp only [liveAtTermIn, Std.ExtTreeSet.union_eq,
    Std.ExtTreeSet.mem_union_iff]
  exact Or.inr (by
    simpa only [Array.getD_eq_getD_getElem?, Std.ExtTreeSet.union_eq] using hout)

/-- Stability makes the transfer result at a declared block a subset of live-in. -/
theorem liveStable_entry {d : FunDecl} {b : BlockId} {blk : Block}
    (hstable : liveStable d (liveAnalysis d) = true)
    (hb : d.body.blocks b = some blk) (hlt : b < d.body.size)
    {x : LocalIndex}
    (hx : x ∈ liveBeforeSuffix (liveAtTermIn (liveAnalysis d) blk)
      blk.instrs) : x ∈ (liveAnalysis d).getD b ∅ := by
  have hall := List.all_eq_true.mp hstable b
    (by simp [List.mem_range, hlt])
  rw [hb] at hall
  have hxList : x ∈
      (liveThroughBlock blk
        ((termSuccs blk.term).foldl
          (fun acc s => acc.union ((liveAnalysis d).getD s ∅)) ∅)).toList :=
    Std.ExtTreeSet.mem_toList.mpr (by
      simpa [liveThroughBlock, liveBeforeSuffix, liveAtTermIn] using hx)
  have hxall := List.all_eq_true.mp hall x hxList
  simpa [Std.ExtTreeSet.contains_iff_mem] using hxall

end MoveModel.IR
