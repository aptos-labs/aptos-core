-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.Prover.Ivl.Syntax
import Move.Prover.Ivl.Semantics

/-!
# Weakest Preconditions over the Block Graph

The verification condition generator is defined *in Lean* as a
weakest-precondition calculus over the block-structured IVL, rather than
deferring to Boogie.  `wpB G anns rank Q fuel l` is the precondition
guaranteeing that execution from block `l` is safe and every normal outcome
satisfies `Q`.

## Loop handling

`wpB` recurses along the block graph.  Termination of the *definition* is by
`fuel` (structural), but the calculus is designed so that recursion follows
the `rank` order:

* an edge `l → l'` with `rank l < rank l'` is a **forward edge**: `wpB`
  recurses into the target;
* an edge with `rank l' ≤ rank l` is a **back edge**: it must target an
  annotated loop header, and its weakest precondition is just the header's
  invariant — this is the back-edge cut of the `LoopAnalysisProcessor`
  (retarget the back edge to a fresh `assert I; stop` block);
* at an **annotated header**, `wpB` applies the invariant rule: assert `I`,
  then continue for an *arbitrary* state reachable by the loop-target
  relation that satisfies `I` — precisely the `havoc targets; assume I`
  sequence the `LoopAnalysisProcessor` inserts at the header.

A fuel of `0` denotes `False`; `wpB` is monotone in fuel
(`wpB_fuel_mono`), so a verification condition is stated as
`∃ fuel, wpB … fuel …` or with any concrete sufficient fuel.

## Side conditions

`WfProgram` collects what the invariant rule needs to be sound — the
semantic counterpart of the structure the prover's fat-loop recognition and
loop-target analysis guarantee:

* back edges start inside the loop of the header they target
  (`backMember`), loops are entered only through their header
  (`entryAtHeader`), the header belongs to its loop (`headerMember`), and a
  nested loop lies entirely within its enclosing loops (`nestedMember`) —
  the natural-loop structure of a reducible CFG;
* the loop-target relation is reflexive and closed under executions of the
  loop's blocks (`targetsRefl`, `targetsClosed`) — "the loop-target
  analysis is complete".
-/

namespace Move.Prover.Ivl

/-- Weakest precondition of a straight-line command list.  Deliberately not
`@[simp]`: proofs over long straight-line blocks step it one command at a
time (`Translate.wpCmds_onOk_step`) — unfolding the whole nest at once
produces terms whose kernel-checking cost grows superlinearly with the
block length. -/
def wpCmds {σ : Type} : List (BCmd σ) → (σ → Prop) → σ → Prop
  | [], Q => Q
  | .assign f :: cs, Q => fun s => wpCmds cs Q (f s)
  | .havoc R :: cs, Q => fun s => ∀ s', R s s' → wpCmds cs Q s'
  | .assume p :: cs, Q => fun s => p s → wpCmds cs Q s
  | .assert p :: cs, Q => fun s => p s ∧ wpCmds cs Q s

/-- The empty command list is its postcondition (the safe part of the old
`@[simp]` unfolding of `wpCmds`). -/
@[simp] theorem wpCmds_nil {σ : Type} {Q : σ → Prop} {s : σ} :
    wpCmds [] Q s = Q s := rfl

/-! Definitional single-command steps, for proofs that walk a block one
command at a time (see `Translate.wpCmds_onOk_step`). -/

theorem wpCmds_cons_assign {σ : Type} {f : σ → σ} {cs : List (BCmd σ)}
    {Q : σ → Prop} {s : σ} :
    wpCmds (.assign f :: cs) Q s ↔ wpCmds cs Q (f s) := Iff.rfl

theorem wpCmds_cons_havoc {σ : Type} {R : σ → σ → Prop}
    {cs : List (BCmd σ)} {Q : σ → Prop} {s : σ} :
    wpCmds (.havoc R :: cs) Q s ↔ ∀ s', R s s' → wpCmds cs Q s' := Iff.rfl

theorem wpCmds_cons_assume {σ : Type} {p : σ → Prop} {cs : List (BCmd σ)}
    {Q : σ → Prop} {s : σ} :
    wpCmds (.assume p :: cs) Q s ↔ (p s → wpCmds cs Q s) := Iff.rfl

