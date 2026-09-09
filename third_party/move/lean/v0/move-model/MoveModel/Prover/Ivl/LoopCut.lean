-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.Prover.Ivl.Syntax
import MoveModel.Prover.Ivl.Semantics
import MoveModel.Prover.Ivl.Wp
import MoveModel.Prover.Ivl.WpSound

/-!
# Loop Cutting: the `LoopAnalysisProcessor` as a Program Transformation

`wpB` handles loop invariants inside the calculus.  The production Move
Prover instead rewrites loops before verification.  For each loop,
`LoopAnalysisProcessor`:

* at the loop header, insert `assert I; havoc targets; assume I` (base case,
  then an arbitrary target-related iteration state);
* append, per loop, a fresh block `X = assert I; stop` (the induction step);
* retarget every back edge to `X`.

`loopCut` implements the same transformation for IVL programs.  Here `stop`
means `assume False; ret`.  The main results connect the transformed program
to the annotated calculus:

* `loopCut_wp`: annotation-free WP on the cut program equals `wpB` on the
  original annotated program;
* `loopCut_acyclic`: every cut-program edge increases the extended rank, so
  the result is a DAG;
* `wpB_complete`: on an acyclic annotation-free program, safety and
  postcondition validity for every execution imply its weakest precondition.

The cut program allocates one fresh label `base + h` per annotated header
`h`.  `CutOk` requires `base` to exceed every original label.  `cutRank` maps
fresh blocks to `maxR`, which exceeds every original rank.
-/

namespace MoveModel.Prover.Ivl

variable {σ : Type}

/-- Retarget the back edges (targets that do not increase rank) of a
terminator to the fresh induction-step blocks. -/
def cutTerm (rank : Label → Nat) (base : Nat) (l : Label) :
    BTerm σ → BTerm σ
  | .ret => .ret
  | .goto targets => .goto (targets.map fun gt =>
      if rank l < rank gt.2 then gt else (gt.1, base + gt.2))

/-- Cut one block: prepend `assert I; havoc T; assume I` at an annotated
header, and retarget back edges. -/
def cutBlock (anns : Anns σ) (rank : Label → Nat) (base : Nat) (l : Label)
    (blk : BBlock σ) : BBlock σ where
  cmds :=
    (match anns l with
      | some ann => [.assert ann.inv, .havoc ann.targets, .assume ann.inv]
      | none => []) ++ blk.cmds
  term := cutTerm rank base l blk.term

/-- The loop-cut program: original blocks transformed by `cutBlock`, plus
one induction-step block `assert I; stop` at label `base + h` per annotated
header `h`. -/
def loopCut (G : BProgram σ) (anns : Anns σ) (rank : Label → Nat)
    (base : Nat) : BProgram σ where
  blocks := fun l =>
    if l < base then (G.blocks l).map (cutBlock anns rank base l)
    else
      match anns (l - base) with
      | some ann =>
          some ⟨[.assert ann.inv, .assume fun _ => False], .ret⟩
      | none => none
  entry := G.entry

/-- Freshness/bound conditions for `loopCut`: `base` bounds the labels the
original program declares or targets, and `maxR` bounds the ranks of those
labels. -/
structure CutOk (G : BProgram σ) (rank : Label → Nat) (base maxR : Nat) :
    Prop where
  declaredLt : ∀ l, G.blocks l ≠ none → l < base
  targetLt : ∀ l l', Edge G l l' → l' < base
  rankLt : ∀ l, l < base → rank l < maxR

/-- The rank of the cut program: fresh induction-step blocks sit above
everything at `maxR`. -/
def cutRank (rank : Label → Nat) (base maxR : Nat) : Label → Nat :=
  fun l => if l < base then rank l else maxR

