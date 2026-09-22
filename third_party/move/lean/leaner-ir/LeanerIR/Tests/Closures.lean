-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Interpreter
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
          (.call (.closure { namespaceId := ⟨0⟩, name := ⟨1⟩ })) #[] #[⟨0⟩] },
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

private def executable? : Option ExecutableUnit := do
  let checked ← (validate #[schema] fixture).toOption
  (prepareExecution #[semantics] checked).toOption

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
  | some executable => match LeanerIR.Interpreter.run executable 32
      { namespaceId := ⟨0⟩, functionId := ⟨0⟩ } #[] with
    | .ok (_, outcome) => outcome.value == .returned #[.bool true]
    | .error _ => false

private def prepared : ExecutableUnit := executable?.get (by native_decide)

private theorem successfulRunHasDerivation (executable : ExecutableUnit)
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
