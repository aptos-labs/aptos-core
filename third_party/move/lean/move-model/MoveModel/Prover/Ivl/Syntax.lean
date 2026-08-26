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
* `goto` contains guarded targets.  Control may choose any target whose guard
  holds.  Guards set to `True` model Boogie's nondeterministic multi-target
  `goto`.  Complementary guards model a conditional branch.  Boogie expresses
  the latter with target-block assumptions; this IVL stores those conditions
  directly on edges.
* `ret` ends the frame normally (Boogie `return`; also the prover's `Stop`).

Commands form a deep inductive syntax, while their conditions are shallow.
Guards and assertions are Lean predicates `σ → Prop`; assignments are
functions `σ → σ`; havoc is a relation `σ → σ → Prop`.

The state type `σ` is a parameter, so the IVL is state polymorphic.  The Move
translation instantiates it with the verification state and denotes deep IR
specifications as shallow predicates.

Loops are annotations rather than commands.  `Anns` maps each loop header to
its invariant, target relation, and member labels.  `Wp.lean` applies the
invariant rule at annotated headers.  `LoopCut.lean` gives the equivalent
program transformation from loops to a DAG.
-/

namespace MoveModel.Prover.Ivl

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

end MoveModel.Prover.Ivl
