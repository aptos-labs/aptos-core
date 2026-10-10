-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Interpreter
import LeanerIR.Proofs.Denote.Types
import LeanerIR.Validation.Check

namespace LeanerIR.Tests.Closures

open LeanerIR
open LeanerIR.Import
open LeanerIR.Validation

private def config : ProfileConfig := { profile := .rust, name := "rust-test" }
private def schema : ProfileSchema := { profile := .rust, name := "rust-test" }
private def semantics : SemanticProfile := {
  profile := .rust, name := "rust-test", classify := fun _ _ => none }

private def typeUse (typeId loc : Nat) : TypeUse := { typeId := ⟨typeId⟩, loc := ⟨loc⟩ }

private def localDecl (id : Nat) (name : String) (loc : Nat) : LocalDecl := {
  id := ⟨id⟩, name, type := typeUse 0 loc, mutable := false, loc := ⟨loc⟩ }

private def parameter (_id : Nat) (name : String) (loc : Nat) : Parameter := {
  name, typeUse := typeUse 0 loc }

private def fixture : RawUnit where
  tables := {
    files := #[{ name := "closures.rs" }]
    locations := (Array.range 12).map fun index => {
      primary := some { file := ⟨0⟩, startByte := index, endByte := index + 1 } }
    origins := #[{ kind := .rustMir, location := ⟨0⟩ }]
    alignments := #[{ source := ⟨0⟩, trust := .checked, description := "closure fixture" }]
    types := #[.bool, .function #[⟨0⟩] ⟨0⟩]
    namespaces := #[{ segments := #["test", "Closures"] }]
    names := #[
      { namespaceId := ⟨0⟩, name := "main" },
      { namespaceId := ⟨0⟩, name := "first" }] }
  profiles := #[config]
  namespaces := #[{
    loc := ⟨0⟩
    identity := ⟨0⟩
    profile := some .rust
    expressions := #[
      { loc := ⟨0⟩, typeId := ⟨0⟩, kind := .value (.bool true) },
      { loc := ⟨1⟩, typeId := ⟨0⟩, kind := .value (.bool false) },
      { loc := ⟨2⟩, typeId := ⟨1⟩, kind := .operation
          (.call (.closure { namespaceId := ⟨0⟩, name := ⟨1⟩ } 1)) #[] #[⟨0⟩] },
      { loc := ⟨3⟩, typeId := ⟨0⟩, kind := .operation (.call .invoke) #[] #[⟨2⟩, ⟨1⟩] },
      { loc := ⟨4⟩, typeId := ⟨0⟩, kind := .localVar ⟨0⟩ }]
    functions := #[
      {
        loc := ⟨5⟩
        name := ⟨0⟩
        profile := .rust
        signature := { results := #[typeUse 0 5] }
        body := .structured ⟨3⟩
        origin := ⟨0⟩
        alignment := ⟨0⟩
      },
      {
        loc := ⟨6⟩
        name := ⟨1⟩
        profile := .rust
        signature := {
          parameters := #[parameter 0 "capture" 6, parameter 1 "argument" 6]
          results := #[typeUse 0 6] }
        body := .structured ⟨4⟩
        origin := ⟨0⟩
        alignment := ⟨0⟩
        locals := #[localDecl 0 "capture" 6, localDecl 1 "argument" 6]
      }] }]

