-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.IR.Value

/-!
# IR States

The state of a bytecode computation consists of

* the function-local store `locals` (a partial map from locals to values;
  the language is stackless bytecode: there is no operand stack, all instruction
  operands are local indices — `TempIndex` in the Rust implementation), and
* the *type-indexed global memory* `memory` (TACAS'22 §2): a resource value is
  addressed by a pair of a resource type and an account address, mirroring the
  Move Prover's Boogie encoding `memory : ResourceId → Address → Option Value`.

`Footprint`s (sets of global locations, as predicates) represent `modifies`
clauses; `agreesOutside` is the frame condition stating that a memory
transition only touched a footprint.
-/

namespace Move.IR

/-- The local store of a function activation. `none` = uninitialized. -/
abbrev Locals := LocalIndex → Option Value

/-- Type-indexed global memory: `memory r a` is the resource of type `r`
stored at address `a`, if present. -/
abbrev Memory := ResourceId → Address → Option Value

/-- The initial local store of a function activation: locals
`0..args.length-1` hold the arguments, everything else is uninitialized. -/
def initLocals (args : List Value) : Locals :=
  fun x => args[x]?

/-- Store a resource of type `r` at address `a`. -/
def memWrite (m : Memory) (r : ResourceId) (a : Address) (v : Value) : Memory :=
  fun r' a' => if r' = r ∧ a' = a then some v else m r' a'

/-- Remove the resource of type `r` at address `a`. -/
def memRemove (m : Memory) (r : ResourceId) (a : Address) : Memory :=
  fun r' a' => if r' = r ∧ a' = a then none else m r' a'

/-- A global memory location: a (resource type, address) pair. -/
structure Location where
  rsrc : ResourceId
  addr : Address

/-- A set of global memory locations, e.g. the footprint of a `modifies`
clause.  Kept as a predicate; no decidability is required because footprints
only occur in specifications and havoc relations. -/
abbrev Footprint := Location → Prop

/-- `agreesOutside Δ m m'`: the transition from memory `m` to `m'` did not
touch any location outside the footprint `Δ` (the frame condition of a
`modifies` clause). -/
def agreesOutside (Δ : Footprint) (m m' : Memory) : Prop :=
  ∀ (r : ResourceId) (a : Address), ¬ Δ ⟨r, a⟩ → m' r a = m r a

/-- A full bytecode-level state: locals plus global memory. -/
structure MoveState where
  locals : Locals
  memory : Memory

namespace MoveState

/-- Update one local. -/
def writeLocal (s : MoveState) (x : LocalIndex) (v : Value) : MoveState :=
  { s with locals := fun y => if y = x then some v else s.locals y }

/-- Update several locals, pointwise (used for call returns). -/
def writeLocals : MoveState → List LocalIndex → List Value → MoveState
  | s, x :: xs, v :: vs => (s.writeLocal x v).writeLocals xs vs
  | s, _, _ => s

/-- Replace the global memory (used at call boundaries). -/
def setMemory (s : MoveState) (m : Memory) : MoveState :=
  { s with memory := m }

/-- Store a resource of type `r` at address `a`. -/
def writeGlobal (s : MoveState) (r : ResourceId) (a : Address) (v : Value) :
    MoveState :=
  { s with memory := memWrite s.memory r a v }

/-- Remove the resource of type `r` at address `a`. -/
def removeGlobal (s : MoveState) (r : ResourceId) (a : Address) : MoveState :=
  { s with memory := memRemove s.memory r a }

/-- Read the value a reference designates (`none` if the root or the path
does not exist — e.g. a dangling reference after `move_from`). -/
def readTarget (s : MoveState) (t : RefTarget) : Option Value :=
  match t.root with
  | .loc x => (s.locals x).bind (·.getPath t.path)
  | .global r a => (s.memory r a).bind (·.getPath t.path)

/-- Write through a reference: read-modify-write of the root location. -/
def writeTarget (s : MoveState) (t : RefTarget) (v : Value) :
    Option MoveState :=
  match t.root with
  | .loc x => (s.locals x).bind fun root =>
      (root.setPath t.path v).map fun root' => s.writeLocal x root'
  | .global r a => (s.memory r a).bind fun root =>
      (root.setPath t.path v).map fun root' => s.writeGlobal r a root'

/-- Write through several references, in order (the write-back of
checked-out call arguments, see `Semantics.lean`). -/
def writeTargets : MoveState → List RefTarget → List Value → Option MoveState
  | s, [], [] => some s
  | s, t :: ts, v :: vs => (s.writeTarget t v).bind (·.writeTargets ts vs)
  | _, _, _ => none

end MoveState

end Move.IR
