-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Interpreter
import LeanerIR.Validation.Check

namespace LeanerIR.Tests.Declarations

open LeanerIR
open LeanerIR.Import
open LeanerIR.Validation

private def config : ProfileConfig := { profile := .rust, name := "rust-test" }
private def schema : ProfileSchema := { profile := .rust, name := "rust-test" }
private def semantics : SemanticProfile := {
  profile := .rust, name := "rust-test", classify := fun _ _ => none }
private def moveConfig : ProfileConfig := { profile := .move, name := "move-test" }
private def moveSchema : ProfileSchema := { profile := .move, name := "move-test" }
private def moveSemantics : SemanticProfile := {
  profile := .move, name := "move-test", classify := fun _ _ => none }

private def typeUse (typeId loc : Nat) : TypeUse := { typeId := ⟨typeId⟩, loc := ⟨loc⟩ }
private def ownRef (name : Nat) : QualifiedRef := { namespaceId := ⟨0⟩, name := ⟨name⟩ }

private def fixture : RawUnit where
  tables := {
    files := #[{ name := "declarations.lir" }]
    locations := (Array.range 12).map fun index => {
      primary := some { file := ⟨0⟩, startByte := index, endByte := index + 1 } }
    origins := #[{ kind := .leanerSource, location := ⟨0⟩ }]
    alignments := #[{
      source := ⟨0⟩, trust := .checked, description := "declaration fixture" }]
    types := #[.unit, .bool, .integer (.bits 64) false]
    namespaces := #[{ segments := #["test", "Declarations"] }]
    names := #[
      { namespaceId := ⟨0⟩, name := "answer" },
      { namespaceId := ⟨0⟩, name := "useAnswer" },
      { namespaceId := ⟨0⟩, name := "missing" }] }
  profiles := #[config]
  namespaces := #[{
    loc := ⟨0⟩
    identity := ⟨0⟩
    profile := some .rust
    expressions := #[
      { loc := ⟨0⟩, typeId := ⟨2⟩, kind := .value (.integer 42) },
      { loc := ⟨1⟩, typeId := ⟨2⟩, kind := .constant (ownRef 0) }]
    constants := #[{
      loc := ⟨3⟩, name := ⟨0⟩, type := typeUse 2 3, value := ⟨0⟩ }]
    functions := #[{
      loc := ⟨4⟩
      name := ⟨1⟩
      profile := .rust
      signature := { results := #[typeUse 2 4] }
      body := .structured ⟨1⟩
      origin := ⟨0⟩
      alignment := ⟨0⟩ }] }]

private def prepare? (raw : RawUnit) : Option ExecutableUnit := do
  let checked ← (validate #[schema] raw).toOption
  (prepareExecution #[semantics] checked).toOption

private def executable? : Option ExecutableUnit := prepare? fixture
private def handle : FunctionHandle := { namespaceId := ⟨0⟩, functionId := ⟨0⟩ }

#guard match executable? with
  | some executable => match Interpreter.run executable 16 handle #[] with
      | .ok (_, { value := .returned #[.integer 42], .. }) => true
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

private def badInitializerFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with namespaces := #[{
      ns with expressions := ns.expressions.set! 0 {
        ns.expressions[0]! with typeId := ⟨1⟩, kind := .value (.bool true) } }] }

#guard validationHasDiagnosticAt badInitializerFixture "LIR-SEMANTIC-TYPE" ⟨3⟩

private def badReferenceTypeFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with namespaces := #[{
      ns with
      expressions := ns.expressions.set! 1 { ns.expressions[1]! with typeId := ⟨1⟩ }
      functions := #[{
        ns.functions[0]! with signature := { results := #[typeUse 1 4] } }] }] }

#guard validationHasDiagnosticAt badReferenceTypeFixture "LIR-SEMANTIC-TYPE" ⟨1⟩

private def missingReferenceFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with namespaces := #[{
      ns with expressions := ns.expressions.set! 1 {
        ns.expressions[1]! with kind := .constant (ownRef 2) } }] }

#guard validationHasDiagnosticAt missingReferenceFixture "LIR-SEMANTIC-TARGET" ⟨1⟩

