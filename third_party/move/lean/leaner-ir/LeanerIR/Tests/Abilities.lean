-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Interpreter
import LeanerIR.Validation.Check

namespace LeanerIR.Tests.Abilities

open LeanerIR
open LeanerIR.Import
open LeanerIR.Validation

private def rustConfig : ProfileConfig := { profile := .rust, name := "rust-test" }
private def rustSchema : ProfileSchema := { profile := .rust, name := "rust-test" }
private def rustSemantics : SemanticProfile := {
  profile := .rust, name := "rust-test", classify := fun _ _ => none }

private def moveConfig : ProfileConfig := { profile := .move, name := "move-test" }
private def moveSchema : ProfileSchema := { profile := .move, name := "move-test" }
private def moveSemantics : SemanticProfile := {
  profile := .move, name := "move-test", classify := fun _ _ => none }

private def typeUse (typeId loc : Nat) : TypeUse := { typeId := ⟨typeId⟩, loc := ⟨loc⟩ }

private def blob : QualifiedRef := { namespaceId := ⟨0⟩, name := ⟨1⟩ }

/-! An ability-free nominal value is droppable under the Rust profile but not
under Move. Copy remains explicit under both profiles. -/
private def fixture : RawUnit where
  tables := {
    files := #[{ name := "abilities.rs" }]
    locations := (Array.range 16).map fun index => {
      primary := some { file := ⟨0⟩, startByte := index, endByte := index + 1 } }
    origins := #[{ kind := .rustMir, location := ⟨0⟩ }]
    alignments := #[{
      source := ⟨0⟩, trust := .checked, description := "ability fixture" }]
    types := #[.unit, .bool, .nominal ⟨1⟩ #[]]
    namespaces := #[{ segments := #["test", "Abilities"] }]
    names := #[
      { namespaceId := ⟨0⟩, name := "drop_blob" },
      { namespaceId := ⟨0⟩, name := "Blob" },
      { namespaceId := ⟨0⟩, name := "value" }] }
  profiles := #[rustConfig]
  namespaces := #[{
    loc := ⟨0⟩
    identity := ⟨0⟩
    profile := some .rust
    expressions := #[
      { loc := ⟨0⟩, typeId := ⟨1⟩, kind := .value (.bool true) },
      { loc := ⟨1⟩, typeId := ⟨2⟩,
        kind := .operation (.call (.constructor blob)) #[] #[⟨0⟩] },
      { loc := ⟨2⟩, typeId := ⟨0⟩,
        kind := .operation (.drop ⟨0⟩) #[] #[] },
      { loc := ⟨3⟩, typeId := ⟨0⟩,
        kind := .letDecl ⟨0⟩ (some ⟨1⟩) ⟨2⟩ }]
    patterns := #[{
      loc := ⟨4⟩, typeId := ⟨2⟩, kind := .variable ⟨0⟩ }]
    places := #[.localVar ⟨0⟩]
    structs := #[{
      loc := ⟨5⟩
      name := ⟨1⟩
      fields := #[{ loc := ⟨5⟩, name := ⟨2⟩, type := typeUse 1 5 }] }]
    functions := #[{
      loc := ⟨6⟩
      name := ⟨0⟩
      profile := .rust
      signature := { results := #[typeUse 0 6] }
      body := .structured ⟨3⟩
      origin := ⟨0⟩
      alignment := ⟨0⟩
      locals := #[{
        id := ⟨0⟩, name := "blob", type := typeUse 2 6, loc := ⟨6⟩ }]
      }] }]

