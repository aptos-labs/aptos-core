-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean

/-!
# Simp inventory of the denotation

One named set carries the unfolding of a compiled function's denotation
and the weakest-precondition rule of every native operation.  A `verify`
normalizes its goal with this set once, then closes leaves with decision
procedures; nothing in it selects a route or matches a goal shape.
-/

/-- Denotation unfolding and native weakest-precondition rules. -/
register_simp_attr vir_denote

/-- The general normal-form lemmas of verification conditions: logic,
encodings, runtime accessors, and injectivity.  Precomputed here so that a
normalization names only the per-target constants. -/
register_simp_attr vir_denote_norm

/-- Ground evaluation: literal arithmetic, comparisons, casts, string
equality, and decided conditionals, so a normalization settles the guards
of an operation at literal arguments. -/
register_simp_attr vir_denote_eval

/-- The lemmas a discharger rewrites a side condition with. A discharger
runs in a state simp discards afterwards, so a lemma it names that is not
an equation would leave its auxiliary simp theorem behind in that state and
the proof naming it without a declaration; registered here, the auxiliary
theorem is made with the declaration. -/
register_simp_attr vir_denote_side

open Lean Elab Command Meta in
/-- Put the equations of recursive definitions into `vir_denote` as rewrite
rules taken before a term's arguments are visited. A rewrite by an equation
leaves its instance in the proof, which the kernel checks by instantiating
the equation's statement; an unfolding leaves nothing, and the kernel
re-derives it by reducing the definition's compiled recursion, whose size
is that of the whole definition. -/
elab "vir_denote_equations " ids:ident* : command => do
  for id in ids do
    let name ← liftCoreM <| realizeGlobalConstNoOverloadWithInfo id
    let some eqns ← liftTermElabM <| getEqnsFor? name
      | throwErrorAt id m!"`{name}` has no equations"
    let eqnIds := eqns.map fun eqn => mkIdent eqn
    elabCommand (← `(attribute [vir_denote ↓] $eqnIds*))