/- Constant initializers inherit their namespace profile. This local Rust call
would be rejected as a cross-profile call if preparation silently used the
default Move scan context. -/
private def rustConstantCallFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with
    tables := { fixture.tables with names := fixture.tables.names.push {
      namespaceId := ⟨0⟩, name := "makeAnswer" } }
    namespaces := #[{ ns with
      expressions := ns.expressions.push {
        loc := ⟨5⟩, typeId := ⟨2⟩, kind := .operation
          (.call (.function (ownRef 3))) #[] #[] }
      constants := #[{ ns.constants[0]! with value := ⟨2⟩ }]
      functions := ns.functions.push {
        loc := ⟨5⟩
        name := ⟨3⟩
        profile := .rust
        signature := { results := #[typeUse 2 5] }
        body := .structured ⟨0⟩
        origin := ⟨0⟩
        alignment := ⟨0⟩ } }] }

#guard (prepare? rustConstantCallFixture).isSome

private def associatedConstantFixture (typeId : Nat := 2) : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with
    tables := { fixture.tables with names := fixture.tables.names ++ #[
        { namespaceId := ⟨0⟩, name := "HasAnswer" },
        { namespaceId := ⟨0⟩, name := "ANSWER" }] }
    namespaces := #[{ ns with
      associatedItems := #[{
        id := ⟨0⟩, loc := ⟨5⟩, owner := ⟨0⟩, name := ⟨4⟩,
        kind := .constant (typeUse typeId 5) (some ⟨0⟩) }]
      traits := #[{
        id := ⟨0⟩, loc := ⟨5⟩, name := ⟨3⟩, associatedItems := #[⟨0⟩] }] }] }

#guard (prepare? associatedConstantFixture).isSome
#guard validationHasDiagnosticAt (associatedConstantFixture 1)
  "LIR-SEMANTIC-TYPE" ⟨5⟩

private def associatedConstantBindingFixture (bad : Bool := false) : RawUnit :=
  let ns := fixture.namespaces[0]!
  let value : ExprId := if bad then ⟨ns.expressions.size⟩ else ⟨0⟩
  let expressions := if bad then ns.expressions.push {
      loc := ⟨6⟩, typeId := ⟨1⟩, kind := .value (.bool true) }
    else ns.expressions
  { fixture with
    tables := { fixture.tables with
      types := fixture.tables.types.push (.typeParameter 0)
      names := fixture.tables.names ++ #[
        { namespaceId := ⟨0⟩, name := "HasAnswer" },
        { namespaceId := ⟨0⟩, name := "ANSWER" }] }
    namespaces := #[{ ns with
      expressions
      associatedItems := #[{
        id := ⟨0⟩, loc := ⟨5⟩, owner := ⟨0⟩, name := ⟨4⟩,
        kind := .constant (typeUse 3 5) }]
      traits := #[{
        id := ⟨0⟩, loc := ⟨5⟩, name := ⟨3⟩,
        generics := #[{
          loc := ⟨5⟩, name := "T", kind := .typeArg, abilities := #[.copy] }],
        associatedItems := #[⟨0⟩] }]
      implementations := #[{
        id := ⟨0⟩, loc := ⟨6⟩,
        trait := {
          trait := { namespaceId := ⟨0⟩, name := ⟨3⟩ },
          arguments := #[.typeArg (typeUse 2 6)] },
        target := typeUse 1 6,
        bindings := #[{ loc := ⟨6⟩, item := ⟨0⟩, value := .constant value }] }] }] }

#guard (prepare? associatedConstantBindingFixture).isSome
#guard validationHasDiagnosticAt (associatedConstantBindingFixture true)
  "LIR-SEMANTIC-TYPE" ⟨6⟩

private def associatedMethodFixture (binding : Bool := false)
    (badTarget : Bool := false) : RawUnit :=
  let ns := fixture.namespaces[0]!
  let target : QualifiedRef := if badTarget then ownRef 2 else ownRef 5
  let itemKind : AssociatedItemKind := if binding then
      .method { results := #[typeUse 2 7] }
    else .method { results := #[typeUse 2 7] } (some target)
  let implementation : Array ImplDecl := if binding then #[{
      id := ⟨0⟩
      loc := ⟨7⟩
      trait := { trait := ownRef 3 }
      target := typeUse 1 7
      bindings := #[{ loc := ⟨7⟩, item := ⟨0⟩, value := .method target }] }]
    else #[]
  { fixture with
    tables := { fixture.tables with names := fixture.tables.names ++ #[
      { namespaceId := ⟨0⟩, name := "HasAnswer" },
      { namespaceId := ⟨0⟩, name := "answer" },
      { namespaceId := ⟨0⟩, name := "defaultAnswer" }] }
    namespaces := #[{ ns with
      associatedItems := #[{
        id := ⟨0⟩, loc := ⟨7⟩, owner := ⟨0⟩, name := ⟨4⟩,
        kind := itemKind }]
      traits := #[{
        id := ⟨0⟩, loc := ⟨7⟩, name := ⟨3⟩,
        associatedItems := #[⟨0⟩] }]
      implementations := implementation
      functions := ns.functions.push {
        loc := ⟨7⟩
        name := ⟨5⟩
        profile := .rust
        signature := { results := #[typeUse 2 7] }
        body := .structured ⟨0⟩
        origin := ⟨0⟩
        alignment := ⟨0⟩ } }] }