/-- `wpCmds` distributes over concatenation. -/
theorem wpCmds_append {cs₁ cs₂ : List (BCmd σ)} {P : σ → Prop} :
    ∀ {s : σ}, wpCmds (cs₁ ++ cs₂) P s ↔ wpCmds cs₁ (wpCmds cs₂ P) s := by
  induction cs₁ with
  | nil => intro s; exact Iff.rfl
  | cons c cs ih =>
    intro s
    cases c with
    | assign f => exact ih
    | havoc R => exact forall_congr' fun s' => imp_congr Iff.rfl ih
    | assume p => exact imp_congr Iff.rfl ih
    | assert p => exact and_congr Iff.rfl ih

/-! ## The cut program is acyclic -/

/-- Every edge of the cut program strictly increases `cutRank`. -/
theorem loopCut_acyclic {G : BProgram σ} {anns : Anns σ}
    {rank : Label → Nat} {base maxR : Nat} (hok : CutOk G rank base maxR) :
    ∀ l l', Edge (loopCut G anns rank base) l l' →
      cutRank rank base maxR l < cutRank rank base maxR l' := by
  rintro l l' ⟨blk, gt, hblk, hmem, rfl⟩
  simp only [loopCut] at hblk
  by_cases hl : l < base
  · rw [if_pos hl] at hblk
    cases hblk₀ : G.blocks l with
    | none => rw [hblk₀] at hblk; cases hblk
    | some blk₀ =>
      rw [hblk₀] at hblk
      injection hblk with he
      subst he
      -- edges come from the retargeted terminator
      cases ht : blk₀.term with
      | ret => simp [cutBlock, cutTerm, BTerm.targetsOf, ht] at hmem
      | goto targets =>
        simp only [cutBlock, cutTerm, ht, BTerm.targetsOf, List.mem_map]
          at hmem
        obtain ⟨gt₀, hmem₀, hgt⟩ := hmem
        have hlt : rank l < maxR := hok.rankLt l hl
        by_cases hr : rank l < rank gt₀.2
        · rw [if_pos hr] at hgt
          subst hgt
          have htlt : gt₀.2 < base :=
            hok.targetLt l gt₀.2 ⟨blk₀, gt₀, hblk₀, by rw [ht]; exact hmem₀,
              rfl⟩
          simp only [cutRank, if_pos hl, if_pos htlt]
          exact hr
        · rw [if_neg hr] at hgt
          subst hgt
          have : ¬ base + gt₀.2 < base := by omega
          simp only [cutRank, if_pos hl, if_neg this]
          exact hlt
  · rw [if_neg hl] at hblk
    -- fresh induction-step blocks have no outgoing edges
    cases hann : anns (l - base) with
    | none => rw [hann] at hblk; cases hblk
    | some ann =>
      rw [hann] at hblk
      injection hblk with he
      subst he
      simp [BTerm.targetsOf] at hmem

/-! ## WP equivalence with the original annotated program -/

section CutWp

variable {G : BProgram σ} {anns : Anns σ} {rank : Label → Nat}
  {base maxR : Nat} {Q : σ → Prop}