private def prepare? (schema : ProfileSchema) (semantics : SemanticProfile)
    (raw : RawUnit) : Option ExecutableUnit := do
  let checked ← (validate #[schema] raw).toOption
  (prepareExecution #[semantics] checked).toOption

private def executable? : Option ExecutableUnit :=
  prepare? rustSchema rustSemantics fixture

private def handle : FunctionHandle := { namespaceId := ⟨0⟩, functionId := ⟨0⟩ }

#guard match executable? with
  | some executable => match Interpreter.run executable 16 handle #[] with
      | .ok (_, { value := .returned #[.unit], .. }) => true
      | _ => false
  | none => false

private def preparationHasDiagnosticAt (schema : ProfileSchema)
    (semantics : SemanticProfile) (raw : RawUnit) (code : String) (loc : LocId) : Bool :=
  match validate #[schema] raw with
  | .error _ => false
  | .ok checked => match prepareExecution #[semantics] checked with
      | .error diagnostics => diagnostics.any fun diagnostic =>
          diagnostic.code == code && diagnostic.primary == some loc
      | .ok _ => false

private def validationHasDiagnosticAt (schema : ProfileSchema)
    (_semantics : SemanticProfile) (raw : RawUnit) (code : String) (loc : LocId) : Bool :=
  match validate #[schema] raw with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == code && diagnostic.primary == some loc
  | .ok _ => false

private def moveDropFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with
    profiles := #[moveConfig]
    namespaces := #[{
      ns with
      profile := some .move
      functions := #[{ ns.functions[0]! with profile := .move }] }] }

#guard preparationHasDiagnosticAt moveSchema moveSemantics moveDropFixture
  "LIR-SEMANTIC-ABILITY" ⟨2⟩

private def copyFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with namespaces := #[{
      ns with
      expressions := ns.expressions.set! 2 {
        ns.expressions[2]! with typeId := ⟨2⟩, kind := .operation (.copy ⟨0⟩) #[] #[] }
        |>.set! 3 { ns.expressions[3]! with typeId := ⟨2⟩ }
      functions := #[{
        ns.functions[0]! with signature := { results := #[typeUse 2 6] } }] }] }

#guard preparationHasDiagnosticAt rustSchema rustSemantics copyFixture
  "LIR-SEMANTIC-ABILITY" ⟨2⟩

private def localReadFixture : RawUnit :=
  let ns := copyFixture.namespaces[0]!
  { copyFixture with namespaces := #[{
      ns with expressions := ns.expressions.set! 2 {
        ns.expressions[2]! with kind := .localVar ⟨0⟩ } }] }

#guard preparationHasDiagnosticAt rustSchema rustSemantics localReadFixture
  "LIR-SEMANTIC-ABILITY" ⟨2⟩

private def placeReadFixture : RawUnit :=
  let ns := copyFixture.namespaces[0]!
  { copyFixture with namespaces := #[{
      ns with expressions := ns.expressions.set! 2 {
        ns.expressions[2]! with kind := .operation (.read ⟨0⟩) #[] #[] } }] }

#guard preparationHasDiagnosticAt rustSchema rustSemantics placeReadFixture
  "LIR-SEMANTIC-ABILITY" ⟨2⟩

private def moveValueFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with namespaces := #[{
      ns with
      expressions := ns.expressions.set! 2 {
        loc := ⟨2⟩
        typeId := ⟨2⟩
        kind := .operation (.primitive .moveValue) #[] #[⟨1⟩] }
      functions := #[{
        ns.functions[0]! with
        signature := { results := #[typeUse 2 6] }
        body := .structured ⟨2⟩
        locals := #[] }] }] }

private def moveValueExecutable? : Option ExecutableUnit :=
  prepare? rustSchema rustSemantics moveValueFixture

#guard match moveValueExecutable? with
  | some executable => match Interpreter.run executable 16 handle #[] with
      | .ok (_, { value := .returned #[.nominal _ none #[.bool true]], .. }) => true
      | _ => false
  | none => false

private def copyableFixture : RawUnit :=
  let ns := copyFixture.namespaces[0]!
  { copyFixture with namespaces := #[{
      ns with structs := #[{ ns.structs[0]! with abilities := #[.copy] }] }] }

private def copyableExecutable? : Option ExecutableUnit :=
  prepare? rustSchema rustSemantics copyableFixture

#guard match copyableExecutable? with
  | some executable => match Interpreter.run executable 16 handle #[] with
      | .ok (_, { value := .returned #[.nominal _ none #[.bool true]], .. }) => true
      | _ => false
  | none => false

private def invalidRustCopyDeclarationFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  let referenceType : Ty := .reference {
    profile := .rust, kind := .mutable, referent := ⟨1⟩, lifetime := ⟨0⟩ }
  { fixture with
    tables := {
      fixture.tables with
      lifetimes := #[{ kind := .local, loc := ⟨7⟩ }]
      types := fixture.tables.types.push referenceType
      names := fixture.tables.names.push {
        namespaceId := ⟨0⟩, name := "MutableReferenceOwner" } }
    namespaces := #[{
      ns with structs := ns.structs.push {
        loc := ⟨7⟩
        name := ⟨3⟩
        abilities := #[.copy]
        fields := #[{ loc := ⟨7⟩, name := ⟨2⟩, type := typeUse 3 7 }] } }] }

#guard validationHasDiagnosticAt rustSchema rustSemantics invalidRustCopyDeclarationFixture
  "LIR-SEMANTIC-ABILITY" ⟨7⟩

private def genericVectorCopyFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  let binder : GenericBinder := {
    name := "T", kind := .typeArg, abilities := #[.copy], loc := ⟨8⟩ }
  { fixture with
    tables := {
      fixture.tables with
      types := fixture.tables.types ++ #[.typeParameter 0, .vector ⟨3⟩ none]
      names := fixture.tables.names.push {
        namespaceId := ⟨0⟩, name := "CopyVector" } }
    namespaces := #[{
      ns with structs := ns.structs.push {
        loc := ⟨8⟩
        name := ⟨3⟩
        generics := #[binder]
        abilities := #[.copy]
        fields := #[{ loc := ⟨8⟩, name := ⟨2⟩, type := typeUse 4 8 }] } }] }

#guard (prepare? rustSchema rustSemantics genericVectorCopyFixture).isSome

private def unconstrainedVectorCopyFixture : RawUnit :=
  let ns := genericVectorCopyFixture.namespaces[0]!
  let declaration := ns.structs[1]!
  let binder := declaration.generics[0]!
  { genericVectorCopyFixture with namespaces := #[{
      ns with structs := ns.structs.set! 1 {
        declaration with generics := #[{ binder with abilities := #[] }] } }] }

#guard validationHasDiagnosticAt rustSchema rustSemantics unconstrainedVectorCopyFixture
  "LIR-SEMANTIC-ABILITY" ⟨8⟩

private def unconstrainedRustDropFixture : RawUnit :=
  let ns := unconstrainedVectorCopyFixture.namespaces[0]!
  let declaration := ns.structs[1]!
  { unconstrainedVectorCopyFixture with namespaces := #[{
      ns with structs := ns.structs.set! 1 {
        declaration with abilities := #[.drop] } }] }

#guard (prepare? rustSchema rustSemantics unconstrainedRustDropFixture).isSome

private def moveKeyFixture (fieldType : Nat) : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with
    tables := {
      fixture.tables with
      types := (fixture.tables.types.set! 2 (.nominal ⟨3⟩ #[])).push .signer
      names := fixture.tables.names.push {
        namespaceId := ⟨0⟩, name := "Resource" } }
    profiles := #[moveConfig]
    namespaces := #[{
      ns with
      profile := some .move
      expressions := #[]
      patterns := #[]
      places := #[]
      structs := #[{
        loc := ⟨9⟩
        name := ⟨3⟩
        abilities := #[.key]
        fields := #[{ loc := ⟨9⟩, name := ⟨2⟩, type := typeUse fieldType 9 }] }]
      functions := #[] }] }

/-! Move `Key` requires every payload field to have `Store`, rather than
requiring nested fields to have `Key`. -/
#guard (prepare? moveSchema moveSemantics (moveKeyFixture 1)).isSome
#guard validationHasDiagnosticAt moveSchema moveSemantics (moveKeyFixture 3)
  "LIR-SEMANTIC-ABILITY" ⟨9⟩

private def prepared : ExecutableUnit := executable?.get (by native_decide)
private def copyPrepared : ExecutableUnit := copyableExecutable?.get (by native_decide)

private theorem successfulRunHasDerivation (executable : ExecutableUnit)
    (success : (Interpreter.run executable 16 handle #[]).isOk) :
    ∃ (finalState : RuntimeState) (outcome : LocatedOutcome),
      BigStep.EvalFunction executable handle #[] {} #[] finalState outcome.value := by
  generalize result_eq : Interpreter.run executable 16 handle #[] = result
  cases result with
  | error error => simp [result_eq, Except.isOk, Except.toBool] at success
  | ok result =>
      exact ⟨result.1, result.2,
        LeanerIR.Proofs.Interpreter.run_sound executable 16 handle #[] {}
          result.1 result.2 result_eq⟩

example : ∃ (finalState : RuntimeState) (outcome : LocatedOutcome),
    BigStep.EvalFunction prepared handle #[] {} #[] finalState outcome.value := by
  apply successfulRunHasDerivation prepared
  native_decide

example : ∃ (finalState : RuntimeState) (outcome : LocatedOutcome),
    BigStep.EvalFunction copyPrepared handle #[] {} #[] finalState outcome.value := by
  apply successfulRunHasDerivation copyPrepared
  native_decide

end LeanerIR.Tests.Abilities
