-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.Prover.Ivl.Syntax
import MoveModel.Prover.Ivl.Semantics
import MoveModel.Prover.Ivl.Wp

/-!
# Soundness of the WP Calculus

The anchor theorem of the IVL layer:

> If `wpB G anns rank Q fuel l s` holds and the program is well-formed
> (`WfProgram G anns rank`), then every execution of `G` from block `l`
> avoids assertion failure and every normal outcome satisfies `Q`.

The key case is an annotated loop header.  `wpB` checks the entry state, an
arbitrary target-related iteration state, and the invariant at each back
edge.  `BExec`, by contrast, executes concrete iterations.

The proof inducts over the `BExec` derivation and carries two facts:

* the straight-line WP fact for the current block (`wpCmds … (EdgeCond …)`),
  with fuel-specific forward references weakened to fuel-existential ones
  (`EdgeCond`), and
* `Ctx`, the context for every loop containing the current block.  It records
  the entry state `s₀` and the header fact `HFact`.  The target relation links
  `s₀` to the current state, and `targetsClosed` preserves that link across
  loop blocks.

At a back edge, `wpB` supplies the invariant and `Ctx` supplies the anchored
header fact.  Together they restart the header.  This is the semantic
justification for the processor's havoc/assume encoding.
-/

namespace MoveModel.Prover.Ivl

variable {σ : Type} {G : BProgram σ} {anns : Anns σ} {rank : Label → Nat}
  {Q : σ → Prop}

/-- Soundness of the straight-line WP: an execution of a command list from a
state satisfying `wpCmds cs P` cannot fail, and ends in `P`. -/
theorem wpCmds_sound {cs : List (BCmd σ)} {P : σ → Prop} :
    ∀ {s : σ} {o : Outcome σ}, wpCmds cs P s → CmdsExec cs s o → o.sat P := by
  induction cs with
  | nil =>
    intro s o hwp hexec
    cases hexec
    exact hwp
  | cons c cs ih =>
    intro s o hwp hexec
    cases c with
    | assign f =>
      cases hexec with
      | assign h => exact ih hwp h
    | havoc R =>
      cases hexec with
      | havoc hR h => exact ih (hwp _ hR) h
    | assume p =>
      cases hexec with
      | assume hp h => exact ih (hwp hp) h
    | assert p =>
      cases hexec with
      | assertOk hp h => exact ih hwp.2 h
      | assertFail hp => exact absurd hwp.1 hp

/-- The edge condition guaranteed at a block's terminator, with the fuel of
forward references existentially quantified (`wpB` is stated for a concrete
fuel; the soundness induction must not depend on it). -/
def EdgeCond (G : BProgram σ) (anns : Anns σ) (rank : Label → Nat)
    (Q : σ → Prop) (l : Label) : BTerm σ → σ → Prop
  | .ret => Q
  | .goto targets => fun s =>
      ∀ gt ∈ targets, gt.1 s →
        if rank l < rank gt.2 then ∃ fuel, wpB G anns rank Q fuel gt.2 s
        else ∃ ann, anns gt.2 = some ann ∧ ann.inv s

/-- A fuel-specific `wpTerm` implies the fuel-existential `EdgeCond`. -/
theorem wpTerm_edgeCond {fuel : Nat} {l : Label} {t : BTerm σ} {s : σ}
    (h : wpTerm anns rank Q (wpB G anns rank Q fuel) l t s) :
    EdgeCond G anns rank Q l t s := by
  cases t with
  | ret => exact h
  | goto targets =>
    intro gt hmem hg
    have hcond := h gt hmem hg
    by_cases hr : rank l < rank gt.2
    · rw [if_pos hr] at hcond ⊢
      exact ⟨fuel, hcond⟩
    · rw [if_neg hr] at hcond ⊢
      cases hann : anns gt.2 with
      | none => rw [hann] at hcond; exact (hcond : False).elim
      | some ann => rw [hann] at hcond; exact ⟨ann, rfl, hcond⟩

/-- The invariant-rule fact of loop header `h`, anchored at the loop-entry
state `s₀`: any target-related state satisfying the invariant passes the
header block. -/
def HFact (G : BProgram σ) (anns : Anns σ) (rank : Label → Nat)
    (Q : σ → Prop) (h : Label) (ann : LoopAnn σ) (s₀ : σ) : Prop :=
  ∀ s', ann.targets s₀ s' → ann.inv s' →
    ∃ blk, G.blocks h = some blk ∧
      wpCmds blk.cmds (EdgeCond G anns rank Q h blk.term) s'