#guard (prepare? associatedMethodFixture).isSome
#guard validationHasDiagnosticAt (associatedMethodFixture (badTarget := true))
  "LIR-SEMANTIC-TARGET" ⟨7⟩
#guard (prepare? (associatedMethodFixture (binding := true))).isSome
#guard validationHasDiagnosticAt
  (associatedMethodFixture (binding := true) (badTarget := true))
  "LIR-SEMANTIC-TARGET" ⟨7⟩

/- An associated method signature is scoped over its owner trait binders
before its own binders. The nested nominal use exercises semantic ability
checking, not only raw generic-index bounds. -/
private def associatedGenericMethodFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  let ownerBinder : GenericBinder := {
    loc := ⟨8⟩, name := "T", kind := .typeArg, abilities := #[.copy] }
  let wrapperBinder : GenericBinder := {
    loc := ⟨8⟩, name := "U", kind := .typeArg, abilities := #[.copy] }
  { fixture with
    tables := { fixture.tables with
      types := fixture.tables.types ++ #[
        .typeParameter 0,
        .nominal ⟨5⟩ #[.typeArg (typeUse 3 8)]]
      names := fixture.tables.names ++ #[
        { namespaceId := ⟨0⟩, name := "Transforms" },
        { namespaceId := ⟨0⟩, name := "transform" },
        { namespaceId := ⟨0⟩, name := "Wrapper" }] }
    namespaces := #[{ ns with
      structs := #[{
        loc := ⟨8⟩, name := ⟨5⟩, generics := #[wrapperBinder], abilities := #[.copy] }]
      associatedItems := #[{
        id := ⟨0⟩, loc := ⟨8⟩, owner := ⟨0⟩, name := ⟨4⟩,
        kind := .method { parameters := #[{
          name := "value", typeUse := typeUse 4 8 }] } }]
      traits := #[{
        id := ⟨0⟩, loc := ⟨8⟩, name := ⟨3⟩,
        generics := #[ownerBinder], associatedItems := #[⟨0⟩] }] }] }

#guard (prepare? associatedGenericMethodFixture).isSome

