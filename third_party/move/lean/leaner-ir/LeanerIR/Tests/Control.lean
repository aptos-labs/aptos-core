-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Interpreter
import LeanerIR.Validation.Check

namespace LeanerIR.Tests.Control

open LeanerIR
open LeanerIR.Import
open LeanerIR.Validation

private def config : ProfileConfig := { profile := .rust, name := "rust-test" }
private def schema : ProfileSchema := { profile := .rust, name := "rust-test" }
private def semantics : SemanticProfile := {
  profile := .rust, name := "rust-test", classify := fun _ _ => none }

private def typeUse (typeId loc : Nat) : TypeUse := { typeId := ⟨typeId⟩, loc := ⟨loc⟩ }

/-! The inner loop propagates `break 1 true` to the outer loop, whose result is
Boolean. Additional arena roots support the negative preparation fixtures. -/
private def fixture : RawUnit where
  tables := {
    files := #[{ name := "control.lir" }]
    locations := (Array.range 16).map fun index => {
      primary := some { file := ⟨0⟩, startByte := index, endByte := index + 1 } }
    origins := #[{ kind := .leanerSource, location := ⟨0⟩ }]
    alignments := #[{
      source := ⟨0⟩, trust := .checked, description := "control fixture" }]
    types := #[.unit, .bool, .never]
    namespaces := #[{ segments := #["test", "Control"] }]
    names := #[{ namespaceId := ⟨0⟩, name := "nestedBreak" }] }
  profiles := #[config]
  namespaces := #[{
    loc := ⟨0⟩
    identity := ⟨0⟩
    profile := some .rust
    expressions := #[
      { loc := ⟨0⟩, typeId := ⟨1⟩, kind := .value (.bool true) },
      { loc := ⟨1⟩, typeId := ⟨2⟩, kind := .break_ 1 (some ⟨0⟩) },
      { loc := ⟨2⟩, typeId := ⟨0⟩, kind := .loop none ⟨1⟩ },
      { loc := ⟨3⟩, typeId := ⟨1⟩, kind := .loop none ⟨2⟩ },
      { loc := ⟨4⟩, typeId := ⟨2⟩, kind := .break_ 0 none },
      { loc := ⟨5⟩, typeId := ⟨2⟩, kind := .continue_ 0 },
      { loc := ⟨6⟩, typeId := ⟨2⟩, kind := .break_ 0 (some ⟨0⟩) },
      { loc := ⟨7⟩, typeId := ⟨0⟩, kind := .loop none ⟨6⟩ }]
    functions := #[{
      loc := ⟨8⟩
      name := ⟨0⟩
      profile := .rust
      signature := { results := #[typeUse 1 8] }
      body := .structured ⟨3⟩
      origin := ⟨0⟩
      alignment := ⟨0⟩ }] }]

private def prepare? (raw : RawUnit) : Option ExecutableUnit := do
  let checked ← (validate #[schema] raw).toOption
  (prepareExecution #[semantics] checked).toOption

private def executable? : Option ExecutableUnit := prepare? fixture
private def handle : FunctionHandle := { namespaceId := ⟨0⟩, functionId := ⟨0⟩ }

#guard match executable? with
  | some executable => match Interpreter.run executable 16 handle #[] with
      | .ok (_, { value := .returned #[.bool true], .. }) => true
      | _ => false
  | none => false

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

private def withRoot (root : Nat) (resultType : Nat := 1) : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with namespaces := #[{
      ns with functions := #[{
        ns.functions[0]! with
        signature := { results := #[typeUse resultType 8] }
        body := .structured ⟨root⟩ }] }] }

#guard validationHasDiagnosticAt (withRoot 4) "LIR-SEMANTIC-CONTROL" ⟨4⟩
#guard validationHasDiagnosticAt (withRoot 5) "LIR-SEMANTIC-CONTROL" ⟨5⟩
#guard validationHasDiagnosticAt (withRoot 7 0) "LIR-SEMANTIC-TYPE" ⟨6⟩
#guard validationHasDiagnosticAt (withRoot 3 0) "LIR-SEMANTIC-TYPE" ⟨3⟩

