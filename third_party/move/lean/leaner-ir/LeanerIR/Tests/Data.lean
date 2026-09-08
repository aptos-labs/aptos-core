-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Interpreter
import LeanerIR.Validation.Check

namespace LeanerIR.Tests.Data

open LeanerIR
open LeanerIR.Import
open LeanerIR.Validation

private def config : ProfileConfig := { profile := .move, name := "move-test" }
private def schema : ProfileSchema := { profile := .move, name := "move-test" }
private def semantics : SemanticProfile := {
  profile := .move, name := "move-test", classify := fun _ _ => none }

private def typeUse (typeId loc : Nat) : TypeUse := { typeId := ⟨typeId⟩, loc := ⟨loc⟩ }

private def pair : QualifiedRef := { namespaceId := ⟨0⟩, name := ⟨1⟩ }
private def choice : QualifiedRef := { namespaceId := ⟨0⟩, name := ⟨4⟩ }

private def field (name : Nat) : FieldDecl := {
  loc := ⟨0⟩, name := ⟨name⟩, type := typeUse 0 0 }

private def fixture : RawUnit where
  tables := {
    files := #[{ name := "data.move" }]
    locations := (Array.range 16).map fun index => {
      primary := some { file := ⟨0⟩, startByte := index, endByte := index + 1 } }
    origins := #[{ kind := .moveSource, location := ⟨0⟩ }]
    alignments := #[{ source := ⟨0⟩, trust := .checked, description := "data fixture" }]
    types := #[.bool, .nominal ⟨1⟩ #[], .nominal ⟨4⟩ #[]]
    namespaces := #[{ segments := #["test", "Data"] }]
    names := #[
      { namespaceId := ⟨0⟩, name := "main" },
      { namespaceId := ⟨0⟩, name := "Pair" },
      { namespaceId := ⟨0⟩, name := "first" },
      { namespaceId := ⟨0⟩, name := "second" },
      { namespaceId := ⟨0⟩, name := "Choice" },
      { namespaceId := ⟨0⟩, name := "Left" },
      { namespaceId := ⟨0⟩, name := "Right" },
      { namespaceId := ⟨0⟩, name := "value" },
      { namespaceId := ⟨0⟩, name := "invalid" }] }
  profiles := #[config]
  namespaces := #[{
    loc := ⟨0⟩
    identity := ⟨0⟩
    profile := some .move
    expressions := #[
      { loc := ⟨0⟩, typeId := ⟨0⟩, kind := .value (.bool true) },
      { loc := ⟨1⟩, typeId := ⟨0⟩, kind := .value (.bool false) },
      { loc := ⟨2⟩, typeId := ⟨1⟩,
        kind := .operation (.call (.constructor pair)) #[] #[⟨0⟩, ⟨1⟩] },
      { loc := ⟨3⟩, typeId := ⟨1⟩,
        kind := .operation (.data (.updateField pair "second")) #[] #[⟨2⟩, ⟨0⟩] },
      { loc := ⟨4⟩, typeId := ⟨0⟩,
        kind := .operation (.data (.select pair "second")) #[] #[⟨3⟩] },
      { loc := ⟨5⟩, typeId := ⟨2⟩,
        kind := .operation (.call (.constructor choice (some "Right"))) #[] #[⟨0⟩] },
      { loc := ⟨6⟩, typeId := ⟨0⟩,
        kind := .operation (.data (.testVariants choice #["Right"])) #[] #[⟨5⟩] },
      { loc := ⟨7⟩, typeId := ⟨0⟩,
        kind := .operation (.data (.selectVariants choice #["value"])) #[] #[⟨5⟩] },
      { loc := ⟨8⟩, typeId := ⟨0⟩,
        kind := .operation (.primitive .logicalAnd) #[] #[⟨4⟩, ⟨6⟩] },
      { loc := ⟨9⟩, typeId := ⟨0⟩,
        kind := .operation (.primitive .logicalAnd) #[] #[⟨8⟩, ⟨7⟩] },
      { loc := ⟨10⟩, typeId := ⟨0⟩,
        kind := .operation (.data (.select pair "missing")) #[] #[⟨2⟩] }]
    structs := #[
      { loc := ⟨11⟩, name := ⟨1⟩, fields := #[field 2, field 3] },
      { loc := ⟨12⟩, name := ⟨4⟩, variants := #[
          { loc := ⟨12⟩, name := ⟨5⟩, fields := #[] },
          { loc := ⟨12⟩, name := ⟨6⟩, fields := #[field 7] }] }]
    functions := #[{
        loc := ⟨13⟩
        name := ⟨0⟩
        profile := .move
        signature := { results := #[typeUse 0 13] }
        body := .structured ⟨9⟩
        origin := ⟨0⟩
        alignment := ⟨0⟩
      }] }]

private def executable? : Option ExecutableUnit := do
  let checked ← (validate #[schema] fixture).toOption
  (prepareExecution #[semantics] checked).toOption

private def invalidDataFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with namespaces := #[{
      ns with functions := ns.functions.push {
        loc := ⟨14⟩
        name := ⟨8⟩
        profile := .move
        signature := { results := #[typeUse 0 14] }
        body := .structured ⟨10⟩
        origin := ⟨0⟩
        alignment := ⟨0⟩ } }] }

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

private def partialFieldMoveFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  let unitType : TypeId := ⟨3⟩
  let expressions := ns.expressions ++ #[
    { loc := ⟨14⟩, typeId := unitType, kind := .assign ⟨0⟩ ⟨2⟩ },
    { loc := ⟨14⟩, typeId := ⟨0⟩, kind := .operation (.move ⟨1⟩) #[] #[] },
    { loc := ⟨14⟩, typeId := ⟨0⟩, kind := .operation (.read ⟨2⟩) #[] #[] },
    { loc := ⟨14⟩, typeId := ⟨0⟩, kind := .block #[⟨11⟩, ⟨12⟩] (some ⟨13⟩) }]
  { fixture with
    tables := { fixture.tables with types := fixture.tables.types.push .unit }
    namespaces := #[{ ns with
      expressions
      places := #[.localVar ⟨0⟩,
        .field ⟨0⟩ { namespaceId := ⟨0⟩, name := ⟨1⟩ } ⟨2⟩,
        .field ⟨0⟩ { namespaceId := ⟨0⟩, name := ⟨1⟩ } ⟨3⟩]
      functions := #[{ ns.functions[0]! with
        body := .structured ⟨14⟩
        locals := #[{
          id := ⟨0⟩, name := "pair", type := typeUse 1 14,
          mutable := true, loc := ⟨14⟩ }] }] }] }

private def partialFieldExecutable? : Option ExecutableUnit := do
  let checked ← (validate #[schema] partialFieldMoveFixture).toOption
  (prepareExecution #[semantics] checked).toOption

#guard match partialFieldExecutable? with
  | some executable => match Interpreter.run executable 32
      { namespaceId := ⟨0⟩, functionId := ⟨0⟩ } #[] with
      | .ok (_, { value := .returned #[.bool false], .. }) => true
      | _ => false
  | none => false

private def movedFieldReadFixture : RawUnit :=
  let ns := partialFieldMoveFixture.namespaces[0]!
  { partialFieldMoveFixture with namespaces := #[{ ns with
      expressions := ns.expressions.set! 13 {
        ns.expressions[13]! with kind := .operation (.read ⟨1⟩) #[] #[] } }] }

#guard validationHasDiagnosticAt movedFieldReadFixture
  "LIR-SEMANTIC-INITIALIZATION" ⟨14⟩

private def partiallyMovedAggregateReadFixture : RawUnit :=
  let ns := partialFieldMoveFixture.namespaces[0]!
  let expressions := ns.expressions.set! 13 {
    ns.expressions[13]! with typeId := ⟨1⟩, kind := .operation (.read ⟨0⟩) #[] #[] }
  let expressions := expressions.set! 14 { expressions[14]! with typeId := ⟨1⟩ }
  { partialFieldMoveFixture with namespaces := #[{ ns with
      expressions
      functions := #[{ ns.functions[0]! with
        signature := { results := #[typeUse 1 14] } }] }] }

#guard validationHasDiagnosticAt partiallyMovedAggregateReadFixture
  "LIR-SEMANTIC-INITIALIZATION" ⟨14⟩

private def reinitializedFieldFixture : RawUnit :=
  let ns := partialFieldMoveFixture.namespaces[0]!
  { partialFieldMoveFixture with namespaces := #[{ ns with
      expressions := ns.expressions ++ #[
        { loc := ⟨14⟩, typeId := ⟨3⟩, kind := .assign ⟨1⟩ ⟨0⟩ },
        { loc := ⟨14⟩, typeId := ⟨1⟩, kind := .operation (.move ⟨0⟩) #[] #[] },
        { loc := ⟨14⟩, typeId := ⟨1⟩,
          kind := .block #[⟨11⟩, ⟨12⟩, ⟨15⟩] (some ⟨16⟩) }]
      functions := #[{ ns.functions[0]! with
        signature := { results := #[typeUse 1 14] }
        body := .structured ⟨17⟩ }] }] }

#guard match validate #[schema] reinitializedFieldFixture with
  | .ok checked => (prepareExecution #[semantics] checked).isOk
  | .error _ => false

#guard match validate #[schema] reinitializedFieldFixture with
  | .ok checked => match prepareExecution #[semantics] checked with
      | .ok executable => match Interpreter.run executable 32
          { namespaceId := ⟨0⟩, functionId := ⟨0⟩ } #[] with
          | .ok (_, { value := .returned #[.nominal _ none #[.bool true, .bool false]], .. }) => true
          | _ => false
      | .error _ => false
  | .error _ => false

private def uninitializedFieldWriteFixture : RawUnit :=
  let ns := reinitializedFieldFixture.namespaces[0]!
  { reinitializedFieldFixture with namespaces := #[{ ns with
      expressions := ns.expressions.set! 17 {
        ns.expressions[17]! with kind := .block #[⟨15⟩] (some ⟨16⟩) } }] }

#guard validationHasDiagnosticAt uninitializedFieldWriteFixture
  "LIR-SEMANTIC-INITIALIZATION" ⟨14⟩

private def branchPartialMoveFixture (readPlace : PlaceId) : RawUnit :=
  let ns := partialFieldMoveFixture.namespaces[0]!
  { partialFieldMoveFixture with namespaces := #[{ ns with
      expressions := ns.expressions ++ #[
        { loc := ⟨14⟩, typeId := ⟨3⟩, kind := .block #[⟨12⟩] none },
        { loc := ⟨14⟩, typeId := ⟨3⟩, kind := .block #[] none },
        { loc := ⟨14⟩, typeId := ⟨3⟩,
          kind := .ifElse ⟨0⟩ ⟨15⟩ (some ⟨16⟩) },
        { loc := ⟨14⟩, typeId := ⟨0⟩,
          kind := .operation (.read readPlace) #[] #[] },
        { loc := ⟨14⟩, typeId := ⟨0⟩,
          kind := .block #[⟨11⟩, ⟨17⟩] (some ⟨18⟩) }]
      functions := #[{ ns.functions[0]! with body := .structured ⟨19⟩ }] }] }

#guard validationHasDiagnosticAt (branchPartialMoveFixture ⟨1⟩)
  "LIR-SEMANTIC-INITIALIZATION" ⟨14⟩

#guard match validate #[schema] (branchPartialMoveFixture ⟨2⟩) with
  | .ok checked => (prepareExecution #[semantics] checked).isOk
  | .error _ => false

private def literalIndexMoveFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  let unitType : TypeId := ⟨3⟩
  let tupleType : TypeId := ⟨4⟩
  let indexType : TypeId := ⟨5⟩
  let expressions := ns.expressions ++ #[
    { loc := ⟨14⟩, typeId := tupleType,
      kind := .value (.tuple #[.bool true, .bool false]) },
    { loc := ⟨14⟩, typeId := indexType, kind := .value (.integer 0) },
    { loc := ⟨14⟩, typeId := indexType, kind := .value (.integer 1) },
    { loc := ⟨14⟩, typeId := unitType, kind := .assign ⟨0⟩ ⟨11⟩ },
    { loc := ⟨14⟩, typeId := ⟨0⟩, kind := .operation (.move ⟨1⟩) #[] #[] },
    { loc := ⟨14⟩, typeId := ⟨0⟩, kind := .operation (.read ⟨2⟩) #[] #[] },
    { loc := ⟨14⟩, typeId := ⟨0⟩, kind := .block #[⟨14⟩, ⟨15⟩] (some ⟨16⟩) }]
  { fixture with
    tables := { fixture.tables with types := fixture.tables.types ++ #[
      .unit, .tuple #[⟨0⟩, ⟨0⟩], .integer (.bits 64) false] }
    namespaces := #[{ ns with
      expressions
      places := #[.localVar ⟨0⟩, .index ⟨0⟩ ⟨12⟩, .index ⟨0⟩ ⟨13⟩]
      functions := #[{ ns.functions[0]! with
        body := .structured ⟨17⟩
        locals := #[{
          id := ⟨0⟩, name := "pair", type := typeUse 4 14,
          mutable := true, loc := ⟨14⟩ }] }] }] }

#guard match validate #[schema] literalIndexMoveFixture with
  | .ok checked => match prepareExecution #[semantics] checked with
      | .ok executable => match Interpreter.run executable 32
          { namespaceId := ⟨0⟩, functionId := ⟨0⟩ } #[] with
          | .ok (_, { value := .returned #[.bool false], .. }) => true
          | _ => false
      | .error _ => false
  | .error _ => false

private def movedLiteralIndexReadFixture : RawUnit :=
  let ns := literalIndexMoveFixture.namespaces[0]!
  { literalIndexMoveFixture with namespaces := #[{ ns with
      expressions := ns.expressions.set! 16 {
        ns.expressions[16]! with kind := .operation (.read ⟨1⟩) #[] #[] } }] }

#guard validationHasDiagnosticAt movedLiteralIndexReadFixture
  "LIR-SEMANTIC-INITIALIZATION" ⟨14⟩

private def partiallyMovedIndexAggregateReadFixture : RawUnit :=
  let ns := literalIndexMoveFixture.namespaces[0]!
  let expressions := ns.expressions
    |>.set! 16 { ns.expressions[16]! with
      typeId := ⟨4⟩, kind := .operation (.read ⟨0⟩) #[] #[] }
    |>.set! 17 { ns.expressions[17]! with typeId := ⟨4⟩ }
  { literalIndexMoveFixture with namespaces := #[{ ns with
      expressions
      functions := #[{ ns.functions[0]! with
        signature := { results := #[typeUse 4 14] } }] }] }

#guard validationHasDiagnosticAt partiallyMovedIndexAggregateReadFixture
  "LIR-SEMANTIC-INITIALIZATION" ⟨14⟩

private def reinitializedLiteralIndexFixture : RawUnit :=
  let ns := literalIndexMoveFixture.namespaces[0]!
  { literalIndexMoveFixture with namespaces := #[{ ns with
      expressions := ns.expressions ++ #[
        { loc := ⟨14⟩, typeId := ⟨3⟩, kind := .assign ⟨1⟩ ⟨0⟩ },
        { loc := ⟨14⟩, typeId := ⟨4⟩, kind := .operation (.move ⟨0⟩) #[] #[] },
        { loc := ⟨14⟩, typeId := ⟨4⟩,
          kind := .block #[⟨14⟩, ⟨15⟩, ⟨18⟩] (some ⟨19⟩) }]
      functions := #[{ ns.functions[0]! with
        signature := { results := #[typeUse 4 14] }
        body := .structured ⟨20⟩ }] }] }

#guard match validate #[schema] reinitializedLiteralIndexFixture with
  | .ok checked => match prepareExecution #[semantics] checked with
      | .ok executable => match Interpreter.run executable 32
          { namespaceId := ⟨0⟩, functionId := ⟨0⟩ } #[] with
          | .ok (_, { value := .returned #[.tuple #[.bool true, .bool false]], .. }) => true
          | _ => false
      | .error _ => false
  | .error _ => false

private def dynamicIndexMoveFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with
    tables := { fixture.tables with types := fixture.tables.types ++ #[
      .vector ⟨0⟩ (some (.integer 2)), .integer (.bits 64) false] }
    namespaces := #[{ ns with
      expressions := #[
        { loc := ⟨14⟩, typeId := ⟨4⟩, kind := .localVar ⟨1⟩ },
        { loc := ⟨14⟩, typeId := ⟨0⟩, kind := .operation (.move ⟨1⟩) #[] #[] }]
      places := #[.localVar ⟨0⟩, .index ⟨0⟩ ⟨0⟩]
      functions := #[{ ns.functions[0]! with
        signature := {
          parameters := #[
            { name := "values", typeUse := typeUse 3 14 },
            { name := "index", typeUse := typeUse 4 14 }]
          results := #[typeUse 0 14] }
        body := .structured ⟨1⟩
        locals := #[
          { id := ⟨0⟩, name := "values", type := typeUse 3 14,
            mutable := false, loc := ⟨14⟩ },
          { id := ⟨1⟩, name := "index", type := typeUse 4 14,
            mutable := false, loc := ⟨14⟩ }] }] }] }

#guard match validate #[schema] dynamicIndexMoveFixture with
  | .ok checked => match prepareExecution #[semantics] checked with
      | .ok executable => match Interpreter.run executable 16
          { namespaceId := ⟨0⟩, functionId := ⟨0⟩ }
          #[.vector #[.bool true, .bool false], .integer 1] with
          | .ok (_, { value := .returned #[.bool false], .. }) => true
          | _ => false
      | .error _ => false
  | .error _ => false

private def dynamicIndexAggregateReadFixture : RawUnit :=
  let ns := dynamicIndexMoveFixture.namespaces[0]!
  { dynamicIndexMoveFixture with namespaces := #[{ ns with
      expressions := ns.expressions ++ #[
        { loc := ⟨14⟩, typeId := ⟨3⟩, kind := .operation (.read ⟨0⟩) #[] #[] },
        { loc := ⟨14⟩, typeId := ⟨3⟩, kind := .block #[⟨1⟩] (some ⟨2⟩) }]
      functions := #[{ ns.functions[0]! with
        signature := {
          parameters := ns.functions[0]!.signature.parameters
          results := #[typeUse 3 14] }
        body := .structured ⟨3⟩ }] }] }

#guard validationHasDiagnosticAt dynamicIndexAggregateReadFixture
  "LIR-SEMANTIC-INITIALIZATION" ⟨14⟩

private def badConstructorArgumentFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with
    tables := { fixture.tables with types := fixture.tables.types.push (.integer (.bits 8) false) }
    namespaces := #[{
      ns with
      expressions := ns.expressions.set! 0 {
        ns.expressions[0]! with typeId := ⟨3⟩, kind := .value (.integer 7) } }] }

#guard validationHasDiagnosticAt badConstructorArgumentFixture "LIR-SEMANTIC-TYPE" ⟨2⟩

private def badVariantTestFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with namespaces := #[{
      ns with expressions := ns.expressions.set! 6 {
        ns.expressions[6]! with kind := (.operation
          (.data (.testVariants choice #["Missing"])) #[] #[⟨5⟩]) } }] }

#guard validationHasDiagnosticAt badVariantTestFixture "LIR-SEMANTIC-TARGET" ⟨6⟩

private def constructorPatternFixture (patternType : Nat := 1)
    (childType : Nat := 0) (variant : Option String := none)
    (fieldCount : Nat := 2) : RawUnit :=
  let ns := fixture.namespaces[0]!
  let fields := Array.replicate fieldCount (⟨0⟩ : PatternId)
  let root : ExprId := ⟨ns.expressions.size⟩
  { fixture with namespaces := #[{ ns with
      patterns := #[
        { loc := ⟨14⟩, typeId := ⟨childType⟩, kind := .wildcard },
        { loc := ⟨14⟩, typeId := ⟨patternType⟩,
          kind := .constructor pair.name #[] variant fields }]
      expressions := ns.expressions.push {
        loc := ⟨14⟩, typeId := ⟨0⟩,
        kind := .match_ ⟨2⟩ #[{ pattern := ⟨1⟩, body := ⟨0⟩ }] }
      functions := #[{ ns.functions[0]! with body := .structured root }] }] }

private def constructorPatternPrepares (raw : RawUnit) : Bool :=
  match validate #[schema] raw with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard constructorPatternPrepares constructorPatternFixture
#guard validationHasDiagnosticAt (constructorPatternFixture (childType := 1))
  "LIR-SEMANTIC-TYPE" ⟨14⟩
#guard validationHasDiagnosticAt (constructorPatternFixture (variant := some "Right"))
  "LIR-SEMANTIC-TARGET" ⟨14⟩
#guard validationHasDiagnosticAt (constructorPatternFixture (fieldCount := 1))
  "LIR-SEMANTIC-ARITY" ⟨14⟩

private def genericVariantSelectionFixture (resultType : Nat := 0) : RawUnit :=
  let ns := fixture.namespaces[0]!
  let choiceDecl := ns.structs[1]!
  let typeArgument : GenericArgument := .typeArg (typeUse 0 12)
  let binder : GenericBinder := {
    loc := ⟨12⟩, name := "T", kind := .typeArg, abilities := #[.copy] }
  let variants := choiceDecl.variants.map fun variant =>
    { variant with fields := variant.fields.map fun field =>
        { field with type := typeUse 3 12 } }
  { fixture with
    tables := { fixture.tables with
      types := (fixture.tables.types.set! 2 (.nominal ⟨4⟩ #[typeArgument])).push
        (.typeParameter 0) }
    namespaces := #[{ ns with
      expressions := (ns.expressions.set! 5 {
        ns.expressions[5]! with
        kind := .operation (.call (.constructor choice (some "Right")))
          #[typeArgument] #[⟨0⟩] }).set! 7 {
        ns.expressions[7]! with typeId := ⟨resultType⟩ }
      structs := ns.structs.set! 1 {
        choiceDecl with generics := #[binder], variants } }] }

#guard constructorPatternPrepares genericVariantSelectionFixture
#guard validationHasDiagnosticAt (genericVariantSelectionFixture 1)
  "LIR-SEMANTIC-TYPE" ⟨7⟩

private def discriminantFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  let choiceDecl := ns.structs[1]!
  let variants := #[
    { choiceDecl.variants[0]! with discriminant := some 11 },
    { choiceDecl.variants[1]! with discriminant := some 29 }]
  { fixture with
    tables := { fixture.tables with
      names := fixture.tables.names.push { namespaceId := ⟨0⟩, name := "discriminant" }
      types := fixture.tables.types ++ #[.integer (.bits 8) false, .unit] }
    namespaces := #[{ ns with
      expressions := ns.expressions ++ #[
        { loc := ⟨15⟩, typeId := ⟨4⟩, kind := .assign ⟨0⟩ ⟨5⟩ },
        { loc := ⟨15⟩, typeId := ⟨2⟩, kind := .operation (.read ⟨0⟩) #[] #[] },
        { loc := ⟨15⟩, typeId := ⟨3⟩,
          kind := .operation (.data (.discriminant choice)) #[] #[⟨12⟩] },
        { loc := ⟨15⟩, typeId := ⟨3⟩, kind := .block #[⟨11⟩] (some ⟨13⟩) }]
      places := #[.localVar ⟨0⟩]
      structs := ns.structs.set! 1 { choiceDecl with variants }
      functions := ns.functions.push {
        loc := ⟨15⟩
        name := ⟨9⟩
        profile := .move
        signature := { results := #[typeUse 3 15] }
        body := .structured ⟨14⟩
        locals := #[{
          id := ⟨0⟩, name := "choice", type := typeUse 2 15,
          mutable := true, loc := ⟨15⟩ }]
        origin := ⟨0⟩
        alignment := ⟨0⟩ } }] }

private def discriminantExecutable? : Option ExecutableUnit := do
  let checked ← (validate #[schema] discriminantFixture).toOption
  (prepareExecution #[semantics] checked).toOption

private def handle (functionId : Nat) : FunctionHandle := {
  namespaceId := ⟨0⟩, functionId := ⟨functionId⟩ }

#guard match executable? with
  | some executable => match Interpreter.run executable 32 (handle 0) #[] with
      | .ok (_, { value := .returned #[.bool true], .. }) => true
      | _ => false
  | none => false

#guard match discriminantExecutable? with
  | some executable => match Interpreter.run executable 32 (handle 1) #[] with
      | .ok (_, { value := .returned #[.integer 29], .. }) => true
      | _ => false
  | none => false

/-- The exemption above is specific to the immediate operand of discriminant
inspection; returning the same non-Copy enum through an ordinary read remains
an ability error. -/
private def ordinaryEnumReadFixture : RawUnit :=
  let ns := discriminantFixture.namespaces[0]!
  { discriminantFixture with namespaces := #[{ ns with
      expressions := ns.expressions.set! 14 {
        ns.expressions[14]! with typeId := ⟨2⟩, kind := .block #[⟨11⟩] (some ⟨12⟩) }
      functions := ns.functions.set! 1 { ns.functions[1]! with
        signature := { results := #[typeUse 2 15] } } }] }

#guard preparationHasDiagnosticAt ordinaryEnumReadFixture
  "LIR-SEMANTIC-ABILITY" ⟨15⟩

#guard match validate #[schema] invalidDataFixture with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-TARGET" &&
        diagnostic.primary == some ⟨10⟩ &&
        invalidDataFixture.tables.locations[10]!.primary == some {
          file := ⟨0⟩, startByte := 10, endByte := 11 }
  | .ok _ => false

private def prepared : ExecutableUnit := executable?.get (by native_decide)

private theorem successfulRunHasDerivation (fuel : Nat) (function : FunctionHandle)
    (success : (Interpreter.run prepared fuel function #[]).isOk) :
    ∃ (finalState : RuntimeState) (outcome : LocatedOutcome),
      BigStep.EvalFunction prepared function {} #[] finalState outcome.value := by
  generalize result_eq : Interpreter.run prepared fuel function #[] = result
  cases result with
  | error error => simp [result_eq, Except.isOk, Except.toBool] at success
  | ok result =>
      exact ⟨result.1, result.2,
        LeanerIR.Proofs.Interpreter.run_sound prepared fuel function #[] {}
          result.1 result.2 result_eq⟩

example : ∃ (finalState : RuntimeState) (outcome : LocatedOutcome),
    BigStep.EvalFunction prepared (handle 0) {} #[] finalState outcome.value := by
  apply successfulRunHasDerivation 32 (handle 0)
  native_decide

end LeanerIR.Tests.Data
