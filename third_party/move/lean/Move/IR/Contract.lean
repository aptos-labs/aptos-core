-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.IR.Value
import Move.IR.State
import Move.IR.Spec

/-!
# Function Contracts

A Move function specification — `requires`, `aborts_if`, `ensures`,
`modifies` — with the conditions as deep spec expressions (`SpecExp`).  A
function's boundary state is its **global memory** plus its argument values;
caller locals are invisible to a callee, so contracts do not mention a local
store.

## Environment discipline

Which `SpecEnv` components a clause may reference:

* `requires`, `aborts` — evaluated in the **pre-state**: `loc i` is the
  `i`-th argument, `mem` is pre-memory.  No `result`, no labeled memory.
  (`preEnv`)
* `ensures` — evaluated in the **post-state**: `loc i` is the `i`-th
  argument (arguments are call-by-value; the callee's later writes to its
  parameter locals are not visible at the boundary), `result i` is the
  `i`-th return value, `mem` is post-memory, and the pre-state memory is
  reachable through snapshot label `preLabel` (`global`/`exists_` with
  `some preLabel`) — this is the `SaveMem`-based representation of `old(..)`.
  (`postEnv`)
* `modifies` — a list of `(resource, address-expression)` pairs; the address
  expressions are evaluated in the pre-state.  Their denotation is the write
  footprint (`Contract.footprint`).

The intended reading (made precise by `SatisfiesContract` in
`Semantics.lean`): per the FMCAD'26 paper the `aborts_if` clauses form a
**biconditional** — the function aborts *iff* their disjunction holds — which
is strictly stronger than a Hoare-style implication.
-/

namespace Move.IR

/-- A function contract, with deep spec-expression conditions.

`aborts = none` means the specification makes **no claim** about aborts —
the Move Prover's *partial* default when a function has no `aborts_if`
clauses.  With `some e` the abort condition is biconditional (the function
aborts *iff* `e`); an explicit `aborts_if false` forbids aborting. -/
structure Contract where
  requires : SpecExp
  aborts : Option SpecExp
  ensures : SpecExp
  modifies : List (ResourceId × SpecExp)

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

/-- The evaluation environment of pre-state clauses (`requires`, `aborts`,
`modifies` addresses): arguments as locals, pre-memory.  All snapshot
labels also denote the pre-memory, so that a clause cannot distinguish the
current state from a snapshot — pre-state clauses have only one state. -/
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

/-- The write footprint denoted by the `modifies` clause, relative to a
(pre-state) environment: all locations `⟨r, a⟩` such that some listed pair
`(r, e)` has `e` evaluating to the address `a`. -/
def Contract.footprint (c : Contract) (env : SpecEnv) : Footprint :=
  fun loc => ∃ p ∈ c.modifies,
    p.1 = loc.rsrc ∧ EvalSpec env p.2 (.address loc.addr)

end Move.IR
