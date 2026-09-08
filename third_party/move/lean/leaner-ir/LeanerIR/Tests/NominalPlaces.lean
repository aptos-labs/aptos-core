-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Interpreter
import LeanerIR.Validation.Check

namespace LeanerIR.Tests.NominalPlaces

open LeanerIR
open LeanerIR.Import
open LeanerIR.Validation

private def config : ProfileConfig := { profile := .rust, name := "rust-test" }
private def schema : ProfileSchema := { profile := .rust, name := "rust-test" }
private def semantics : SemanticProfile := {
  profile := .rust, name := "rust-test", classify := fun _ _ => none }

private def typeUse (typeId loc : Nat) : TypeUse := { typeId := ⟨typeId⟩, loc := ⟨loc⟩ }

private def pair : QualifiedRef := { namespaceId := ⟨0⟩, name := ⟨1⟩ }
private def choice : QualifiedRef := { namespaceId := ⟨0⟩, name := ⟨5⟩ }

private def boolField (name : Nat) : FieldDecl := {
  loc := ⟨0⟩, name := ⟨name⟩, type := typeUse 1 0 }

/-! The first function mutates and reads a structure field. The second reads
an enum payload through an explicit downcast. Both paths exercise the same
nominal projections used by concrete references. -/
private def fixture : RawUnit where
  tables := {
    files := #[{ name := "nominal_places.rs" }]
    locations := (Array.range 24).map fun index => {
      primary := some { file := ⟨0⟩, startByte := index, endByte := index + 1 } }
    origins := #[{ kind := .rustMir, location := ⟨0⟩ }]
    alignments := #[{
      source := ⟨0⟩, trust := .checked, description := "nominal place fixture" }]
    types := #[.unit, .bool, .nominal ⟨1⟩ #[], .nominal ⟨5⟩ #[]]
    namespaces := #[{ segments := #["test", "NominalPlaces"] }]
    names := #[
      { namespaceId := ⟨0⟩, name := "mutate_pair" },
      { namespaceId := ⟨0⟩, name := "Pair" },
      { namespaceId := ⟨0⟩, name := "first" },
      { namespaceId := ⟨0⟩, name := "second" },
      { namespaceId := ⟨0⟩, name := "read_choice" },
      { namespaceId := ⟨0⟩, name := "Choice" },
      { namespaceId := ⟨0⟩, name := "Left" },
      { namespaceId := ⟨0⟩, name := "Right" },
      { namespaceId := ⟨0⟩, name := "value" },
      { namespaceId := ⟨0⟩, name := "missing" }] }
  profiles := #[config]
  namespaces := #[{
    loc := ⟨0⟩
    identity := ⟨0⟩
    profile := some .rust
    expressions := #[
      { loc := ⟨0⟩, typeId := ⟨1⟩, kind := .value (.bool true) },
      { loc := ⟨1⟩, typeId := ⟨1⟩, kind := .value (.bool false) },
      { loc := ⟨2⟩, typeId := ⟨2⟩,
        kind := .operation (.call (.constructor pair)) #[] #[⟨0⟩, ⟨1⟩] },
      { loc := ⟨3⟩, typeId := ⟨0⟩,
        kind := .operation (.write ⟨1⟩) #[] #[⟨0⟩] },
      { loc := ⟨4⟩, typeId := ⟨1⟩,
        kind := .operation (.read ⟨1⟩) #[] #[] },
      { loc := ⟨5⟩, typeId := ⟨1⟩, kind := .block #[⟨3⟩] (some ⟨4⟩) },
      { loc := ⟨6⟩, typeId := ⟨1⟩, kind := .letDecl ⟨0⟩ (some ⟨2⟩) ⟨5⟩ },
      { loc := ⟨7⟩, typeId := ⟨1⟩, kind := .value (.bool false) },
      { loc := ⟨8⟩, typeId := ⟨3⟩,
        kind := .operation (.call (.constructor choice (some "Right"))) #[] #[⟨7⟩] },
      { loc := ⟨9⟩, typeId := ⟨1⟩,
        kind := .operation (.read ⟨4⟩) #[] #[] },
      { loc := ⟨10⟩, typeId := ⟨1⟩, kind := .letDecl ⟨1⟩ (some ⟨8⟩) ⟨9⟩ }]
    patterns := #[
      { loc := ⟨11⟩, typeId := ⟨2⟩, kind := .variable ⟨0⟩ },
      { loc := ⟨12⟩, typeId := ⟨3⟩, kind := .variable ⟨0⟩ }]
    places := #[
      .localVar ⟨0⟩,
      .field ⟨0⟩ { namespaceId := ⟨0⟩, name := ⟨1⟩ } ⟨3⟩,
      .localVar ⟨0⟩,
      .downcast ⟨2⟩ ⟨7⟩,
      .field ⟨3⟩ { namespaceId := ⟨0⟩, name := ⟨5⟩ } ⟨8⟩]
    structs := #[
      { loc := ⟨13⟩, name := ⟨1⟩, fields := #[boolField 2, boolField 3] },
      { loc := ⟨14⟩, name := ⟨5⟩, variants := #[
          { loc := ⟨14⟩, name := ⟨6⟩, fields := #[boolField 8] },
          { loc := ⟨14⟩, name := ⟨7⟩, fields := #[boolField 8] }] }]
    functions := #[
      {
        loc := ⟨15⟩
        name := ⟨0⟩
        profile := .rust
        signature := { results := #[typeUse 1 15] }
        body := .structured ⟨6⟩
        origin := ⟨0⟩
        alignment := ⟨0⟩
        locals := #[{
          id := ⟨0⟩, name := "pair", type := typeUse 2 15,
          mutable := true, loc := ⟨15⟩ }]
      },
      {
        loc := ⟨16⟩
        name := ⟨4⟩
        profile := .rust
        signature := { results := #[typeUse 1 16] }
        body := .structured ⟨10⟩
        origin := ⟨0⟩
        alignment := ⟨0⟩
        locals := #[{
          id := ⟨0⟩, name := "choice", type := typeUse 3 16,
          loc := ⟨16⟩ }]
      }] }]

private def executable? : Option ExecutableUnit := do
  let checked ← (validate #[schema] fixture).toOption
  (prepareExecution #[semantics] checked).toOption

private def handle (functionId : Nat) : FunctionHandle := {
  namespaceId := ⟨0⟩, functionId := ⟨functionId⟩ }

#guard match executable? with
  | some executable => match Interpreter.run executable 32 (handle 0) #[] with
      | .ok (_, { value := .returned #[.bool true], .. }) => true
      | _ => false
  | none => false

#guard match executable? with
  | some executable => match Interpreter.run executable 32 (handle 1) #[] with
      | .ok (_, { value := .returned #[.bool false], .. }) => true
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

private def badFieldResultFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with namespaces := #[{
      ns with expressions := ns.expressions.set! 4 {
        ns.expressions[4]! with typeId := ⟨0⟩ } }] }

#guard validationHasDiagnosticAt badFieldResultFixture "LIR-SEMANTIC-TYPE" ⟨4⟩

private def missingFieldFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with namespaces := #[{
      ns with places := ns.places.set! 1 (Place.field ⟨0⟩ ⟨⟨0⟩, ⟨1⟩⟩ ⟨9⟩) }] }

#guard validationHasDiagnosticAt missingFieldFixture "LIR-SEMANTIC-TARGET" ⟨3⟩

private def missingVariantFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with namespaces := #[{
      ns with places := ns.places.set! 3 (.downcast ⟨2⟩ ⟨9⟩) }] }