/-- Forward direction: the invariant-rule WP of the original program implies
the plain WP of the cut program (with one extra fuel step for the
induction-step blocks). -/
theorem loopCut_wp_mp (hok : CutOk G rank base maxR) :
    ∀ fuel l s, wpB G anns rank Q fuel l s →
      wpB (loopCut G anns rank base) (noAnns σ) (cutRank rank base maxR) Q
        (fuel + 1) l s := by
  intro fuel
  induction fuel with
  | zero => intro l s h; exact absurd h (by simp [wpB])
  | succ fuel ih =>
    intro l s h
    simp only [wpB] at h ⊢
    cases hblk : G.blocks l with
    | none => rw [hblk] at h; exact (h : False).elim
    | some blk =>
      rw [hblk] at h
      have hl : l < base := hok.declaredLt l (by rw [hblk]; exact nofun)
      rw [show (loopCut G anns rank base).blocks l =
          some (cutBlock anns rank base l blk) by
        simp [loopCut, hblk, hl]]
      have h' : wpBlock anns rank Q (wpB G anns rank Q fuel) l blk s := h
      unfold wpBlock at h'
      have hpost : ∀ s',
          wpTerm anns rank Q (wpB G anns rank Q fuel) l blk.term s' →
          wpTerm (noAnns σ) (cutRank rank base maxR) Q
            (wpB (loopCut G anns rank base) (noAnns σ)
              (cutRank rank base maxR) Q (fuel + 1)) l
            (cutTerm rank base l blk.term) s' := by
        intro s' hp
        cases ht : blk.term with
        | ret => rw [ht] at hp; exact hp
        | goto targets =>
          rw [ht] at hp
          intro gt' hmem' hg'
          simp only [List.mem_map] at hmem'
          obtain ⟨gt, hmem, hgt⟩ := hmem'
          by_cases hr : rank l < rank gt.2
          · rw [if_pos hr] at hgt
            subst hgt
            have := hp gt hmem hg'
            rw [if_pos hr] at this
            have htlt : gt.2 < base := hok.targetLt l gt.2
              ⟨blk, gt, hblk, by rw [ht]; exact hmem, rfl⟩
            rw [if_pos (show cutRank rank base maxR l <
              cutRank rank base maxR gt.2 by
                simp only [cutRank, if_pos hl, if_pos htlt]; exact hr)]
            exact ih gt.2 s' this
          · rw [if_neg hr] at hgt
            subst hgt
            have := hp gt hmem hg'
            rw [if_neg hr] at this
            cases hann : anns gt.2 with
            | none => rw [hann] at this; exact (this : False).elim
            | some ann =>
              rw [hann] at this
              have hnb : ¬ base + gt.2 < base := by omega
              rw [if_pos (show cutRank rank base maxR l <
                cutRank rank base maxR (base + gt.2) by
                  simp only [cutRank, if_pos hl, if_neg hnb]
                  exact hok.rankLt l hl)]
              -- unfold the fresh induction-step block
              simp only [wpB]
              rw [show (loopCut G anns rank base).blocks (base + gt.2) =
                  some ⟨[.assert ann.inv, .assume fun _ => False], .ret⟩ by
                simp [loopCut, hnb, hann]]
              exact ⟨this, fun hf => hf.elim⟩
      show wpBlock (noAnns σ) (cutRank rank base maxR) Q _ l
        (cutBlock anns rank base l blk) s
      show wpCmds (cutBlock anns rank base l blk).cmds
        (wpTerm (noAnns σ) (cutRank rank base maxR) Q
          (wpB (loopCut G anns rank base) (noAnns σ)
            (cutRank rank base maxR) Q (fuel + 1)) l
          (cutBlock anns rank base l blk).term) s
      cases hann : anns l with
      | none =>
        rw [hann] at h'
        simp only [cutBlock, hann, List.nil_append]
        exact wpCmds_mono hpost s h'
      | some ann =>
        rw [hann] at h'
        have h1 : ann.inv s := h'.1
        have h2 : ∀ s', ann.targets s s' → ann.inv s' →
            wpCmds blk.cmds
              (wpTerm anns rank Q (wpB G anns rank Q fuel) l blk.term) s' :=
          h'.2
        simp only [cutBlock, hann, List.cons_append, List.nil_append]
        exact ⟨h1, fun s' ht hi => wpCmds_mono hpost s' (h2 s' ht hi)⟩

/-- Backward direction: the plain WP of the cut program implies the
invariant-rule WP of the original program, on the original label space. -/
theorem loopCut_wp_mpr (hok : CutOk G rank base maxR) :
    ∀ fuel l s, l < base →
      wpB (loopCut G anns rank base) (noAnns σ) (cutRank rank base maxR) Q
        fuel l s →
      wpB G anns rank Q fuel l s := by
  intro fuel
  induction fuel with
  | zero => intro l s _ h; exact absurd h (by simp [wpB])
  | succ fuel ih =>
    intro l s hl h
    simp only [wpB] at h ⊢
    cases hblk : G.blocks l with
    | none =>
      rw [show (loopCut G anns rank base).blocks l = none by
        simp [loopCut, hblk, hl]] at h
      exact (h : False).elim
    | some blk =>
      rw [show (loopCut G anns rank base).blocks l =
          some (cutBlock anns rank base l blk) by
        simp [loopCut, hblk, hl]] at h
      have h' : wpCmds (cutBlock anns rank base l blk).cmds
          (wpTerm (noAnns σ) (cutRank rank base maxR) Q
            (wpB (loopCut G anns rank base) (noAnns σ)
              (cutRank rank base maxR) Q fuel) l
            (cutTerm rank base l blk.term)) s := h
      have hpost : ∀ s',
          wpTerm (noAnns σ) (cutRank rank base maxR) Q
            (wpB (loopCut G anns rank base) (noAnns σ)
              (cutRank rank base maxR) Q fuel) l
            (cutTerm rank base l blk.term) s' →
          wpTerm anns rank Q (wpB G anns rank Q fuel) l blk.term s' := by
        intro s' hp
        cases ht : blk.term with
        | ret => rw [ht] at hp; exact hp
        | goto targets =>
          rw [ht] at hp
          simp only [cutTerm] at hp
          intro gt hmem hg
          by_cases hr : rank l < rank gt.2
          · rw [if_pos hr]
            have hmem' : (if rank l < rank gt.2 then gt
                else (gt.1, base + gt.2)) ∈ targets.map fun gt =>
                  if rank l < rank gt.2 then gt else (gt.1, base + gt.2) :=
              List.mem_map_of_mem hmem
            rw [if_pos hr] at hmem'
            have := hp gt hmem' hg
            have htlt : gt.2 < base := hok.targetLt l gt.2
              ⟨blk, gt, hblk, by rw [ht]; exact hmem, rfl⟩
            rw [if_pos (show cutRank rank base maxR l <
              cutRank rank base maxR gt.2 by
                simp only [cutRank, if_pos hl, if_pos htlt]; exact hr)]
              at this
            exact ih gt.2 s' htlt this
          · rw [if_neg hr]
            have hmem' : (if rank l < rank gt.2 then gt
                else (gt.1, base + gt.2)) ∈ targets.map fun gt =>
                  if rank l < rank gt.2 then gt else (gt.1, base + gt.2) :=
              List.mem_map_of_mem hmem
            rw [if_neg hr] at hmem'
            have hcut := hp (gt.1, base + gt.2) hmem' hg
            have hnb : ¬ base + gt.2 < base := by omega
            rw [if_pos (show cutRank rank base maxR l <
              cutRank rank base maxR (base + gt.2) by
                simp only [cutRank, if_pos hl, if_neg hnb]
                exact hok.rankLt l hl)] at hcut
            -- unfold the induction-step block of the cut program
            cases fuel with
            | zero => exact absurd hcut (by simp [wpB])
            | succ fuel =>
              simp only [wpB] at hcut
              cases hann : anns gt.2 with
              | none =>
                rw [show (loopCut G anns rank base).blocks (base + gt.2) =
                    none by simp [loopCut, hnb, hann]] at hcut
                exact (hcut : False).elim
              | some ann =>
                rw [show (loopCut G anns rank base).blocks (base + gt.2) =
                    some ⟨[.assert ann.inv, .assume fun _ => False], .ret⟩
                  by simp [loopCut, hnb, hann]] at hcut
                have hcut' : wpBlock (noAnns σ) (cutRank rank base maxR) Q
                    (wpB (loopCut G anns rank base) (noAnns σ)
                      (cutRank rank base maxR) Q fuel) (base + gt.2)
                    ⟨[.assert ann.inv, .assume fun _ => False], .ret⟩ s' :=
                  hcut
                exact hcut'.1
      show wpBlock anns rank Q (wpB G anns rank Q fuel) l blk s
      unfold wpBlock
      cases hann : anns l with
      | none =>
        simp only [cutBlock, hann, List.nil_append] at h'
        exact wpCmds_mono hpost s h'
      | some ann =>
        simp only [cutBlock, hann, List.cons_append, List.nil_append] at h'
        exact ⟨h'.1, fun s' ht hi => wpCmds_mono hpost s' (h'.2 s' ht hi)⟩