private def withAdditionalRoot (expression : Expr) (pattern : Option Pattern := none) : RawUnit :=
  let ns := fixture.namespaces[0]!
  let root : ExprId := ⟨ns.expressions.size⟩
  { fixture with namespaces := #[{
      ns with
      expressions := ns.expressions.push expression
      patterns := match pattern with
        | some pattern => ns.patterns.push pattern
        | none => ns.patterns
      functions := #[{ ns.functions[0]! with body := .structured root }] }] }

private def nonUnitEmptyBlockFixture : RawUnit :=
  withAdditionalRoot { loc := ⟨9⟩, typeId := ⟨1⟩, kind := .block #[] none }

private def neverEmptyBlockFixture : RawUnit :=
  let raw := withAdditionalRoot { loc := ⟨9⟩, typeId := ⟨2⟩, kind := .block #[] none }
  let ns := raw.namespaces[0]!
  { raw with namespaces := #[{ ns with functions := #[{
      ns.functions[0]! with signature := { results := #[typeUse 2 8] } }] }] }

private def neverReturningBlockFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  let returnId : ExprId := ⟨ns.expressions.size⟩
  let root : ExprId := ⟨ns.expressions.size + 1⟩
  { fixture with namespaces := #[{
      ns with
      expressions := ns.expressions
        |>.push { loc := ⟨9⟩, typeId := ⟨2⟩, kind := .return_ #[] }
        |>.push { loc := ⟨10⟩, typeId := ⟨2⟩, kind := .block #[returnId] none }
      functions := #[{ ns.functions[0]! with
        signature := { results := #[] }
        body := .structured root }] }] }

private def nonUnitIfWithoutElseFixture : RawUnit :=
  withAdditionalRoot { loc := ⟨10⟩, typeId := ⟨1⟩, kind := .ifElse ⟨0⟩ ⟨0⟩ none }

private def nonUnitPatternAssignmentFixture : RawUnit :=
  withAdditionalRoot
    { loc := ⟨11⟩, typeId := ⟨1⟩, kind := .assignPattern ⟨0⟩ ⟨0⟩ }
    (some { loc := ⟨11⟩, typeId := ⟨1⟩, kind := .wildcard })

private def nonUnitLoopBodyFixture : RawUnit :=
  withAdditionalRoot { loc := ⟨12⟩, typeId := ⟨0⟩, kind := .loop none ⟨0⟩ }

#guard validationHasDiagnosticAt nonUnitEmptyBlockFixture "LIR-SEMANTIC-TYPE" ⟨9⟩
#guard validationHasDiagnosticAt neverEmptyBlockFixture "LIR-SEMANTIC-TYPE" ⟨9⟩
#guard (prepare? neverReturningBlockFixture).isSome
#guard validationHasDiagnosticAt nonUnitIfWithoutElseFixture "LIR-SEMANTIC-TYPE" ⟨10⟩
#guard validationHasDiagnosticAt nonUnitPatternAssignmentFixture "LIR-SEMANTIC-TYPE" ⟨11⟩
#guard validationHasDiagnosticAt nonUnitLoopBodyFixture "LIR-SEMANTIC-TYPE" ⟨12⟩

private def prepared : ExecutableUnit := executable?.get (by native_decide)

private theorem successfulRunHasDerivation (success :
    (Interpreter.run prepared 16 handle #[]).isOk) :
    ∃ (finalState : RuntimeState) (outcome : LocatedOutcome),
      BigStep.EvalFunction prepared handle {} #[] finalState outcome.value := by
  generalize result_eq : Interpreter.run prepared 16 handle #[] = result
  cases result with
  | error error => simp [result_eq, Except.isOk, Except.toBool] at success
  | ok result =>
      exact ⟨result.1, result.2,
        LeanerIR.Proofs.Interpreter.run_sound prepared 16 handle #[] {}
          result.1 result.2 result_eq⟩

example : ∃ (finalState : RuntimeState) (outcome : LocatedOutcome),
    BigStep.EvalFunction prepared handle {} #[] finalState outcome.value := by
  apply successfulRunHasDerivation
  native_decide

end LeanerIR.Tests.Control
