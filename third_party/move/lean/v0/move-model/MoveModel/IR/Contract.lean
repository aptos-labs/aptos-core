-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.IR.Value
import MoveModel.IR.State
import MoveModel.IR.Spec

/-!
# Function Contracts

A contract contains `requires`, `aborts_if`, `ensures`, and `modifies`.
Conditions use the deep expression type `SpecExp`.

A function boundary exposes global memory and argument values.  Caller locals
are not visible to the callee, so they do not appear in contract environments.

## Environment discipline

Which `SpecEnv` components a clause may reference:

* `requires` uses `preEnv`.  Here `loc i` is argument `i` and `mem` is the
  entry memory.  Results and labeled memories are unavailable.
* `aborts` has two production-Prover views.  Opaque calls evaluate it in
  `preEnv`; definition exit checks evaluate it in `abortEnv`, where current
  memory is the exit memory and snapshots denote entry memory.
* `ensures` uses `postEnv`.  `loc i` still denotes entry argument `i`, because
  later writes to parameter locals are not part of the call boundary.
  `result i` is return value `i`, and `mem` is the final memory.  Memory at
  entry is available through `preLabel`; this is how `old(..)` is represented.
* `modifies` lists `(resource, address-expression)` pairs.  Address
  expressions use the pre-state.  Their denotation is `Contract.footprint`.

`SatisfiesContract` gives `aborts_if` the biconditional interpretation used in
the FMCAD'26 paper: a function aborts exactly when the disjunction of its
`aborts_if` clauses holds.  This is stronger than a one-way Hoare implication.
-/

namespace MoveModel.IR

/-- A function contract, with deep spec-expression conditions.

`aborts = none` means the specification makes **no claim** about aborts —
the Move Prover's *partial* default when a function has no `aborts_if`
clauses.  With `some e` the abort condition is biconditional (the function
aborts *iff* `e`); an explicit `aborts_if false` forbids aborting. -/
structure Contract where
  requires : SpecExp
  aborts : Option SpecExp
  ensures : SpecExp
  /-- The complete permitted write footprint.  Unlike the production Move
  Prover's opt-in frame checking, an empty list in this model means that
  global memory must remain unchanged; it does not disable frame checking. -/
  modifies : List (ResourceId × SpecExp)

/-- Specialize every type-bearing part of a function contract.  Resource
selectors are still nongeneric at this layer; their address expressions can
nevertheless contain instantiated quantifiers. -/
def Contract.instantiate (args : List Ty) (c : Contract) : Contract where
  requires := c.requires.instantiate args
  aborts := c.aborts.map (·.instantiate args)
  ensures := c.ensures.instantiate args
  modifies := c.modifies.map fun (resource, address) =>
    (resource, address.instantiate args)

/-- The abort condition holds (abort-path completeness); vacuous for
`none`. -/
def Contract.abortsHolds (c : Contract) (env : SpecEnv) : Prop :=
  match c.aborts with
  | none => True
  | some e => Holds env e

/-- The abort condition evaluates to `false` (normal-path tightness);
vacuous for `none`. -/
def Contract.abortsFalse (c : Contract) (env : SpecEnv) : Prop :=
  match c.aborts with
  | none => True
  | some e => EvalSpec env e (.bool false)

/-- The evaluation environment of pre-state clauses (`requires`, opaque-call
`aborts`, and `modifies` addresses): arguments as locals and entry memory.
All snapshot labels also denote entry memory. -/
def preEnv (Δ : StructDecls) (m : Memory) (args : List Value) : SpecEnv where
  structs := Δ
  locals := initLocals args
  result := []
  mem := m
  snaps := fun _ => m
  bound := []

/-- The evaluation environment of the `ensures` clause: arguments as
locals, return values as `result`, post-memory as current memory, and
the pre-memory behind every snapshot label (contracts use only `preLabel`). -/
def postEnv (Δ : StructDecls) (mPre mPost : Memory) (args rets : List Value) :
    SpecEnv where
  structs := Δ
  locals := initLocals args
  result := rets
  mem := mPost
  snaps := fun _ => mPre
  bound := []

/-- The definition-side environment of an `aborts_if` clause: arguments are
entry values, current memory is the memory at the normal or abort exit, and
snapshot labels denote entry memory.  Results are unavailable in abort
clauses. -/
def abortEnv (Δ : StructDecls) (mPre mExit : Memory) (args : List Value) :
    SpecEnv :=
  postEnv Δ mPre mExit args []

/-- Both interpretations of `aborts_if` are false: the pre-state view used at
opaque calls and the exit-state view checked at a function definition. -/
@[simp] def Contract.abortsFalseAtExit (c : Contract) (Δ : StructDecls)
    (mPre mExit : Memory) (args : List Value) : Prop :=
  c.abortsFalse (preEnv Δ mPre args) ∧
  c.abortsFalse (abortEnv Δ mPre mExit args)

/-- Both interpretations of `aborts_if` hold: the pre-state view used at
opaque calls and the exit-state view checked at a function definition. -/
@[simp] def Contract.abortsHoldsAtExit (c : Contract) (Δ : StructDecls)
    (mPre mExit : Memory) (args : List Value) : Prop :=
  c.abortsHolds (preEnv Δ mPre args) ∧
  c.abortsHolds (abortEnv Δ mPre mExit args)

/-- The write footprint denoted by the `modifies` clause, relative to a
(pre-state) environment: all locations `⟨r, a⟩` such that some listed pair
`(r, e)` has `e` evaluating to the address `a`. -/
def Contract.footprint (c : Contract) (env : SpecEnv) : Footprint :=
  fun loc => ∃ p ∈ c.modifies,
    p.1 = loc.rsrc ∧ EvalSpec env p.2 (.address loc.addr)

end MoveModel.IR