private def associatedGenericMethodWithoutAbilityFixture : RawUnit :=
  let ns := associatedGenericMethodFixture.namespaces[0]!
  let trait := ns.traits[0]!
  let binder := trait.generics[0]!
  { associatedGenericMethodFixture with namespaces := #[{ ns with
      traits := #[{ trait with generics := #[{ binder with abilities := #[] }] }] }] }

#guard validationHasDiagnosticAt associatedGenericMethodWithoutAbilityFixture
  "LIR-SEMANTIC-ABILITY" ⟨8⟩

private def crossProfileCallFixture (closure : Bool := false) : RawUnit :=
  let first := fixture.namespaces[0]!
  let root : ExprId := ⟨first.expressions.size⟩
  let target : QualifiedRef := { namespaceId := ⟨1⟩, name := ⟨3⟩ }
  let callKind : CallKind := if closure then .closure target else .function target
  let resultType : TypeId := if closure then ⟨3⟩ else ⟨2⟩
  let second : RawNamespace := {
    loc := ⟨8⟩
    identity := ⟨1⟩
    profile := some .move
    expressions := #[{
      loc := ⟨8⟩, typeId := ⟨2⟩, kind := .value (.integer 42) }]
    functions := #[{
      loc := ⟨8⟩
      name := ⟨3⟩
      profile := .move
      signature := { results := #[typeUse 2 8] }
      body := .structured ⟨0⟩
      origin := ⟨0⟩
      alignment := ⟨0⟩ }] }
  { fixture with
    tables := { fixture.tables with
      types := fixture.tables.types.push (.function #[] ⟨2⟩)
      namespaces := fixture.tables.namespaces.push {
        segments := #["test", "Declarations", "Move"] }
      names := fixture.tables.names.push {
        namespaceId := ⟨1⟩, name := "foreignAnswer" } }
    profiles := #[config, moveConfig]
    namespaces := #[{ first with
      imports := #[⟨1⟩]
      expressions := first.expressions.push {
        loc := ⟨8⟩, typeId := resultType,
        kind := .operation (.call callKind) #[] #[] }
      functions := #[{ first.functions[0]! with
        signature := { results := #[typeUse resultType.index 8] }
        body := .structured root }] }, second] }

private def crossProfileValidationHasMismatch (raw : RawUnit) : Bool :=
  match validate #[schema, moveSchema] raw with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-PROFILE-MISMATCH" && diagnostic.primary == some ⟨8⟩
  | .ok _ => false

#guard crossProfileValidationHasMismatch crossProfileCallFixture
#guard crossProfileValidationHasMismatch (crossProfileCallFixture (closure := true))

private def associatedEqualityFixture (typeItem : Nat := 0)
    (constantItem : Nat := 1) (constantValue : ConstValue := .integer 42) : RawUnit :=
  let ns := fixture.namespaces[0]!
  let signature : Signature := {
    results := #[typeUse 2 9]
    predicates := #[
      .associatedTypeEq { trait := ownRef 3 } ⟨typeItem⟩ ⟨2⟩,
      .associatedConstEq { trait := ownRef 3 } ⟨constantItem⟩ constantValue] }
  { fixture with
    tables := { fixture.tables with names := fixture.tables.names ++ #[
      { namespaceId := ⟨0⟩, name := "HasItems" },
      { namespaceId := ⟨0⟩, name := "Item" },
      { namespaceId := ⟨0⟩, name := "ITEM" },
      { namespaceId := ⟨0⟩, name := "OtherItems" },
      { namespaceId := ⟨0⟩, name := "Other" }] }
    namespaces := #[{ ns with
      associatedItems := #[
        { id := ⟨0⟩, loc := ⟨9⟩, owner := ⟨0⟩, name := ⟨4⟩,
          kind := .type #[] },
        { id := ⟨1⟩, loc := ⟨9⟩, owner := ⟨0⟩, name := ⟨5⟩,
          kind := .constant (typeUse 2 9) },
        { id := ⟨2⟩, loc := ⟨9⟩, owner := ⟨1⟩, name := ⟨7⟩,
          kind := .type #[] }]
      traits := #[
        { id := ⟨0⟩, loc := ⟨9⟩, name := ⟨3⟩,
          associatedItems := #[⟨0⟩, ⟨1⟩] },
        { id := ⟨1⟩, loc := ⟨9⟩, name := ⟨6⟩,
          associatedItems := #[⟨2⟩] }]
      functions := #[{ ns.functions[0]! with signature }] }] }

#guard (prepare? associatedEqualityFixture).isSome
#guard validationHasDiagnosticAt (associatedEqualityFixture (typeItem := 1))
  "LIR-SEMANTIC-ASSOCIATED-ITEM" ⟨0⟩
#guard validationHasDiagnosticAt (associatedEqualityFixture (constantItem := 0))
  "LIR-SEMANTIC-ASSOCIATED-ITEM" ⟨0⟩
#guard validationHasDiagnosticAt (associatedEqualityFixture (typeItem := 2))
  "LIR-SEMANTIC-ASSOCIATED-ITEM" ⟨0⟩
#guard validationHasDiagnosticAt (associatedEqualityFixture (constantValue := .bool true))
  "LIR-SEMANTIC-TYPE" ⟨0⟩

private def associatedGenericConstantEqualityFixture
    (value : ConstValue := .bool true) : RawUnit :=
  let raw := associatedEqualityFixture
  let ns := raw.namespaces[0]!
  let trait := ns.traits[0]!
  let item := ns.associatedItems[1]!
  let function := ns.functions[0]!
  let traitRef : TraitRef := {
    trait := ownRef 3, arguments := #[.typeArg (typeUse 1 9)] }
  { raw with
    tables := { raw.tables with types := raw.tables.types.push (.typeParameter 0) }
    namespaces := #[{ ns with
      associatedItems := ns.associatedItems.set! 1 {
        item with kind := .constant (typeUse 3 9) }
      traits := ns.traits.set! 0 { trait with generics := #[{
        loc := ⟨9⟩, name := "T", kind := .typeArg }] }
      functions := #[{ function with signature := {
        function.signature with predicates := #[
          .associatedConstEq traitRef ⟨1⟩ value] } }] }] }