private def executable? : Option ((unit : ValidatedUnit) × ExecutableUnit unit) := do
  let checked ← (validate #[schema] fixture).toOption
  (prepareExecution #[semantics] checked).toOption.map (⟨checked, ·⟩)

section Typing
open LeanerIR.Proofs.Denote

private def boolRow : Proofs.Denote.NRow := .cons .bool .nil

/-! A closure inhabits a function type when its target's rows at its frame,
split by its mask, are the type's, and its captures inhabit the captured row
(`NTy.admits`). -/

#guard match executable? with
  | some ⟨unit, _⟩ => closureRowsIn? unit ⟨⟨0⟩, ⟨1⟩⟩ 1 #[] == some (boolRow, boolRow, boolRow)
  | none => false

#guard match executable? with
  | some ⟨unit, _⟩ =>
      NTy.admits unit (.function boolRow [false] boolRow)
        (.closure ⟨⟨0⟩, ⟨1⟩⟩ 1 #[] #[.bool true])
  | none => false

-- Not at another parameter row, not with a parameter its target takes by
-- value marked shared, not with a capture of another type, and not under an
-- instantiation the runtime does not build for a target without type
-- parameters.
#guard match executable? with
  | some ⟨unit, _⟩ =>
      !NTy.admits unit (.function boolRow [false] boolRow)
        (.closure ⟨⟨0⟩, ⟨1⟩⟩ 1 #[(⟨0⟩, ⟨0⟩)] #[.bool true]) &&
      !NTy.admits unit (.function (.cons (.int 64 false) .nil) [false] boolRow)
        (.closure ⟨⟨0⟩, ⟨1⟩⟩ 1 #[] #[.bool true]) &&
      !NTy.admits unit (.function boolRow [true] boolRow)
        (.closure ⟨⟨0⟩, ⟨1⟩⟩ 1 #[] #[.bool true]) &&
      !NTy.admits unit (.function boolRow [false] boolRow)
        (.closure ⟨⟨0⟩, ⟨1⟩⟩ 1 #[] #[.integer 1])
  | none => false

end Typing

private def preparationHasDiagnosticAt (raw : RawUnit) (code : String) (loc : LocId) : Bool :=
  match validate #[schema] raw with
  | .error _ => false
  | .ok checked => match prepareExecution #[semantics] checked with
    | .error diagnostics => diagnostics.any fun diagnostic =>
        diagnostic.code == code && diagnostic.primary == some loc
    | .ok _ => false

private def validationHasDiagnosticAt (raw : RawUnit) (code : String) (loc : LocId) : Bool :=
  match validate #[schema] raw with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == code && diagnostic.primary == some loc
  | .ok _ => false

private def badCaptureTypeFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with
    tables := { fixture.tables with types := fixture.tables.types.push (.integer (.bits 8) false) }
    namespaces := #[{
      ns with
      expressions := ns.expressions.set! 0 {
        ns.expressions[0]! with typeId := ⟨2⟩, kind := .value (.integer 7) } }] }

#guard validationHasDiagnosticAt badCaptureTypeFixture "LIR-SEMANTIC-TYPE" ⟨2⟩

private def badInvokeArgumentFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with
    tables := { fixture.tables with types := fixture.tables.types.push (.integer (.bits 8) false) }
    namespaces := #[{
      ns with
      expressions := ns.expressions.set! 1 {
        ns.expressions[1]! with typeId := ⟨2⟩, kind := .value (.integer 7) } }] }

#guard validationHasDiagnosticAt badInvokeArgumentFixture "LIR-SEMANTIC-TYPE" ⟨3⟩

private def badClosureResultFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with namespaces := #[{
      ns with expressions := ns.expressions.set! 2 {
        ns.expressions[2]! with typeId := ⟨0⟩ } }] }

#guard validationHasDiagnosticAt badClosureResultFixture "LIR-SEMANTIC-TYPE" ⟨2⟩

#guard match executable? with
  | none => false
  | some ⟨_, executable⟩ => match LeanerIR.Interpreter.run executable 32
      { namespaceId := ⟨0⟩, functionId := ⟨0⟩ } #[] with
    | .ok (_, outcome) => outcome.value == .returned #[.bool true]
    | .error _ => false

/-! ## Masks

A mask selects the captured parameters by position, as Move's `ClosureMask`:
`compose` interleaves captures and supplied arguments and is the inverse of
`extract`. -/

#guard ClosureMask.compose 0b0101 [1, 3] [2, 4] == some [1, 2, 3, 4]
#guard ClosureMask.compose 0b0101 [1, 3] [2, 4, 5] == some [1, 2, 3, 4, 5]
#guard ClosureMask.compose 0b0101 [1] [2, 4] == none
#guard ClosureMask.compose 0b0001 [1, 3] [] == none
#guard ClosureMask.compose 0b0100 [3] [1] == none
#guard ClosureMask.compose 0 [] [7] == some [7]
#guard ClosureMask.extract 0b0101 true [1, 2, 3, 4] == [1, 3]
#guard ClosureMask.extract 0b0101 false [1, 2, 3, 4] == [2, 4]
#guard ClosureMask.leading 3 == 0b111
#guard (List.range 32).all fun mask =>
  let row := [10, 11, 12, 13, 14]
  ClosureMask.compose mask (ClosureMask.extract mask true row)
    (ClosureMask.extract mask false row) == some row

private def withClosure (mask : Nat) (captures : Array ExprId := #[⟨0⟩]) : RawUnit :=
  let ns := fixture.namespaces[0]!
  let closure : ExprKind :=
    .operation (.call (.closure { namespaceId := ⟨0⟩, name := ⟨1⟩ } mask)) #[] captures
  { fixture with namespaces := #[{
      ns with expressions := ns.expressions.set! 2 { ns.expressions[2]! with kind := closure } }] }