theorem wpCmds_cons_assert {σ : Type} {p : σ → Prop} {cs : List (BCmd σ)}
    {Q : σ → Prop} {s : σ} :
    wpCmds (.assert p :: cs) Q s ↔ p s ∧ wpCmds cs Q s := Iff.rfl

/-- `wpCmds` is monotone in the postcondition. -/
theorem wpCmds_mono {σ : Type} {cs : List (BCmd σ)} {Q Q' : σ → Prop}
    (h : ∀ s, Q s → Q' s) : ∀ s, wpCmds cs Q s → wpCmds cs Q' s := by
  induction cs with
  | nil => exact h
  | cons c cs ih =>
    intro s hs
    cases c with
    | assign f => exact ih _ hs
    | havoc R => exact fun s' hR => ih _ (hs s' hR)
    | assume p => exact fun hp => ih _ (hs hp)
    | assert p => exact ⟨hs.1, ih _ hs.2⟩

/-- Weakest precondition of traversing one `goto` edge `l → gt.2` under the
edge guard `gt.1`: recurse via `wpNext` on a forward edge; on a back edge,
the invariant of the (necessarily annotated) target header. -/
def wpEdge {σ : Type} (anns : Anns σ) (rank : Label → Nat)
    (wpNext : Label → σ → Prop) (l : Label) (gt : (σ → Prop) × Label)
    (s : σ) : Prop :=
  gt.1 s →
    if rank l < rank gt.2 then wpNext gt.2 s
    else (anns gt.2).elim False fun ann => ann.inv s

/-- Weakest precondition of a terminator. -/
def wpTerm {σ : Type} (anns : Anns σ) (rank : Label → Nat) (Q : σ → Prop)
    (wpNext : Label → σ → Prop) (l : Label) : BTerm σ → σ → Prop
  | .ret => Q
  | .goto targets => fun s => ∀ gt ∈ targets, wpEdge anns rank wpNext l gt s

/-- Weakest precondition of one block: the invariant rule at an annotated
header, plain straight-line WP otherwise. -/
def wpBlock {σ : Type} (anns : Anns σ) (rank : Label → Nat) (Q : σ → Prop)
    (wpNext : Label → σ → Prop) (l : Label) (blk : BBlock σ) (s : σ) :
    Prop :=
  (anns l).elim
    (wpCmds blk.cmds (wpTerm anns rank Q wpNext l blk.term) s)
    fun ann =>
      ann.inv s ∧
      ∀ s', ann.targets s s' → ann.inv s' →
        wpCmds blk.cmds (wpTerm anns rank Q wpNext l blk.term) s'

/-- Weakest precondition from the start of block `l`, with `fuel` bounding
the recursion depth along forward edges (`False` when exhausted; see
`wpB_fuel_mono`).  Jumping to an undeclared block is a failure. -/
def wpB {σ : Type} (G : BProgram σ) (anns : Anns σ) (rank : Label → Nat)
    (Q : σ → Prop) : Nat → Label → σ → Prop
  | 0, _ => fun _ => False
  | fuel + 1, l => fun s =>
      (G.blocks l).elim False fun blk =>
        wpBlock anns rank Q (wpB G anns rank Q fuel) l blk s

/-- `wpEdge` is monotone in `wpNext`. -/
theorem wpEdge_mono {σ : Type} {anns : Anns σ} {rank : Label → Nat}
    {wpNext wpNext' : Label → σ → Prop}
    (h : ∀ l' s, wpNext l' s → wpNext' l' s) {l : Label}
    {gt : (σ → Prop) × Label} {s : σ}
    (hw : wpEdge anns rank wpNext l gt s) :
    wpEdge anns rank wpNext' l gt s := by
  intro hg
  have := hw hg
  by_cases hr : rank l < rank gt.2
  · simp only [if_pos hr] at this ⊢
    exact h _ _ this
  · simpa only [if_neg hr] using this

/-- `wpTerm` is monotone in `wpNext`. -/
theorem wpTerm_mono {σ : Type} {anns : Anns σ} {rank : Label → Nat}
    {Q : σ → Prop} {wpNext wpNext' : Label → σ → Prop}
    (h : ∀ l' s, wpNext l' s → wpNext' l' s) {l : Label} {t : BTerm σ}
    {s : σ} (hw : wpTerm anns rank Q wpNext l t s) :
    wpTerm anns rank Q wpNext' l t s := by
  cases t with
  | ret => exact hw
  | goto targets =>
    intro gt hmem
    exact wpEdge_mono h (hw gt hmem)

/-- `wpBlock` is monotone in `wpNext`. -/
theorem wpBlock_mono {σ : Type} {anns : Anns σ} {rank : Label → Nat}
    {Q : σ → Prop} {wpNext wpNext' : Label → σ → Prop}
    (h : ∀ l' s, wpNext l' s → wpNext' l' s) {l : Label} {blk : BBlock σ}
    {s : σ} (hw : wpBlock anns rank Q wpNext l blk s) :
    wpBlock anns rank Q wpNext' l blk s := by
  unfold wpBlock at hw ⊢
  cases hann : anns l with
  | none =>
    rw [hann] at hw
    exact wpCmds_mono (fun s' => wpTerm_mono h) s hw
  | some ann =>
    rw [hann] at hw
    have h1 : ann.inv s := hw.1
    have h2 : ∀ s', ann.targets s s' → ann.inv s' →
        wpCmds blk.cmds (wpTerm anns rank Q wpNext l blk.term) s' := hw.2
    exact ⟨h1, fun s' ht hi =>
      wpCmds_mono (fun s'' => wpTerm_mono h) s' (h2 s' ht hi)⟩

/-- `wpB` is monotone in fuel: more fuel only weakens the condition. -/
theorem wpB_fuel_mono {σ : Type} {G : BProgram σ} {anns : Anns σ}
    {rank : Label → Nat} {Q : σ → Prop} :
    ∀ {fuel fuel' : Nat}, fuel ≤ fuel' → ∀ {l : Label} {s : σ},
      wpB G anns rank Q fuel l s → wpB G anns rank Q fuel' l s := by
  intro fuel
  induction fuel with
  | zero => intro fuel' _ l s h; exact absurd h (by simp [wpB])
  | succ fuel ih =>
    intro fuel' hle l s h
    match fuel', hle with
    | fuel' + 1, hle =>
      simp only [wpB] at h ⊢
      cases hblk : G.blocks l with
      | none => rw [hblk] at h; exact (h : False).elim
      | some blk =>
        rw [hblk] at h
        exact wpBlock_mono (fun l' s' => ih (Nat.le_of_succ_le_succ hle)) h

/-- Side conditions for the soundness of `wpB`'s invariant rule (see the
module docs). -/
structure WfProgram {σ : Type} (G : BProgram σ) (anns : Anns σ)
    (rank : Label → Nat) : Prop where
  /-- A non-forward edge targets an annotated header, from inside its
  loop. -/
  backMember : ∀ l l', Edge G l l' → ¬ rank l < rank l' →
    ∃ ann, anns l' = some ann ∧ ann.members l
  /-- A forward edge into a loop enters at its header. -/
  entryAtHeader : ∀ l l', Edge G l l' → rank l < rank l' →
    ∀ h ann, anns h = some ann → ann.members l' → ann.members l ∨ l' = h
  /-- A header belongs to its own loop. -/
  headerMember : ∀ h ann, anns h = some ann → ann.members h
  /-- A nested loop lies entirely within any loop containing its header. -/
  nestedMember : ∀ h ann h' ann', anns h = some ann → anns h' = some ann' →
    ann'.members h → ∀ l, ann.members l → ann'.members l
  /-- The loop-target relation is reflexive. -/
  targetsRefl : ∀ h ann, anns h = some ann → ∀ s, ann.targets s s
  /-- The loop-target relation is closed under (normal) executions of the
  loop's blocks: the loop-target analysis is complete. -/
  targetsClosed : ∀ h ann, anns h = some ann →
    ∀ l blk, ann.members l → G.blocks l = some blk →
    ∀ s₀ s s', ann.targets s₀ s → CmdsExec blk.cmds s (.ok s') →
    ann.targets s₀ s'

end Move.Prover.Ivl
