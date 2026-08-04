-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

/-!
# IVL Syntax: Boogie-Style Programs over Basic Blocks

A Boogie-like intermediate verification language (IVL), the target of the
Move Prover translation.  Its shape is that of a Boogie implementation body:
a partial map from **labels** to **basic blocks**, each a list of
straight-line commands followed by a control-transfer terminator.

* Commands are `assign`/`havoc`/`assume`/`assert` — Boogie's straight-line
  fragment.
* The terminator `goto` carries a list of *guarded* targets: control may
  transfer to any target whose guard holds in the current state.  This fuses
  Boogie's nondeterministic `goto L₁, …, Lₙ` (all guards `True`) with the
  conditional `if (c) { goto L₁ } else { goto L₂ }` (complementary guards)
  that the Boogie backend emits for `Branch` — Boogie itself desugars the
  latter into `goto` plus head `assume`s; here the assume is fused into the
  edge.
* `ret` ends the frame normally (Boogie `return`; also the prover's `Stop`).

The command layer is deep (inductive), the *conditions* are shallow:
guards/assertions are Lean predicates `σ → Prop`, deterministic updates are
functions `σ → σ`, and `havoc` is a relation `σ → σ → Prop`.  The state type
`σ` is a parameter: the IVL is state-polymorphic, and the translation
denotes the deep IR spec expressions into shallow predicates when it
instantiates `σ` with its verification state.

Loops are *annotations*, not syntax: `Anns` maps loop-header labels to a
`LoopAnn` — invariant, loop-target relation (the havoc of the
`LoopAnalysisProcessor`, as a relation), and the member labels of the loop.
The WP calculus (`Wp.lean`) applies the invariant rule at annotated headers;
the `loopCut` transformation (`LoopCut.lean`) is the corresponding
program-level loop-to-DAG reduction.
-/

namespace Move.Prover.Ivl

/-- Block label. -/
abbrev Label := Nat

/-- Straight-line commands over an abstract state type `σ`.

* `assign f` — deterministic update `s ↦ f s` (Boogie `x := e`).
* `havoc R`  — nondeterministic update: any `s'` with `R s s'`.
* `assume p` — assumption: continue only from states satisfying `p`.
* `assert p` — assertion: fails if `p` does not hold. -/
inductive BCmd (σ : Type) where
  | assign (f : σ → σ)
  | havoc (R : σ → σ → Prop)
  | assume (p : σ → Prop)
  | assert (p : σ → Prop)

/-- Block terminators: guarded `goto` or normal end of frame (`ret`). -/
inductive BTerm (σ : Type) where
  | goto (targets : List ((σ → Prop) × Label))
  | ret

/-- The (syntactic) successor edges of a terminator. -/
def BTerm.targetsOf {σ : Type} : BTerm σ → List ((σ → Prop) × Label)
  | .goto targets => targets
  | .ret => []

/-- A basic block. -/
structure BBlock (σ : Type) where
  cmds : List (BCmd σ)
  term : BTerm σ

/-- A program: labeled basic blocks plus an entry label. -/
structure BProgram (σ : Type) where
  blocks : Label → Option (BBlock σ)
  entry : Label

/-- Loop annotation of a header label: the loop invariant `inv`, the
*loop-target relation* `targets` over-approximating what an arbitrary number
of iterations may change (the havoc of the `LoopAnalysisProcessor`, as a
relation), and the labels belonging to the loop (`members` — the blocks of
the prover's fat loop, including the header itself). -/
structure LoopAnn (σ : Type) where
  inv : σ → Prop
  targets : σ → σ → Prop
  members : Label → Prop

/-- Loop annotations of a program: a partial map from header labels. -/
abbrev Anns (σ : Type) := Label → Option (LoopAnn σ)

/-- The empty annotation map (acyclic programs). -/
def noAnns (σ : Type) : Anns σ := fun _ => none

/-- `Edge G l l'`: block `l` has a (syntactic) edge to `l'`. -/
def Edge {σ : Type} (G : BProgram σ) (l l' : Label) : Prop :=
  ∃ blk gt, G.blocks l = some blk ∧ gt ∈ blk.term.targetsOf ∧ gt.2 = l'

end Move.Prover.Ivl