private def runMain (raw : RawUnit) : Option RuntimeValue := do
  let checked ← (validate #[schema] raw).toOption
  let executable ← (prepareExecution #[semantics] checked).toOption
  match LeanerIR.Interpreter.run executable 32 { namespaceId := ⟨0⟩, functionId := ⟨0⟩ } #[] with
  | .ok (_, { value := .returned #[value], .. }) => some value
  | _ => none

-- `first` returns its first parameter. Captured first, `true` is the result;
-- captured second, the supplied `false` is.
#guard runMain (withClosure 0b01) == some (.bool true)
#guard runMain (withClosure 0b10) == some (.bool false)

-- A mask selecting a parameter the target lacks, and one selecting more
-- parameters than there are captures, are rejected.
#guard validationHasDiagnosticAt (withClosure 0b100) "LIR-CLOSURE-MASK" ⟨2⟩
#guard validationHasDiagnosticAt (withClosure 0b11) "LIR-SEMANTIC-ARITY" ⟨2⟩

/-! ## Captures

A closure captures values: never a reference, and it claims only the
abilities every capture has. -/

private def referenceCaptureFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  let main := ns.functions[0]!
  { fixture with
    tables := { fixture.tables with
      lifetimes := #[{ kind := .local, loc := ⟨0⟩ }]
      types := fixture.tables.types.push
        (.reference { profile := .rust, kind := .shared, referent := ⟨0⟩, lifetime := ⟨0⟩ }) }
    namespaces := #[{
      ns with
      expressions := ns.expressions.set! 0 {
        ns.expressions[0]! with typeId := ⟨2⟩, kind := .localVar ⟨0⟩ }
      functions := ns.functions.set! 0 { main with
        signature := { main.signature with
          parameters := #[{ name := "held", typeUse := typeUse 2 5 }] }
        locals := #[{ localDecl 0 "held" 5 with type := typeUse 2 5 }] } }] }

#guard validationHasDiagnosticAt referenceCaptureFixture "LIR-CLOSURE-CAPTURE" ⟨2⟩

-- A closure capturing a function value without `copy` cannot claim `copy`:
-- `main` invokes a closure of `keep` that captures a closure of `first`.
private def abilityFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  let first := ns.functions[1]!
  let keep : QualifiedRef := { namespaceId := ⟨0⟩, name := ⟨2⟩ }
  let inner : ExprKind :=
    .operation (.call (.closure { namespaceId := ⟨0⟩, name := ⟨1⟩ } 1)) #[] #[⟨0⟩]
  { fixture with
    tables := { fixture.tables with
      types := fixture.tables.types.push (.function #[⟨0⟩] ⟨0⟩ #[.copy])
      names := fixture.tables.names.push { namespaceId := ⟨0⟩, name := "keep" } }
    namespaces := #[{
      ns with
      expressions := (ns.expressions.set! 2 { ns.expressions[2]! with
          typeId := ⟨2⟩, kind := .operation (.call (.closure keep 1)) #[] #[⟨5⟩] }) ++ #[
        { loc := ⟨7⟩, typeId := ⟨1⟩, kind := inner },
        { loc := ⟨8⟩, typeId := ⟨0⟩, kind := .localVar ⟨1⟩ }]
      functions := ns.functions.push { first with
        loc := ⟨9⟩
        name := ⟨2⟩
        signature := {
          parameters := #[{ name := "held", typeUse := typeUse 1 9 }, parameter 1 "argument" 9]
          results := #[typeUse 0 9] }
        body := .structured ⟨6⟩
        locals := #[{ localDecl 0 "held" 9 with type := typeUse 1 9 }, localDecl 1 "argument" 9] }
      }] }

#guard validationHasDiagnosticAt abilityFixture "LIR-CLOSURE-ABILITY" ⟨2⟩

/-! ## Order

Closures order by their target's qualified name, as Move compares function
values, not by handle: `first` precedes `main` though its handle is larger. -/

#guard match validate #[schema] fixture with
  | .ok checked =>
      let ranks := SemanticOperations.valueRanks checked.namespaces[0]!.orders
      let closure (index : Nat) : RuntimeValue := .closure ⟨⟨0⟩, ⟨index⟩⟩ 0 #[] #[]
      RuntimeValue.order ranks (closure 1) (closure 0) == .lt &&
        RuntimeValue.order ranks (closure 0) (closure 1) == .gt
  | .error _ => false

/-! ## Lending through an invocation

An invocation lends a mutable reference as a call does: `set_nine` writes
through it, and the lender observes the write. -/

private def lendingFixture : RawUnit where
  tables := {
    files := #[{ name := "lending.rs" }]
    locations := (Array.range 12).map fun index => {
      primary := some { file := ⟨0⟩, startByte := index, endByte := index + 1 } }
    origins := #[{ kind := .rustMir, location := ⟨0⟩ }]
    alignments := #[{ source := ⟨0⟩, trust := .checked, description := "lending fixture" }]
    lifetimes := #[{ kind := .local, loc := ⟨0⟩ }]
    types := #[
      .unit,
      .integer (.bits 64) true,
      .reference { profile := .rust, kind := .mutable, referent := ⟨1⟩, lifetime := ⟨0⟩ },
      .function #[⟨2⟩] ⟨0⟩]
    namespaces := #[{ segments := #["test", "Lending"] }]
    names := #[
      { namespaceId := ⟨0⟩, name := "main" },
      { namespaceId := ⟨0⟩, name := "set_nine" }] }
  profiles := #[config]
  namespaces := #[{
    loc := ⟨0⟩
    identity := ⟨0⟩
    profile := some .rust
    expressions := #[
      { loc := ⟨0⟩, typeId := ⟨1⟩, kind := .value (.integer 7) },
      { loc := ⟨1⟩, typeId := ⟨3⟩, kind := .operation
          (.call (.closure { namespaceId := ⟨0⟩, name := ⟨1⟩ } 0)) #[] #[] },
      { loc := ⟨2⟩, typeId := ⟨2⟩, kind := .operation (.borrow .mutable ⟨0⟩) #[] #[] },
      { loc := ⟨3⟩, typeId := ⟨0⟩, kind := .operation (.call .invoke) #[] #[⟨1⟩, ⟨2⟩] },
      { loc := ⟨4⟩, typeId := ⟨1⟩, kind := .operation (.move ⟨0⟩) #[] #[] },
      { loc := ⟨5⟩, typeId := ⟨1⟩, kind := .block #[⟨3⟩] (some ⟨4⟩) },
      { loc := ⟨6⟩, typeId := ⟨1⟩, kind := .letDecl ⟨0⟩ (some ⟨0⟩) ⟨5⟩ },
      { loc := ⟨7⟩, typeId := ⟨1⟩, kind := .value (.integer 9) },
      { loc := ⟨8⟩, typeId := ⟨0⟩, kind := .operation (.write ⟨2⟩) #[] #[⟨7⟩] }]
    patterns := #[{ loc := ⟨9⟩, typeId := ⟨1⟩, kind := .variable ⟨0⟩ }]
    places := #[.localVar ⟨0⟩, .localVar ⟨0⟩, .deref ⟨1⟩]
    functions := #[
      {
        loc := ⟨10⟩
        name := ⟨0⟩
        profile := .rust
        signature := { results := #[typeUse 1 10] }
        body := .structured ⟨6⟩
        origin := ⟨0⟩
        alignment := ⟨0⟩
        locals := #[
          { id := ⟨0⟩, name := "value", type := typeUse 1 10, mutable := true, loc := ⟨10⟩ }]
      },
      {
        loc := ⟨11⟩
        name := ⟨1⟩
        profile := .rust
        signature := { parameters := #[{ name := "target", typeUse := typeUse 2 11 }] }
        body := .structured ⟨8⟩
        origin := ⟨0⟩
        alignment := ⟨0⟩
        locals := #[{ id := ⟨0⟩, name := "target", type := typeUse 2 11, loc := ⟨11⟩ }]
      }] }]

#guard runMain lendingFixture == some (.integer 9)

private def prepared := (executable?.get (by native_decide)).2

private theorem successfulRunHasDerivation {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    (fuel : Nat) (function : FunctionHandle) (arguments : Array RuntimeValue)
    (success : (LeanerIR.Interpreter.run executable fuel function arguments).isOk) :
    ∃ (finalState : RuntimeState) (outcome : LocatedOutcome),
      BigStep.EvalFunction executable function #[] {} arguments finalState outcome.value := by
  generalize result_eq : LeanerIR.Interpreter.run executable fuel function arguments = result
  cases result with
  | error error => simp [result_eq, Except.isOk, Except.toBool] at success
  | ok result =>
      exact ⟨result.1, result.2,
        LeanerIR.Proofs.Interpreter.run_sound executable fuel function arguments {}
          result.1 result.2 result_eq⟩

example : ∃ (finalState : RuntimeState) (outcome : LocatedOutcome),
    BigStep.EvalFunction prepared { namespaceId := ⟨0⟩, functionId := ⟨0⟩ }
      #[] {} #[] finalState outcome.value := by
  apply successfulRunHasDerivation prepared 32
    { namespaceId := ⟨0⟩, functionId := ⟨0⟩ } #[]
  native_decide

end LeanerIR.Tests.Closures