/-- **The invariant-rule WP is the WP of the loop-cut program**: for labels
of the original program, verification conditions coincide. -/
theorem loopCut_wp (hok : CutOk G rank base maxR) {l : Label} {s : σ}
    (hl : l < base) :
    (∃ fuel, wpB (loopCut G anns rank base) (noAnns σ)
      (cutRank rank base maxR) Q fuel l s) ↔
    (∃ fuel, wpB G anns rank Q fuel l s) := by
  constructor
  · rintro ⟨fuel, h⟩
    exact ⟨fuel, loopCut_wp_mpr hok fuel l s hl h⟩
  · rintro ⟨fuel, h⟩
    exact ⟨fuel + 1, loopCut_wp_mp hok fuel l s h⟩

end CutWp

/-! ## Completeness on acyclic programs -/

/-- Completeness of the straight-line WP: if every outcome of a command list
satisfies `P`, its weakest precondition holds. -/
theorem wpCmds_complete {cs : List (BCmd σ)} {P : σ → Prop} :
    ∀ {s : σ}, (∀ o, CmdsExec cs s o → o.sat P) → wpCmds cs P s := by
  induction cs with
  | nil => intro s hall; exact hall (.ok s) .nil
  | cons c cs ih =>
    intro s hall
    cases c with
    | assign f => exact ih fun o h => hall o (.assign h)
    | havoc R => exact fun s' hR => ih fun o h => hall o (.havoc hR h)
    | assume p => exact fun hp => ih fun o h => hall o (.assume hp h)
    | assert p =>
      have hp : p s := Classical.byContradiction fun hnp =>
        hall .fail (.assertFail hnp)
      exact ⟨hp, ih fun o h => hall o (.assertOk hp h)⟩

