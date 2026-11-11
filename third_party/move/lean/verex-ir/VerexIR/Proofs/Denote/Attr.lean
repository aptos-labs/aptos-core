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

/-- The specification forms of the normal form's role predicates: how an
intrinsic operation's atom (an abort predicate, an observation) reads in a
specification's vocabulary. Applied where a proof meets a clause, on the
authored path; the automatic normal form keeps the atoms. -/
register_simp_attr vir_denote_spec

/-- Ground evaluation: literal arithmetic, comparisons, casts, string
equality, and decided conditionals, so a normalization settles the guards
of an operation at literal arguments. -/
register_simp_attr vir_denote_eval

/-- The transport normal form: a value, an operation or a contract at the
frame a call's type arguments induce, read at the caller's frame. A
callee's contract specialized at a call's type arguments is its generic
contract normalized by this set alone; the call normalizes it further at its
arguments, as a contract stated at the caller's frame. -/
register_simp_attr vir_transport

/-- The lemmas a discharger rewrites a side condition with. A discharger
runs in a state simp discards afterwards, so a lemma it names that is not
an equation would leave its auxiliary simp theorem behind in that state and
the proof naming it without a declaration; registered here, the auxiliary
theorem is made with the declaration. -/
register_simp_attr vir_denote_side

/-- What decides a variant split's case once its computations at the
substituted constructors are evaluated: equations of literals and of
constructors, and the connectives and conditionals they feed. -/
register_simp_attr variant_decision

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

/-!
## The recursive specification functions the verifier has defined

A definition and the theorem that unfolds it once are emitted together,
and the leaf reads the theorem and the measure the recursion descends on
from here rather than from the definition's name.
-/

namespace VerexIR.Proofs.Denote

open Lean

/-- A recursive specification function's definition, as the leaf reads it:
the theorem that unfolds the definition once at its arguments, and the
measure the recursion descends on.  Both are stated at the definition's
leading parameters — what its body reads besides its arguments — followed
by the bundled arguments, so an application's own arguments instantiate
them. -/
structure SpecDefinition where
  /-- The unfolding theorem, at the parameters and the bundle. -/
  unfold : Name
  /-- The measure, a function of the parameters and the bundle into `Nat`.
  An application whose measure is a literal unfolds at most that often,
  since every recursive call is at a smaller measure. -/
  measure : Expr
  deriving Inhabited

/-- The recursive specification functions defined so far, by the constant
each defines: written where the definition and its unfolding theorem are
emitted, read where a leaf evaluates an application.  An extension, so
that nothing reads a definition out of the spelling of its name. -/
initialize specDefinitions :
    SimplePersistentEnvExtension (Name × SpecDefinition) (NameMap SpecDefinition) ←
  registerSimplePersistentEnvExtension {
    addEntryFn := fun map (name, definition) => map.insert name definition
    addImportedFn := fun entries =>
      mkStateFromImportedEntries (fun map (name, definition) => map.insert name definition) {}
        entries }

/-- The definition a constant is of, where it is a recursive specification
function's. -/
def specDefinition? (env : Environment) (name : Name) : Option SpecDefinition :=
  (specDefinitions.getState env).find? name

end VerexIR.Proofs.Denote