#guard (prepare? associatedGenericConstantEqualityFixture).isSome
#guard validationHasDiagnosticAt
  (associatedGenericConstantEqualityFixture (.integer 42))
  "LIR-SEMANTIC-TYPE" ⟨0⟩

private def crossNamespaceAssociatedEqualityFixture : RawUnit :=
  let first := fixture.namespaces[0]!
  let second : RawNamespace := {
    loc := ⟨10⟩
    identity := ⟨1⟩
    profile := some .rust
    imports := #[⟨0⟩]
    expressions := #[{
      loc := ⟨10⟩, typeId := ⟨2⟩, kind := .value (.integer 42) }]
    functions := #[{
      loc := ⟨10⟩
      name := ⟨5⟩
      profile := .rust
      signature := {
        results := #[typeUse 2 10]
        predicates := #[.associatedTypeEq {
          trait := { namespaceId := ⟨0⟩, name := ⟨3⟩ } } ⟨0⟩ ⟨2⟩] }
      body := .structured ⟨0⟩
      origin := ⟨0⟩
      alignment := ⟨0⟩ }] }
  { fixture with
    tables := { fixture.tables with
      namespaces := fixture.tables.namespaces.push {
        segments := #["test", "Declarations", "Consumer"] }
      names := fixture.tables.names ++ #[
        { namespaceId := ⟨0⟩, name := "HasItem" },
        { namespaceId := ⟨0⟩, name := "Item" },
        { namespaceId := ⟨1⟩, name := "consumeItem" }] }
    namespaces := #[{ first with
      associatedItems := #[{
        id := ⟨0⟩, loc := ⟨10⟩, owner := ⟨0⟩, name := ⟨4⟩,
        kind := .type #[] }]
      traits := #[{
        id := ⟨0⟩, loc := ⟨10⟩, name := ⟨3⟩,
        associatedItems := #[⟨0⟩] }] }, second] }

#guard (prepare? crossNamespaceAssociatedEqualityFixture).isSome

private def crossNamespaceImplementationFixture : RawUnit :=
  let first := fixture.namespaces[0]!
  let second : RawNamespace := {
    loc := ⟨10⟩
    identity := ⟨1⟩
    profile := some .rust
    imports := #[⟨0⟩]
    expressions := #[{
      loc := ⟨10⟩, typeId := ⟨2⟩, kind := .value (.integer 42) }]
    implementations := #[{
      id := ⟨0⟩
      loc := ⟨10⟩
      trait := { trait := { namespaceId := ⟨0⟩, name := ⟨3⟩ } }
      target := typeUse 1 10
      bindings := #[{ loc := ⟨10⟩, item := ⟨0⟩, value := .constant ⟨0⟩ }] }] }
  { fixture with
    tables := { fixture.tables with
      namespaces := fixture.tables.namespaces.push {
        segments := #["test", "Declarations", "Consumer"] }
      names := fixture.tables.names ++ #[
        { namespaceId := ⟨0⟩, name := "HasAnswer" },
        { namespaceId := ⟨0⟩, name := "ANSWER" }] }
    namespaces := #[{ first with
      associatedItems := #[{
        id := ⟨0⟩, loc := ⟨10⟩, owner := ⟨0⟩, name := ⟨4⟩,
        kind := .constant (typeUse 2 10) }]
      traits := #[{
        id := ⟨0⟩, loc := ⟨10⟩, name := ⟨3⟩,
        associatedItems := #[⟨0⟩] }] }, second] }