/-- **Completeness of `wpB` on acyclic programs**: on an annotation-free
program whose edges increase rank, target only declared blocks, and whose
ranks are bounded, `wpB` holds (with enough fuel) at every declared block
from which every execution is safe and ends in `Q`.  Together with
`wpB_sound`, `wpB` is thus *exact* on the DAGs produced by `loopCut`. -/
theorem wpB_complete {G : BProgram σ} {rank : Label → Nat} {maxR : Nat}
    {Q : σ → Prop}
    (hforward : ∀ l l', Edge G l l' → rank l < rank l')
    (hclosed : ∀ l l', Edge G l l' → G.blocks l' ≠ none)
    (hbound : ∀ l, G.blocks l ≠ none → rank l < maxR) :
    ∀ (n : Nat) (l : Label) (s : σ), maxR - rank l ≤ n →
      G.blocks l ≠ none →
      (∀ o, BExec G l s o → o.sat Q) →
      wpB G (noAnns σ) rank Q n l s := by
  intro n
  induction n with
  | zero =>
    intro l s hgap hne _
    exact absurd (hbound l hne) (by omega)
  | succ n ih =>
    intro l s hgap hne hall
    simp only [wpB]
    cases hblk : G.blocks l with
    | none => exact absurd hblk hne
    | some blk =>
      obtain ⟨cs, t⟩ := blk
      show wpCmds cs
        (wpTerm (noAnns σ) rank Q (wpB G (noAnns σ) rank Q n) l t) s
      apply wpCmds_complete
      intro o hcmds
      cases o with
      | fail => exact hall .fail (.fail hblk hcmds)
      | ok s' =>
        cases t with
        | ret => exact hall (.ok s') (.ret hblk hcmds)
        | goto targets =>
          intro gt hmem hg
          have hedge : Edge G l gt.2 :=
            ⟨⟨cs, .goto targets⟩, gt, hblk, hmem, rfl⟩
          rw [if_pos (hforward l gt.2 hedge)]
          have hgap' : maxR - rank gt.2 ≤ n := by
            have := hforward l gt.2 hedge
            omega
          refine ih gt.2 s' hgap' (hclosed l gt.2 hedge) ?_
          intro o' hnext
          exact hall o' (.goto hblk hcmds hmem hg hnext)

end MoveModel.Prover.Ivl
