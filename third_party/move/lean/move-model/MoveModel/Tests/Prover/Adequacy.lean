-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.Prover.Translate.Adequacy

/-!
# Adequacy Boundary Witness

This module instantiates every assumption of `prover_sound` for a minimal
straight-line program.  The witness provides source typing, compiled-IVL
well-formedness, and verification certificates together.  It therefore
shows that the public adequacy theorem is applicable and not vacuous because
of inconsistent hypotheses.
-/

namespace Tests.Prover.Adequacy

open MoveModel.IR
open MoveModel.Prover.Ivl
open MoveModel.Prover.Translate

/-- The one-block body of the identity function. -/
def idBody : Cfg where
  blocks := fun b => if b = 0 then some ⟨[], .ret [0]⟩ else none
  entry := 0
  size := 1

/-- A typed identity function with no loop annotations. -/
def idDecl : FunDecl where
  numParams := 1
  numLocals := 1
  locals := fun x => if x = 0 then some .u64 else none
  returns := [.u64]
  body := idBody
  loopSpecs := fun _ => none
  contract := {
    requires := .value (.bool true)
    aborts := none
    ensures := .value (.bool true)
    modifies := []
  }

/-- The singleton program containing `idDecl`. -/
def prog : Program where
  funs := fun f => if f = 0 then some idDecl else none
  structs := fun _ => none

/-- The singleton identity program satisfies the source bytecode typing
certificate required by adequacy. -/
theorem prog_wf : WfProg prog := by
  intro f d hd
  by_cases hf : f = 0
  · subst f
    simp only [prog, if_pos, Option.some.injEq] at hd
    subst d
    constructor
    · intro b hb
      simp [idDecl, idBody] at hb ⊢
      exact hb
    · intro b blk hb i hi
      simp [idDecl, idBody] at hb
      rcases hb with ⟨rfl, rfl⟩
      simp at hi
    · intro b blk hb
      simp [idDecl, idBody] at hb
      rcases hb with ⟨rfl, rfl⟩
      exact .ret (by simp [idDecl]) (.cons (.refl _ _) .nil)
  · simp [prog, hf] at hd

/-- Every edge of the compiled identity function increases the identity
rank; there are no loop annotations. -/
theorem compiled_wf :
    WfProgram (compileFun prog idDecl) (compAnns prog idDecl) (fun l => l) := by
  have hann : compAnns prog idDecl = noAnns VState := by
    funext l
    cases l <;> simp [compAnns, idDecl, noAnns]
  rw [hann]
  apply WfProgram.noAnns_of_forward
  intro l l' hedge
  rcases hedge with ⟨blk, gt, hblk, hmem, rfl⟩
  by_cases h0 : l = 0
  · subst l
    simp [compileFun, idDecl] at hblk
    subst blk
    simp [BTerm.targetsOf] at hmem
    rcases hmem with rfl
    omega
  by_cases h2 : l = 2
  · subst l
    dsimp [compileFun, idDecl, idBody] at hblk
    simp at hblk
    subst blk
    simp [retExitBlock, BTerm.targetsOf] at hmem
  by_cases h3 : l = 3
  · subst l
    dsimp [compileFun, idDecl, idBody] at hblk
    simp at hblk
    subst blk
    simp [abortExitBlock, BTerm.targetsOf] at hmem
  have hl : l = 1 := by
    by_cases hsub : l - 1 = 0
    · cases l with
      | zero => exact (h0 rfl).elim
      | succ n =>
        cases n with
        | zero => rfl
        | succ n => simp at hsub
    · simp [compileFun, idDecl, idBody, h0, h2, h3, hsub] at hblk
  subst l
  simp [compileFun, idDecl, idBody, compileBlock, termGoto] at hblk
  subst blk
  simp [BTerm.targetsOf] at hmem
  rcases hmem with rfl | rfl <;> omega

/-- The identity function's verification condition is inhabited. -/
theorem id_verified : Verified prog 0 := by
  refine ⟨idDecl, by simp [prog], 3, ?_⟩
  intro m args current frames
  simp [wpB, compileFun, compAnns, idDecl, idBody, compileBlock, termCmds,
    termGoto, retExitBlock, abortExitBlock, wpBlock, wpTerm, wpEdge, wpCmds,
    typedEntry, initVStateAt, Holds, MoveState.locals, Contract.footprint,
    agreesOutside]
  intro htyped _
  obtain ⟨v, rfl⟩ : ∃ v, args = [v] := by
    have hlen := htyped.1
    cases args with
    | nil => simp at hlen
    | cons a rest =>
      cases rest with
      | nil => exact ⟨a, rfl⟩
      | cons b rest => simp at hlen
  intro a b hab hflag
  rcases hab with ⟨rfl, rfl⟩ | ⟨rfl, rfl⟩
  · simp [initLocals, flagSet] at hflag
  · simp [initLocals, flagClear] at hflag
    simp [initLocals, wpCmds, Contract.abortsFalse]

/-- An undeclared call is compiled to an explicit verification failure,
rather than disappearing as a silent no-op. -/
theorem undeclared_call_fails :
    compileInstr prog (.call [] (.function 1) []) = refFail := by
  simp [compileInstr, prog]

/-- All hypotheses of `prover_sound` are instantiated on `prog`, yielding
the semantic contract theorem for its declared identity function. -/
theorem id_satisfies_contract : SatisfiesContract prog 0 idDecl := by
  exact prover_sound prog prog_wf
    (by
      intro f d hd
      have hf : f = 0 := by
        by_cases h : f = 0
        · exact h
        · simp [prog, h] at hd
      subst f
      have hd' : d = idDecl := by simpa [prog] using hd.symm
      subst d
      exact compiled_wf)
    (by
      intro f d hd
      have hf : f = 0 := by
        by_cases h : f = 0
        · exact h
        · simp [prog, h] at hd
      subst f
      exact id_verified)
    0 idDecl (by simp [prog])

end Tests.Prover.Adequacy