#guard (prepare? crossNamespaceImplementationFixture).isSome

private def badCrossNamespaceImplementationValueFixture : RawUnit :=
  let first := crossNamespaceImplementationFixture.namespaces[0]!
  let second := crossNamespaceImplementationFixture.namespaces[1]!
  { crossNamespaceImplementationFixture with namespaces := #[first, { second with
      expressions := #[{ second.expressions[0]! with
        typeId := ⟨1⟩, kind := .value (.bool true) }] }] }

#guard validationHasDiagnosticAt badCrossNamespaceImplementationValueFixture
  "LIR-SEMANTIC-TYPE" ⟨10⟩

private def missingCrossNamespaceImplementationBindingFixture : RawUnit :=
  let first := crossNamespaceImplementationFixture.namespaces[0]!
  let second := crossNamespaceImplementationFixture.namespaces[1]!
  { crossNamespaceImplementationFixture with namespaces := #[first, { second with
      implementations := #[{ second.implementations[0]! with bindings := #[] }] }] }

#guard match validate #[schema] missingCrossNamespaceImplementationBindingFixture with
  | .error diagnostics => diagnostics.any (fun diagnostic =>
      diagnostic.code == "LIR-ASSOCIATED-BINDING-MISSING")
  | .ok _ => false

private def crossNamespaceGenericConstantFixture : RawUnit :=
  let typeArgument : GenericArgument := .typeArg (typeUse 1 5)
  let binder : GenericBinder := {
    loc := ⟨5⟩, name := "T", kind := .typeArg, abilities := #[.copy] }
  let first := fixture.namespaces[0]!
  let second : RawNamespace := {
    loc := ⟨6⟩
    identity := ⟨1⟩
    profile := some .rust
    imports := #[⟨0⟩]
    expressions := #[{
      loc := ⟨6⟩, typeId := ⟨3⟩,
      kind := .constant { namespaceId := ⟨0⟩, name := ⟨5⟩ } }]
    functions := #[{
      loc := ⟨6⟩
      name := ⟨6⟩
      profile := .rust
      signature := { results := #[typeUse 3 6] }
      body := .structured ⟨0⟩
      origin := ⟨0⟩
      alignment := ⟨0⟩ }] }
  { fixture with
    tables := { fixture.tables with
      types := fixture.tables.types ++ #[
        .nominal ⟨3⟩ #[typeArgument], .typeParameter 0]
      namespaces := fixture.tables.namespaces.push {
        segments := #["test", "Declarations", "Consumer"] }
      names := fixture.tables.names ++ #[
        { namespaceId := ⟨0⟩, name := "Box" },
        { namespaceId := ⟨0⟩, name := "value" },
        { namespaceId := ⟨0⟩, name := "boxed" },
        { namespaceId := ⟨1⟩, name := "useBox" }] }
    namespaces := #[{ first with
      expressions := (first.expressions.push {
        loc := ⟨5⟩, typeId := ⟨1⟩, kind := .value (.bool true) }).push {
        loc := ⟨5⟩, typeId := ⟨3⟩,
        kind := .operation (.call (.constructor {
          namespaceId := ⟨0⟩, name := ⟨3⟩ })) #[typeArgument] #[⟨2⟩] }
      constants := first.constants.push {
        loc := ⟨5⟩, name := ⟨5⟩, type := typeUse 3 5, value := ⟨3⟩ }
      structs := #[{
        loc := ⟨5⟩, name := ⟨3⟩, generics := #[binder],
        fields := #[{ loc := ⟨5⟩, name := ⟨4⟩, type := typeUse 4 5 }] }] }, second] }

#guard match prepare? crossNamespaceGenericConstantFixture with
  | some executable => match Interpreter.run executable 24
      { namespaceId := ⟨1⟩, functionId := ⟨0⟩ } #[] with
    | .ok (_, { value := .returned #[.nominal _ none #[.bool true]], .. }) => true
    | _ => false
  | none => false

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

end LeanerIR.Tests.Declarations