#guard validationHasDiagnosticAt missingVariantFixture "LIR-SEMANTIC-TARGET" ⟨9⟩

private def prepared : ExecutableUnit := executable?.get (by native_decide)

private theorem successfulRunHasDerivation (function : FunctionHandle)
    (success : (Interpreter.run prepared 32 function #[]).isOk) :
    ∃ (finalState : RuntimeState) (outcome : LocatedOutcome),
      BigStep.EvalFunction prepared function {} #[] finalState outcome.value := by
  generalize result_eq : Interpreter.run prepared 32 function #[] = result
  cases result with
  | error error => simp [result_eq, Except.isOk, Except.toBool] at success
  | ok result =>
      exact ⟨result.1, result.2,
        LeanerIR.Proofs.Interpreter.run_sound prepared 32 function #[] {}
          result.1 result.2 result_eq⟩

example : ∃ (finalState : RuntimeState) (outcome : LocatedOutcome),
    BigStep.EvalFunction prepared (handle 0) {} #[] finalState outcome.value := by
  apply successfulRunHasDerivation (handle 0)
  native_decide

example : ∃ (finalState : RuntimeState) (outcome : LocatedOutcome),
    BigStep.EvalFunction prepared (handle 1) {} #[] finalState outcome.value := by
  apply successfulRunHasDerivation (handle 1)
  native_decide

end LeanerIR.Tests.NominalPlaces
