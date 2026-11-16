-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean
import VerexIR.Proofs.Denote.OwnBridge.Bridge

/-!
# The bridges a unit's declarations have, and those they do not

A declaration's own invariant has a bridge per instance row
(`VerexIR.Proofs.Denote.OwnBridge.Bridge`): a theorem stating a value's fact at
a caller's frame as the declaration's clauses read at that frame and the facts
of its fields. The generator records each theorem here under the unit, the
declaration and the row, as the value's type spells them, and the solver reads
it by that key. A declaration at a row the generator could build no bridge for
is recorded with the clause and the operator or obligation outstanding, so
that the rows without a bridge can be listed.
-/

namespace VerexIR.Proofs.Denote.OwnBridge

open Lean

/-- A bridge: the unit's constant, the declaration's handle and the instance
row as a value's type states them, and the theorem. -/
structure Entry where
  unit : Name
  source : Lean.Expr
  row : Lean.Expr
  theoremName : Name
  /-- The declaration and the row, spelled for a listing. -/
  declaration : String
  rowSpelling : String
  clauses : Nat
  deriving Inhabited

/-- A declaration at a row without a bridge, and why. -/
structure Gap where
  unit : Name
  source : Lean.Expr
  row : Lean.Expr
  declaration : String
  rowSpelling : String
  /-- The clause the construction stopped in, by its source range. -/
  clause : String
  /-- The operator without a law, or the obligation outstanding. -/
  reason : String
  deriving Inhabited

/-- What a bridge is recorded under: the unit, the declaration's handle and
the row. -/
structure Key where
  unit : Name
  source : Lean.Expr
  row : Lean.Expr
  deriving BEq

instance : Hashable Key where
  hash key := mixHash (hash key.unit) (mixHash key.source.hash key.row.hash)

structure State where
  bridges : Std.HashMap Key Entry := {}
  gaps : Std.HashMap Key Gap := {}

instance : Inhabited State := ⟨{}⟩

inductive Record where
  | bridge (entry : Entry)
  | gap (gap : Gap)
  deriving Inhabited

private def State.add (state : State) : Record → State
  | .bridge entry =>
      { state with bridges := state.bridges.insert ⟨entry.unit, entry.source, entry.row⟩ entry }
  | .gap gap => { state with gaps := state.gaps.insert ⟨gap.unit, gap.source, gap.row⟩ gap }

initialize registry : SimplePersistentEnvExtension Record State ←
  registerSimplePersistentEnvExtension {
    addEntryFn := State.add
    addImportedFn := fun modules => modules.foldl (fun state records => records.foldl State.add state) {} }

/-- The bridge of a declaration of a unit at a row, where one is recorded. -/
def bridge? (env : Environment) (unit : Name) (source row : Lean.Expr) : Option Entry :=
  (registry.getState env).bridges[(⟨unit, source, row⟩ : Key)]?

/-- The recorded absence of a bridge of a declaration of a unit at a row. -/
def gap? (env : Environment) (unit : Name) (source row : Lean.Expr) : Option Gap :=
  (registry.getState env).gaps[(⟨unit, source, row⟩ : Key)]?

/-- The spelled name of a declaration of a unit that has a record, a bridge or
the absence of one, at some row: a scan of the records, there being no key of
a declaration alone.  For a listing of a declaration whose record is at
another row than the one asked for. -/
def declarationRecorded? (env : Environment) (unit : Name) (source : Lean.Expr) : Option String :=
  let state := registry.getState env
  let atDeclaration (key : Key) : Bool := key.unit == unit && key.source == source
  let bridged := state.bridges.fold (init := none) fun found key entry =>
    found <|> (if atDeclaration key then some entry.declaration else none)
  let absent := state.gaps.fold (init := none) fun found key gap =>
    found <|> (if atDeclaration key then some gap.declaration else none)
  bridged <|> absent

/-- Every bridge recorded, and every recorded absence. -/
def recorded (env : Environment) : Array Entry × Array Gap :=
  let state := registry.getState env
  (state.bridges.toArray.map (·.2), state.gaps.toArray.map (·.2))

def Entry.describe (entry : Entry) : String :=
  s!"own bridge: {entry.unit} {entry.declaration} {entry.rowSpelling}: \
    bridged, {entry.clauses} clause(s), {entry.theoremName}"

def Gap.describe (gap : Gap) : String :=
  s!"own bridge: {gap.unit} {gap.declaration} {gap.rowSpelling}: none; clause {gap.clause}: \
    {gap.reason}"

/-- List the bridges recorded in the environment and the declarations at rows
that have none, each with its reason. -/
elab "#verex_own_bridges" : command => do
  let (entries, gaps) := recorded (← getEnv)
  let lines := (entries.map Entry.describe ++ gaps.map Gap.describe).qsort (· < ·)
  logInfo ("\n".intercalate lines.toList)

end VerexIR.Proofs.Denote.OwnBridge
