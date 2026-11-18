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
  /-- The application descriptor, closed data: the unit's function values, the
  declaration's parameter row and the instance type the theorem is stated at.
  The theorem's telescope is `∀ (frame) (inhabitable) (value), NTy.Holds …`,
  and these three spell every type in it, so a consumer applies the theorem
  without reading its statement back. Checked against the theorem when the
  entry is registered (`Entry.describesTheorem?`). -/
  functions : Name
  params : Lean.Expr
  instanceType : Lean.Expr
  /-- The declaration and the row, spelled for a listing. -/
  declaration : String
  rowSpelling : String
  clauses : Nat
  deriving Inhabited

open Lean Meta in
/-- The frame the theorem takes: a frame of the unit at its function values. -/
def Entry.frameType (entry : Entry) : Lean.Expr :=
  mkApp2 (mkConst ``Skolems) (mkConst entry.unit) (mkConst entry.functions)

open Lean Meta in
/-- The carriers a frame induces, as the theorem spells them. -/
def Entry.carriersAt (entry : Entry) (frame : Lean.Expr) : Lean.Expr :=
  mkApp3 (mkConst ``Skolems.toCarriers) (mkConst entry.unit) (mkConst entry.functions) frame

open Lean Meta in
/-- The resolutions of the declaration's parameters at a frame, as the
theorem's premise maps them. -/
def Entry.resolutionsAt (entry : Entry) (frame : Lean.Expr) : Lean.Expr :=
  mkApp2 (mkConst ``NRow.map)
    (mkApp3 (mkConst ``Skolems.resolve) (mkConst entry.unit) (mkConst entry.functions) frame)
    entry.params

open Lean Meta in
/-- The inhabitability the theorem takes of those resolutions. -/
def Entry.premiseAt (entry : Entry) (frame : Lean.Expr) : MetaM Lean.Expr :=
  mkEq (mkApp (mkConst ``NRow.inhabitable) (entry.resolutionsAt frame)) (mkConst ``Bool.true)

open Lean Meta in
/-- The value the theorem takes, at the frame's carriers. -/
def Entry.valueType (entry : Entry) (frame : Lean.Expr) : Lean.Expr :=
  mkApp2 (mkConst ``NTy.carrier)
    (mkApp (mkConst ``Carriers.toSorts) (entry.carriersAt frame)) entry.instanceType

open Lean Meta in
/-- The fact the theorem states of such a value: the left side of its
equivalence. -/
def Entry.factAt (entry : Entry) (frame value : Lean.Expr) : Lean.Expr :=
  mkApp3 (mkConst ``NTy.Holds) (entry.carriersAt frame) entry.instanceType value

/-- The declaration's parameters, entry by entry: the row the descriptor
carries must be a literal, so that a consumer can state the resolutions one by
one. -/
partial def rowEntries? (row : Lean.Expr) : Option (Array Lean.Expr) :=
  if row.isConstOf ``NRow.nil then some #[]
  else if row.isAppOfArity ``NRow.cons 2 then
    (rowEntries? (row.getArg! 1)).map (#[row.getArg! 0] ++ ·)
  else none

open Lean Meta in
/-- Whether the descriptor describes the theorem it names: the application the
descriptor builds at fresh binders is type-correct, and its type is the
equivalence whose left side the descriptor states. The reason it does not,
which is a producer error at registration and never a decline at a leaf. -/
def Entry.describesTheorem? (entry : Entry) : MetaM (Option String) := do
  let some info := (← getEnv).find? entry.theoremName
    | return some s!"the theorem {entry.theoremName} is not declared"
  unless info.levelParams.isEmpty do
    return some s!"the theorem {entry.theoremName} takes universe parameters"
  unless (rowEntries? entry.params).isSome do
    return some s!"the descriptor's parameter row is no literal: {entry.params}"
  withLocalDeclD `frame entry.frameType fun frame => do
    let premise ← entry.premiseAt frame
    withLocalDeclD `inhabitable premise fun inhabitable =>
    withLocalDeclD `value (entry.valueType frame) fun value => do
      let applied := mkApp3 (mkConst entry.theoremName) frame inhabitable value
      let some _ ← observing? (Meta.check applied)
        | return some s!"the descriptor does not apply {entry.theoremName} at the binders it \
            spells"
      let type ← whnfR (← inferType applied)
      let_expr Iff held _ := type
        | return some s!"the theorem {entry.theoremName} states no equivalence: {type}"
      unless held == entry.factAt frame value do
        return some s!"the theorem {entry.theoremName} states {held}, not the fact the \
          descriptor claims"
      return none

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
  Id.run do
    let state := registry.getState env
    let atDeclaration (key : Key) : Bool := key.unit == unit && key.source == source
    for (key, entry) in state.bridges do
      if atDeclaration key then return some entry.declaration
    for (key, gap) in state.gaps do
      if atDeclaration key then return some gap.declaration
    return none

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