/-- The loop context at block `l`, state `s`: every loop containing `l` has
an anchor related to `s` whose invariant-rule fact holds. -/
def Ctx (G : BProgram σ) (anns : Anns σ) (rank : Label → Nat) (Q : σ → Prop)
    (l : Label) (s : σ) : Prop :=
  ∀ h ann, anns h = some ann → ann.members l →
    ∃ s₀, ann.inv s₀ ∧ ann.targets s₀ s ∧ HFact G anns rank Q h ann s₀

/-- Arriving at a block with a `wpB` fact: yields the straight-line WP fact
for the block, and the full loop context — anchors of already-entered loops
are supplied by `hctx`, and if `l` is itself a header, its fresh anchor is
`s` itself (by reflexivity of the loop-target relation). -/
theorem arrive (hwf : WfProgram G anns rank) {l : Label} {s : σ}
    (hfact : ∃ fuel, wpB G anns rank Q fuel l s)
    (hctx : ∀ h ann, anns h = some ann → ann.members l → l ≠ h →
      ∃ s₀, ann.inv s₀ ∧ ann.targets s₀ s ∧
        HFact G anns rank Q h ann s₀) :
    (∃ blk, G.blocks l = some blk ∧
      wpCmds blk.cmds (EdgeCond G anns rank Q l blk.term) s) ∧
    Ctx G anns rank Q l s := by
  obtain ⟨fuel, hwp⟩ := hfact
  cases fuel with
  | zero => exact absurd hwp (by simp [wpB])
  | succ fuel =>
    simp only [wpB] at hwp
    cases hblk : G.blocks l with
    | none => rw [hblk] at hwp; exact (hwp : False).elim
    | some blk =>
      rw [hblk] at hwp
      have hwp' : wpBlock anns rank Q (wpB G anns rank Q fuel) l blk s := hwp
      unfold wpBlock at hwp'
      cases hann : anns l with
      | none =>
        rw [hann] at hwp'
        refine ⟨⟨blk, rfl, wpCmds_mono (fun s' => wpTerm_edgeCond) s hwp'⟩, ?_⟩
        intro h ann hh hmem
        refine hctx h ann hh hmem (fun he => ?_)
        subst he; rw [hann] at hh; cases hh
      | some ann =>
        rw [hann] at hwp'
        have hInv : ann.inv s := hwp'.1
        have hAll : ∀ s', ann.targets s s' → ann.inv s' →
            wpCmds blk.cmds
              (wpTerm anns rank Q (wpB G anns rank Q fuel) l blk.term) s' :=
          hwp'.2
        have hHF : HFact G anns rank Q l ann s := fun s' ht hi =>
          ⟨blk, hblk, wpCmds_mono (fun s'' => wpTerm_edgeCond) s'
            (hAll s' ht hi)⟩
        refine ⟨⟨blk, rfl, wpCmds_mono (fun s'' => wpTerm_edgeCond) s
          (hAll s (hwf.targetsRefl l ann hann s) hInv)⟩, ?_⟩
        intro h ann' hh hmem
        by_cases he : l = h
        · subst he
          rw [hann] at hh
          injection hh with he'; subst he'
          exact ⟨s, hInv, hwf.targetsRefl l ann hann s, hHF⟩
        · exact hctx h ann' hh hmem he

/-- Main induction for `wpB_sound`, over the execution derivation. -/
theorem sound_aux (hwf : WfProgram G anns rank) :
    ∀ {l : Label} {s : σ} {o : Outcome σ}, BExec G l s o →
    ∀ blk, G.blocks l = some blk →
    wpCmds blk.cmds (EdgeCond G anns rank Q l blk.term) s →
    Ctx G anns rank Q l s →
    o.sat Q := by
  intro l s o hexec
  induction hexec with
  | @fail l cs t s hblk hcmds =>
    intro blk hblk' hwp _hctx
    rw [hblk] at hblk'; injection hblk' with he; subst he
    have hf := wpCmds_sound hwp hcmds
    exact hf
  | @ret l cs s s' hblk hcmds =>
    intro blk hblk' hwp _hctx
    rw [hblk] at hblk'; injection hblk' with he; subst he
    exact wpCmds_sound hwp hcmds
  | @goto l cs targets s s' gt o hblk hcmds hmem hg _hnext ih =>
    intro blk hblk' hwp hctx
    rw [hblk] at hblk'; injection hblk' with he; subst he
    have hpost := wpCmds_sound hwp hcmds
    have hedge := hpost gt hmem hg
    have hEdge : Edge G l gt.2 := ⟨⟨cs, .goto targets⟩, gt, hblk, hmem, rfl⟩
    by_cases hr : rank l < rank gt.2
    · -- forward edge
      rw [if_pos hr] at hedge
      have hctx' : ∀ h ann, anns h = some ann → ann.members gt.2 →
          gt.2 ≠ h →
          ∃ s₀, ann.inv s₀ ∧ ann.targets s₀ s' ∧
            HFact G anns rank Q h ann s₀ := by
        intro h ann hh hmem' hne
        rcases hwf.entryAtHeader l gt.2 hEdge hr h ann hh hmem' with hml | he
        · obtain ⟨s₀, hi₀, ht, hHF⟩ := hctx h ann hh hml
          exact ⟨s₀, hi₀, hwf.targetsClosed h ann hh l _ hml hblk s₀ s s'
            hi₀ ht hcmds, hHF⟩
        · exact absurd he hne
      obtain ⟨⟨blk', hblk'', hwp''⟩, hctx''⟩ := arrive hwf hedge hctx'
      exact ih blk' hblk'' hwp'' hctx''
    · -- back edge: reconstruct the header entry from the context anchor
      rw [if_neg hr] at hedge
      obtain ⟨ann, hann, hinv⟩ := hedge
      obtain ⟨ann₂, hann₂, hml⟩ := hwf.backMember l gt.2 hEdge hr
      rw [hann] at hann₂; injection hann₂ with he; subst he
      obtain ⟨s₀, hi₀, ht, hHF⟩ := hctx gt.2 ann hann hml
      have ht' : ann.targets s₀ s' :=
        hwf.targetsClosed gt.2 ann hann l _ hml hblk s₀ s s' hi₀ ht hcmds
      obtain ⟨blk', hblk'', hwp''⟩ := hHF s' ht' hinv
      have hctx'' : Ctx G anns rank Q gt.2 s' := by
        intro h' ann' hh' hmem'
        have hml' : ann'.members l :=
          hwf.nestedMember gt.2 ann h' ann' hann hh' hmem' l hml
        obtain ⟨s₀', hi₀', ht₀, hHF'⟩ := hctx h' ann' hh' hml'
        exact ⟨s₀', hi₀', hwf.targetsClosed h' ann' hh' l _ hml' hblk s₀'
          s s' hi₀' ht₀ hcmds, hHF'⟩
      exact ih blk' hblk'' hwp'' hctx''

/-- **Soundness of `wpB`**: if the verification condition holds at block `l`
(for any fuel), every execution from `l` avoids assertion failure and every
normal outcome satisfies `Q`.  `hstart` requires the start block to be
outside all loops — except that it may itself be a loop header. -/
theorem wpB_sound (hwf : WfProgram G anns rank) {l : Label} {s : σ}
    {o : Outcome σ} {fuel : Nat}
    (hstart : ∀ h ann, anns h = some ann → ann.members l → h = l)
    (hwp : wpB G anns rank Q fuel l s) (hexec : BExec G l s o) :
    o.sat Q := by
  have hctx : ∀ h ann, anns h = some ann → ann.members l → l ≠ h →
      ∃ s₀, ann.inv s₀ ∧ ann.targets s₀ s ∧
        HFact G anns rank Q h ann s₀ := by
    intro h ann hh hmem hne
    exact absurd (hstart h ann hh hmem).symm hne
  obtain ⟨⟨blk, hblk, hwp'⟩, hctx'⟩ := arrive hwf ⟨fuel, hwp⟩ hctx
  exact sound_aux hwf hexec blk hblk hwp' hctx'

/-- Verification implies absence of assertion failures. -/
theorem wpB_safe (hwf : WfProgram G anns rank) {l : Label} {s : σ}
    {fuel : Nat}
    (hstart : ∀ h ann, anns h = some ann → ann.members l → h = l)
    (hwp : wpB G anns rank Q fuel l s) : ¬ BExec G l s .fail :=
  fun hexec => wpB_sound hwf hstart hwp hexec

/-- Verification implies the postcondition on normal termination. -/
theorem wpB_post (hwf : WfProgram G anns rank) {l : Label} {s s' : σ}
    {fuel : Nat}
    (hstart : ∀ h ann, anns h = some ann → ann.members l → h = l)
    (hwp : wpB G anns rank Q fuel l s) (hexec : BExec G l s (.ok s')) :
    Q s' :=
  wpB_sound hwf hstart hwp hexec

end MoveModel.Prover.Ivl
