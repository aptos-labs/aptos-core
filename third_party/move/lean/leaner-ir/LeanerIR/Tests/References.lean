-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Interpreter
import LeanerIR.Validation.Check

namespace LeanerIR.Tests.References

open LeanerIR
open LeanerIR.Import
open LeanerIR.SemanticOperations
open LeanerIR.Validation

private def config : ProfileConfig := { profile := .rust, name := "rust-test" }
private def schema : ProfileSchema := { profile := .rust, name := "rust-test" }
private def semantics : SemanticProfile := {
  profile := .rust, name := "rust-test", classify := fun _ _ => none }

private def typeUse (typeId loc : Nat) : TypeUse := { typeId := ⟨typeId⟩, loc := ⟨loc⟩ }

private def fixture : RawUnit where
  tables := {
    files := #[{ name := "references.rs" }]
    locations := (Array.range 20).map fun index => {
      primary := some { file := ⟨0⟩, startByte := index, endByte := index + 1 } }
    origins := #[{ kind := .rustMir, location := ⟨0⟩ }]
    alignments := #[{ source := ⟨0⟩, trust := .checked, description := "reference fixture" }]
    lifetimes := #[{ kind := .local, loc := ⟨0⟩ }]
    types := #[
      .unit,
      .integer (.bits 64) true,
      .reference { profile := .rust, kind := .mutable, referent := ⟨1⟩, lifetime := ⟨0⟩ }]
    namespaces := #[{ segments := #["test", "References"] }]
    names := #[
      { namespaceId := ⟨0⟩, name := "mutate" },
      { namespaceId := ⟨0⟩, name := "identity" }] }
  profiles := #[config]
  namespaces := #[{
    loc := ⟨0⟩
    identity := ⟨0⟩
    profile := some .rust
    expressions := #[
      { loc := ⟨0⟩, typeId := ⟨1⟩, kind := .value (.integer 7) },
      { loc := ⟨1⟩, typeId := ⟨2⟩, kind := .operation (.borrow .mutable ⟨0⟩) #[] #[] },
      { loc := ⟨2⟩, typeId := ⟨1⟩, kind := .value (.integer 9) },
      { loc := ⟨3⟩, typeId := ⟨0⟩, kind := .operation (.write ⟨2⟩) #[] #[⟨2⟩] },
      { loc := ⟨4⟩, typeId := ⟨1⟩, kind := .operation (.read ⟨2⟩) #[] #[] },
      { loc := ⟨5⟩, typeId := ⟨1⟩, kind := .block #[⟨3⟩] (some ⟨4⟩) },
      { loc := ⟨6⟩, typeId := ⟨1⟩, kind := .letDecl ⟨1⟩ (some ⟨8⟩) ⟨5⟩ },
      { loc := ⟨7⟩, typeId := ⟨1⟩, kind := .letDecl ⟨0⟩ (some ⟨0⟩) ⟨6⟩ },
      { loc := ⟨11⟩, typeId := ⟨2⟩, kind := .operation
          (.call (.function { namespaceId := ⟨0⟩, name := ⟨1⟩ })) #[] #[⟨1⟩] },
      { loc := ⟨12⟩, typeId := ⟨2⟩,
        kind := .operation (.move ⟨0⟩) #[] #[] }]
    patterns := #[
      { loc := ⟨8⟩, typeId := ⟨1⟩, kind := .variable ⟨0⟩ },
      { loc := ⟨9⟩, typeId := ⟨2⟩, kind := .variable ⟨1⟩ }]
    places := #[.localVar ⟨0⟩, .localVar ⟨1⟩, .deref ⟨1⟩]
    functions := #[
      {
        loc := ⟨10⟩
        name := ⟨0⟩
        profile := .rust
        signature := { results := #[typeUse 1 10] }
        body := .structured ⟨7⟩
        origin := ⟨0⟩
        alignment := ⟨0⟩
        locals := #[
          { id := ⟨0⟩, name := "value", type := typeUse 1 10, mutable := true, loc := ⟨10⟩ },
          { id := ⟨1⟩, name := "reference", type := typeUse 2 10, mutable := false, loc := ⟨10⟩ }]
      },
      {
        loc := ⟨13⟩
        name := ⟨1⟩
        profile := .rust
        signature := {
          parameters := #[{
            name := "reference", typeUse := typeUse 2 13 }]
          results := #[typeUse 2 13] }
        body := .structured ⟨9⟩
        origin := ⟨0⟩
        alignment := ⟨0⟩
        locals := #[{
          id := ⟨0⟩, name := "reference", type := typeUse 2 13,
          mutable := false, loc := ⟨13⟩ }]
      }] }]

private def executable? : Option ExecutableUnit := do
  let checked ← (validate #[schema] fixture).toOption
  (prepareExecution #[semantics] checked).toOption

private def verifiable? : Option VerifiableUnit := do
  let checked ← (validate #[schema] fixture).toOption
  (prepareVerification #[semantics] checked).toOption

#guard match executable? with
  | none => false
  | some executable =>
      executable.borrowCertificates == #[
        {
          namespaceId := ⟨0⟩
          functionId := ⟨0⟩
          root := ⟨7⟩
          parameters := #[]
          loans := #[{
            expression := ⟨1⟩
            lifetime := ⟨0⟩
            holders := #[⟨1⟩]
            deaths := #[{ anchor := ⟨7⟩ }]
          }]
          lifetimeRelations := #[{ longer := ⟨0⟩, shorter := ⟨0⟩ }]
        },
        {
          namespaceId := ⟨0⟩
          functionId := ⟨1⟩
          root := ⟨9⟩
          parameters := #[{
            localId := ⟨0⟩, kind := .mutable, lifetime := ⟨0⟩ }]
          loans := #[]
          lifetimeRelations := #[{ longer := ⟨0⟩, shorter := ⟨0⟩ }]
        }]

#guard match executable?, verifiable? with
  | some executable, some verifiable =>
      verifiable.borrowCertificates == executable.borrowCertificates
  | _, _ => false

/-! A loan that dies mid-function, with the lender consumed afterwards: the
analysis records the death, preparation materializes an `endLoan` marker,
and the marker is value-transparent to execution. -/

private def markerFixture : RawUnit where
  tables := {
    files := #[{ name := "marker.rs" }]
    locations := (Array.range 20).map fun index => {
      primary := some { file := ⟨0⟩, startByte := index, endByte := index + 1 } }
    origins := #[{ kind := .rustMir, location := ⟨0⟩ }]
    alignments := #[{ source := ⟨0⟩, trust := .checked, description := "marker fixture" }]
    lifetimes := #[{ kind := .local, loc := ⟨0⟩ }]
    types := #[
      .unit,
      .integer (.bits 64) true,
      .reference { profile := .rust, kind := .mutable, referent := ⟨1⟩, lifetime := ⟨0⟩ }]
    namespaces := #[{ segments := #["test", "Marker"] }]
    names := #[{ namespaceId := ⟨0⟩, name := "mutate_then_move" }] }
  profiles := #[config]
  namespaces := #[{
    loc := ⟨0⟩
    identity := ⟨0⟩
    profile := some .rust
    expressions := #[
      { loc := ⟨0⟩, typeId := ⟨1⟩, kind := .value (.integer 7) },
      { loc := ⟨1⟩, typeId := ⟨2⟩, kind := .operation (.borrow .mutable ⟨0⟩) #[] #[] },
      { loc := ⟨2⟩, typeId := ⟨1⟩, kind := .value (.integer 9) },
      { loc := ⟨3⟩, typeId := ⟨0⟩, kind := .operation (.write ⟨2⟩) #[] #[⟨2⟩] },
      { loc := ⟨4⟩, typeId := ⟨1⟩, kind := .operation (.move ⟨0⟩) #[] #[] },
      { loc := ⟨5⟩, typeId := ⟨1⟩, kind := .block #[⟨3⟩] (some ⟨4⟩) },
      { loc := ⟨6⟩, typeId := ⟨1⟩, kind := .letDecl ⟨1⟩ (some ⟨1⟩) ⟨5⟩ },
      { loc := ⟨7⟩, typeId := ⟨1⟩, kind := .letDecl ⟨0⟩ (some ⟨0⟩) ⟨6⟩ }]
    patterns := #[
      { loc := ⟨8⟩, typeId := ⟨1⟩, kind := .variable ⟨0⟩ },
      { loc := ⟨9⟩, typeId := ⟨2⟩, kind := .variable ⟨1⟩ }]
    places := #[.localVar ⟨0⟩, .localVar ⟨1⟩, .deref ⟨1⟩]
    functions := #[
      {
        loc := ⟨10⟩
        name := ⟨0⟩
        profile := .rust
        signature := { results := #[typeUse 1 10] }
        body := .structured ⟨7⟩
        origin := ⟨0⟩
        alignment := ⟨0⟩
        locals := #[
          { id := ⟨0⟩, name := "value", type := typeUse 1 10, mutable := true, loc := ⟨10⟩ },
          { id := ⟨1⟩, name := "reference", type := typeUse 2 10, mutable := false, loc := ⟨10⟩ }]
      }] }]

private def markerExecutable? : Option ExecutableUnit := do
  let checked ← (validate #[schema] markerFixture).toOption
  (prepareExecution #[semantics] checked).toOption

-- The analysis records the death before the lender's consuming move.
#guard match (validate #[schema] markerFixture).toOption with
  | some checked =>
      (checked.borrowCertificates.map BorrowCertificate.loans) == #[#[{
        expression := ⟨1⟩, lifetime := ⟨0⟩, holders := #[⟨1⟩]
        deaths := #[{ anchor := ⟨4⟩, before := true }] }]]
  | none => false

-- Preparation materializes exactly one endLoan marker in the arena, and the
-- validated unit itself stays marker-free.
#guard match (validate #[schema] markerFixture).toOption, markerExecutable? with
  | some checked, some executable =>
      let markers (ns : ValidatedNamespace) :=
        ns.expressions.filter fun expression =>
          match expression.kind with
          | .operation (.reference (.endLoan _)) _ _ _ => true
          | _ => false
      (checked.namespaces.map fun ns => (markers ns).size) == #[0] &&
        (executable.unit.namespaces.map fun ns => (markers ns).size) == #[1]
  | _, _ => false

-- The marker is value-transparent: the mutation through the reference is
-- observed by the moved-out lender.
#guard match markerExecutable? with
  | some executable =>
      match Interpreter.run executable 32 { namespaceId := ⟨0⟩, functionId := ⟨0⟩ } #[] with
      | .ok (_, { value := .returned #[.integer 9], .. }) => true
      | _ => false
  | none => false

private def lifetimeClosureFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  let function := ns.functions[0]!
  { fixture with
    tables := { fixture.tables with lifetimes := fixture.tables.lifetimes ++ #[
      { kind := .local, loc := ⟨0⟩ }, { kind := .local, loc := ⟨0⟩ }] }
    namespaces := #[{ ns with functions := ns.functions.set! 0 {
      function with signature := { function.signature with predicates := #[
        .lifetimeOutlives ⟨0⟩ ⟨1⟩, .lifetimeOutlives ⟨1⟩ ⟨2⟩] } } }] }

#guard match validate #[schema] lifetimeClosureFixture with
  | .error _ => false
  | .ok checked => match prepareExecution #[semantics] checked with
      | .error _ => false
      | .ok executable => match executable.borrowCertificates.find?
          (·.functionId == ⟨0⟩) with
        | some certificate => certificate.lifetimeRelations.contains {
            longer := ⟨0⟩, shorter := ⟨2⟩ }
        | none => false

#guard match executable? with
  | none => false
  | some executable => match LeanerIR.Interpreter.run executable 32
      { namespaceId := ⟨0⟩, functionId := ⟨0⟩ } #[] with
    | .ok (state, outcome) =>
        -- Frame-local loans die with the frame: nothing is exported.
        outcome.value == .returned #[.integer 9] && state.pending.isEmpty
    | .error _ => false

private def valueBorrowFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  let localExpression : Expr := {
    loc := ⟨1⟩, typeId := ⟨1⟩, kind := .localVar ⟨0⟩ }
  let borrowOperation : Operation := .reference (.borrow .mutable)
  { fixture with namespaces := #[{ ns with
      expressions := (ns.expressions.push localExpression).set! 1 {
        ns.expressions[1]! with kind :=
          (ExprKind.operation borrowOperation #[] #[⟨10⟩]) } }] }

#guard match validate #[schema] valueBorrowFixture with
  | .error _ => false
  | .ok checked => match checked.namespaces[0]? with
    | some ns => match ns.expressions[1]? with
      | some expression => match expression.kind with
        | .operation (.borrow .mutable place) instantiations arguments _ =>
            instantiations.isEmpty && arguments.isEmpty &&
              place == ⟨0⟩ && ns.places.size == 3 &&
              ns.places[place.index]? == some (.localVar ⟨0⟩)
        | _ => false
      | none => false
    | _ => false

private def valueBorrowWithoutLocalPlaceFixture : RawUnit :=
  let ns := valueBorrowFixture.namespaces[0]!
  -- `identity` rebinds place 0 to a fresh `extra` local it initializes with a
  -- `let`, so no `.localVar ⟨0⟩` place remains and definite initialization
  -- accepts both functions.
  { valueBorrowFixture with namespaces := #[{ ns with
      places := ns.places.set! 0 (.localVar ⟨1⟩)
      expressions := ns.expressions ++ #[
        { loc := ⟨13⟩, typeId := ⟨2⟩, kind := .localVar ⟨0⟩ },
        { loc := ⟨13⟩, typeId := ⟨2⟩, kind := .letDecl ⟨1⟩ (some ⟨11⟩) ⟨9⟩ }]
      functions := ns.functions.set! 1 { ns.functions[1]! with
        body := .structured ⟨12⟩
        locals := ns.functions[1]!.locals.push {
          id := ⟨1⟩, name := "extra", type := typeUse 2 13, loc := ⟨13⟩ } } }] }
#guard match validate #[schema] valueBorrowWithoutLocalPlaceFixture with
  | .error _ => false
  | .ok checked => match checked.namespaces[0]? with
    | some ns => match ns.expressions[1]? with
      | some expression => match expression.kind with
        | .operation (.borrow .mutable place) #[] #[] _ =>
            place == ⟨3⟩ && ns.places.size == 4 &&
              ns.places[place.index]? == some (.localVar ⟨0⟩)
        | _ => false
      | none => false
    | none => false

private def projectedValueBorrowFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  let nominalType : Ty := .nominal ⟨2⟩ #[]
  let ownerReference : Ty := .reference {
    profile := .rust, kind := .mutable, referent := ⟨3⟩, lifetime := ⟨0⟩ }
  { fixture with
    tables := { fixture.tables with
      types := fixture.tables.types ++ #[nominalType, ownerReference]
      names := fixture.tables.names ++ #[
        { namespaceId := ⟨0⟩, name := "Cell" },
        { namespaceId := ⟨0⟩, name := "value" }] }
    namespaces := #[{
      ns with
      expressions := #[
        { loc := ⟨0⟩, typeId := ⟨4⟩, kind := .localVar ⟨0⟩ },
        { loc := ⟨1⟩, typeId := ⟨1⟩, kind := .operation
            (.data (.select { namespaceId := ⟨0⟩, name := ⟨2⟩ } "value"))
            #[.typeArg (typeUse 3 1)] #[⟨0⟩] },
        { loc := ⟨2⟩, typeId := ⟨2⟩, kind := .operation
            (.reference (.borrow .mutable)) #[] #[⟨1⟩] }]
      patterns := #[]
      places := #[]
      structs := #[{
        loc := ⟨0⟩, name := ⟨2⟩,
        fields := #[{ loc := ⟨0⟩, name := ⟨3⟩, type := typeUse 1 0 }] }]
      functions := #[{
        loc := ⟨0⟩
        name := ⟨0⟩
        profile := .rust
        signature := {
          parameters := #[{ name := "cell", typeUse := typeUse 4 0 }]
          results := #[typeUse 2 0] }
        body := .structured ⟨2⟩
        origin := ⟨0⟩
        alignment := ⟨0⟩
        locals := #[{
          id := ⟨0⟩, name := "cell", type := typeUse 4 0,
          mutable := false, loc := ⟨0⟩ }] }] }] }
#guard match validate #[schema] projectedValueBorrowFixture with
  | .error _ => false
  | .ok checked => match checked.namespaces[0]? with
    | some ns => match ns.expressions[2]? with
      | some expression => match expression.kind with
        | .operation (.borrow .mutable place) #[] #[] _ =>
            place == ⟨2⟩ && ns.places == #[
              .localVar ⟨0⟩, .deref ⟨0⟩, .field ⟨1⟩ ⟨⟨0⟩, ⟨2⟩⟩ ⟨3⟩]
        | _ => false
      | none => false
    | none => false

#guard match validate #[schema] valueBorrowFixture with
  | .error _ => false
  | .ok checked => match prepareExecution #[semantics] checked with
      | .error _ => false
      | .ok executable => match LeanerIR.Interpreter.run executable 32
          { namespaceId := ⟨0⟩, functionId := ⟨0⟩ } #[] with
        | .ok (_, outcome) => outcome.value == .returned #[.integer 9]
        | .error _ => false

private def nonPlaceValueBorrowFixture : RawUnit :=
  let ns := valueBorrowFixture.namespaces[0]!
  let expression := ns.expressions[1]!
  let borrowOperation : Operation := .reference (.borrow .mutable)
  { valueBorrowFixture with namespaces := #[{ ns with
      expressions := ns.expressions.set! 1 { expression with kind :=
        (ExprKind.operation borrowOperation #[] #[⟨0⟩]) } }] }

#guard match validate #[schema] nonPlaceValueBorrowFixture with
  | .error _ => false
  | .ok checked => match prepareExecution #[semantics] checked with
      | .error diagnostics => diagnostics.any fun diagnostic =>
          diagnostic.code == "LIR-EXEC-UNSUPPORTED" && diagnostic.primary == some ⟨1⟩
      | .ok _ => false

private def subsliceFixture (start stop : Nat) (fromEnd : Bool)
    (resultLength : Nat) : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with
    tables := { fixture.tables with types := fixture.tables.types ++ #[
      .vector ⟨1⟩ (some (.integer 4)),
      .vector ⟨1⟩ (some (.integer (Int.ofNat resultLength)))] }
    namespaces := #[{
      ns with
      expressions := #[
        { loc := ⟨0⟩, typeId := ⟨3⟩,
          kind := .value (.vector #[.integer 1, .integer 2, .integer 3, .integer 4]) },
        { loc := ⟨1⟩, typeId := ⟨0⟩, kind := .assign ⟨0⟩ ⟨0⟩ },
        { loc := ⟨2⟩, typeId := ⟨4⟩,
          kind := .operation (.read ⟨1⟩) #[] #[] },
        { loc := ⟨3⟩, typeId := ⟨4⟩, kind := .block #[⟨1⟩] (some ⟨2⟩) }]
      places := #[.localVar ⟨0⟩, .subslice ⟨0⟩ start stop fromEnd]
      functions := #[{
        ns.functions[0]! with
        signature := { results := #[typeUse 4 10] }
        body := .structured ⟨3⟩
        locals := #[{
          id := ⟨0⟩, name := "values", type := typeUse 3 10,
          mutable := true, loc := ⟨10⟩ }] }] }] }

private def runSubslice? (raw : RawUnit) : Option (Array RuntimeValue) := do
  let checked ← (validate #[schema] raw).toOption
  let executable ← (prepareExecution #[semantics] checked).toOption
  let (_, outcome) ← (LeanerIR.Interpreter.run executable 16
    { namespaceId := ⟨0⟩, functionId := ⟨0⟩ } #[]).toOption
  let .returned #[.vector values] := outcome.value | none
  some values

#guard runSubslice? (subsliceFixture 1 3 false 2) ==
  some #[.integer 2, .integer 3]

#guard runSubslice? (subsliceFixture 1 1 true 2) ==
  some #[.integer 2, .integer 3]

private def movedSubsliceFixture : RawUnit :=
  let ns := (subsliceFixture 1 3 false 2).namespaces[0]!
  { subsliceFixture 1 3 false 2 with namespaces := #[{ ns with
      expressions := ns.expressions.set! 2 {
        ns.expressions[2]! with kind := .operation (.move ⟨1⟩) #[] #[] } }] }

#guard runSubslice? movedSubsliceFixture == some #[.integer 2, .integer 3]

private def movedSubsliceAggregateReadFixture : RawUnit :=
  let ns := movedSubsliceFixture.namespaces[0]!
  { movedSubsliceFixture with namespaces := #[{ ns with
      expressions := ns.expressions ++ #[
        { loc := ⟨4⟩, typeId := ⟨3⟩, kind := .operation (.read ⟨0⟩) #[] #[] },
        { loc := ⟨5⟩, typeId := ⟨3⟩,
          kind := .block #[⟨1⟩, ⟨2⟩] (some ⟨4⟩) }]
      functions := #[{ ns.functions[0]! with
        signature := { results := #[typeUse 3 10] }
        body := .structured ⟨5⟩ }] }] }

#guard match validate #[schema] movedSubsliceAggregateReadFixture with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-INITIALIZATION" &&
        diagnostic.primary == some ⟨4⟩
  | .ok _ => false

#guard match validate #[schema] (subsliceFixture 3 1 false 0) with
  | .error diagnostics => diagnostics.any (·.code == "LIR-PLACE-SUBSLICE")
  | .ok _ => false

#guard match validate #[schema] (subsliceFixture 3 2 true 0) with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-TYPE" && diagnostic.primary == some ⟨2⟩
  | .ok _ => false

private def preparationHasCode (raw : RawUnit) (code : String) : Bool :=
  match validate #[schema] raw with
  | .error _ => false
  | .ok checked => match prepareExecution #[semantics] checked with
    | .error diagnostics => diagnostics.any (·.code == code)
    | .ok _ => false

private def validationHasCode (raw : RawUnit) (code : String) : Bool :=
  match validate #[schema] raw with
  | .error diagnostics => diagnostics.any (·.code == code)
  | .ok _ => false

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

private def conflictingLocalReadFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with namespaces := #[{
      ns with expressions := ns.expressions.set! 4 {
        ns.expressions[4]! with kind := .localVar ⟨0⟩ } }] }

#guard match validate #[schema] conflictingLocalReadFixture with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

private def liveConflictingLocalReadFixture : RawUnit :=
  let ns := conflictingLocalReadFixture.namespaces[0]!
  { conflictingLocalReadFixture with namespaces := #[{
      ns with
      expressions := ns.expressions.set! 5 {
        ns.expressions[5]! with kind := .block #[⟨4⟩, ⟨3⟩] (some ⟨0⟩) } }] }

#guard preparationHasDiagnosticAt liveConflictingLocalReadFixture
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨4⟩

private def consumedReferenceHolderFixture (drop : Bool) : RawUnit :=
  let ns := conflictingLocalReadFixture.namespaces[0]!
  let consume : Expr := {
    loc := ⟨14⟩
    typeId := if drop then ⟨0⟩ else ⟨2⟩
    kind := .operation (if drop then .drop ⟨1⟩ else .move ⟨1⟩) #[] #[] }
  { conflictingLocalReadFixture with namespaces := #[{
      ns with expressions := (ns.expressions.push consume).set! 5 {
        ns.expressions[5]! with kind := .block #[⟨10⟩] (some ⟨4⟩) } }] }

#guard match validate #[schema] (consumedReferenceHolderFixture false) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard match validate #[schema] (consumedReferenceHolderFixture true) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

private def wildcardDiscardedLoanFixture : RawUnit :=
  let ns := conflictingLocalReadFixture.namespaces[0]!
  { conflictingLocalReadFixture with namespaces := #[{
      ns with
      expressions := ns.expressions.set! 5 {
        ns.expressions[5]! with kind := .block #[] (some ⟨4⟩) }
      patterns := ns.patterns.set! 1 {
        ns.patterns[1]! with kind := .wildcard } }] }

#guard match validate #[schema] wildcardDiscardedLoanFixture with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

private def wildcardAssignedLoanFixture : RawUnit :=
  let ns := conflictingLocalReadFixture.namespaces[0]!
  let discard : Expr := {
    loc := ⟨14⟩, typeId := ⟨0⟩, kind := .assignPattern ⟨2⟩ ⟨1⟩ }
  let body : Expr := {
    loc := ⟨15⟩, typeId := ⟨1⟩, kind := .block #[⟨10⟩] (some ⟨4⟩) }
  { conflictingLocalReadFixture with namespaces := #[{
      ns with
      expressions := (ns.expressions ++ #[discard, body]).set! 7 {
        ns.expressions[7]! with kind := .letDecl ⟨0⟩ (some ⟨0⟩) ⟨11⟩ }
      patterns := ns.patterns.push {
        loc := ⟨14⟩, typeId := ⟨2⟩, kind := .wildcard } }] }

#guard match validate #[schema] wildcardAssignedLoanFixture with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

private def borrowedIndexLocalFixture : RawUnit :=
  let ns := conflictingLocalReadFixture.namespaces[0]!
  let vector : Expr := {
    loc := ⟨14⟩, typeId := ⟨3⟩,
    kind := .operation (.primitive .vector) #[] #[⟨0⟩, ⟨2⟩] }
  let indexedRead : Expr := {
    loc := ⟨15⟩, typeId := ⟨1⟩, kind := .operation (.read ⟨4⟩) #[] #[] }
  let holderRead : Expr := {
    loc := ⟨16⟩, typeId := ⟨1⟩, kind := .operation (.read ⟨2⟩) #[] #[] }
  let body : Expr := {
    loc := ⟨17⟩, typeId := ⟨1⟩, kind := .block #[⟨11⟩] (some ⟨12⟩) }
  let vectorLet : Expr := {
    loc := ⟨18⟩, typeId := ⟨1⟩, kind := .letDecl ⟨2⟩ (some ⟨10⟩) ⟨13⟩ }
  { conflictingLocalReadFixture with
    tables := { fixture.tables with
      types := fixture.tables.types.push (.vector ⟨1⟩ (some (.integer 2))) }
    namespaces := #[{
      ns with
      expressions := (ns.expressions ++ #[vector, indexedRead, holderRead, body, vectorLet])
        |>.set! 6 { ns.expressions[6]! with
          kind := .letDecl ⟨1⟩ (some ⟨8⟩) ⟨14⟩ }
      patterns := ns.patterns.push {
        loc := ⟨18⟩, typeId := ⟨3⟩, kind := .variable ⟨2⟩ }
      places := ns.places ++ #[.localVar ⟨2⟩, .index ⟨3⟩ ⟨4⟩]
      functions := ns.functions.set! 0 {
        ns.functions[0]! with locals := ns.functions[0]!.locals.push {
          id := ⟨2⟩, name := "values", type := typeUse 3 18,
          mutable := false, loc := ⟨18⟩ } } }] }

#guard preparationHasDiagnosticAt borrowedIndexLocalFixture
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨15⟩

private def projectedIndexBorrowFixture (secondIndex : Nat) (dynamic : Bool) : RawUnit :=
  let ns := fixture.namespaces[0]!
  let vectorType : Ty := .vector ⟨1⟩ (some (.integer 2))
  let secondIndexExpression : Expr := if dynamic then
    { loc := ⟨3⟩, typeId := ⟨1⟩, kind := .localVar ⟨2⟩ }
  else
    { loc := ⟨3⟩, typeId := ⟨1⟩, kind := .value (.integer secondIndex) }
  { fixture with
    tables := { fixture.tables with types := fixture.tables.types.push vectorType }
    namespaces := #[{
      ns with
      expressions := #[
        { loc := ⟨0⟩, typeId := ⟨3⟩,
          kind := .value (.vector #[.integer 7, .integer 9]) },
        { loc := ⟨1⟩, typeId := ⟨1⟩, kind := .value (.integer 0) },
        { loc := ⟨2⟩, typeId := ⟨1⟩, kind := .value (.integer secondIndex) },
        secondIndexExpression,
        { loc := ⟨4⟩, typeId := ⟨2⟩,
          kind := .operation (.borrow .mutable ⟨1⟩) #[] #[] },
        { loc := ⟨5⟩, typeId := ⟨2⟩,
          kind := .operation (.borrow .mutable ⟨2⟩) #[] #[] },
        { loc := ⟨6⟩, typeId := ⟨1⟩,
          kind := .operation (.read ⟨4⟩) #[] #[] },
        { loc := ⟨7⟩, typeId := ⟨1⟩,
          kind := .block #[⟨5⟩] (some ⟨6⟩) },
        { loc := ⟨8⟩, typeId := ⟨1⟩,
          kind := .letDecl ⟨1⟩ (some ⟨4⟩) ⟨7⟩ },
        { loc := ⟨9⟩, typeId := ⟨1⟩,
          kind := .letDecl ⟨0⟩ (some ⟨0⟩) ⟨8⟩ },
        { loc := ⟨10⟩, typeId := ⟨1⟩,
          kind := .letDecl ⟨2⟩ (some ⟨2⟩) ⟨9⟩ }]
      patterns := #[
        { loc := ⟨9⟩, typeId := ⟨3⟩, kind := .variable ⟨0⟩ },
        { loc := ⟨8⟩, typeId := ⟨2⟩, kind := .variable ⟨1⟩ },
        { loc := ⟨10⟩, typeId := ⟨1⟩, kind := .variable ⟨2⟩ }]
      places := #[
        .localVar ⟨0⟩, .index ⟨0⟩ ⟨1⟩,
        .index ⟨0⟩ (if dynamic then ⟨3⟩ else ⟨2⟩),
        .localVar ⟨1⟩, .deref ⟨3⟩]
      functions := #[{
        ns.functions[0]! with
        signature := { results := #[typeUse 1 10] }
        body := .structured ⟨10⟩
        locals := #[
          { id := ⟨0⟩, name := "values", type := typeUse 3 9,
            mutable := true, loc := ⟨9⟩ },
          { id := ⟨1⟩, name := "reference", type := typeUse 2 8,
            mutable := false, loc := ⟨8⟩ },
          { id := ⟨2⟩, name := "index", type := typeUse 1 10,
            mutable := false, loc := ⟨10⟩ }] }] }] }

#guard match validate #[schema] (projectedIndexBorrowFixture 1 false) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard preparationHasDiagnosticAt (projectedIndexBorrowFixture 0 false)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨5⟩

#guard preparationHasDiagnosticAt (projectedIndexBorrowFixture 1 true)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨5⟩

private def projectedSubsliceBorrowFixture (secondStart secondStop : Nat) : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with
    tables := { fixture.tables with types := fixture.tables.types ++ #[
      .vector ⟨1⟩ (some (.integer 4)),
      .vector ⟨1⟩ (some (.integer 2)),
      .reference {
        profile := .rust, kind := .mutable, referent := ⟨4⟩, lifetime := ⟨0⟩ }] }
    namespaces := #[{
      ns with
      expressions := #[
        { loc := ⟨0⟩, typeId := ⟨3⟩,
          kind := .value (.vector #[
            .integer 1, .integer 2, .integer 3, .integer 4]) },
        { loc := ⟨1⟩, typeId := ⟨5⟩,
          kind := .operation (.borrow .mutable ⟨1⟩) #[] #[] },
        { loc := ⟨2⟩, typeId := ⟨5⟩,
          kind := .operation (.borrow .mutable ⟨2⟩) #[] #[] },
        { loc := ⟨3⟩, typeId := ⟨4⟩,
          kind := .operation (.read ⟨4⟩) #[] #[] },
        { loc := ⟨4⟩, typeId := ⟨4⟩,
          kind := .block #[⟨2⟩] (some ⟨3⟩) },
        { loc := ⟨5⟩, typeId := ⟨4⟩,
          kind := .letDecl ⟨1⟩ (some ⟨1⟩) ⟨4⟩ },
        { loc := ⟨6⟩, typeId := ⟨4⟩,
          kind := .letDecl ⟨0⟩ (some ⟨0⟩) ⟨5⟩ }]
      patterns := #[
        { loc := ⟨6⟩, typeId := ⟨3⟩, kind := .variable ⟨0⟩ },
        { loc := ⟨5⟩, typeId := ⟨5⟩, kind := .variable ⟨1⟩ }]
      places := #[
        .localVar ⟨0⟩, .subslice ⟨0⟩ 0 2 false,
        .subslice ⟨0⟩ secondStart secondStop false,
        .localVar ⟨1⟩, .deref ⟨3⟩]
      functions := #[{
        ns.functions[0]! with
        signature := { results := #[typeUse 4 10] }
        body := .structured ⟨6⟩
        locals := #[
          { id := ⟨0⟩, name := "values", type := typeUse 3 6,
            mutable := true, loc := ⟨6⟩ },
          { id := ⟨1⟩, name := "reference", type := typeUse 5 5,
            mutable := false, loc := ⟨5⟩ }] }] }] }

#guard match validate #[schema] (projectedSubsliceBorrowFixture 2 4) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard preparationHasDiagnosticAt (projectedSubsliceBorrowFixture 1 3)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨2⟩

private def projectedSubsliceIndexBorrowFixture (index : Nat) : RawUnit :=
  let raw := projectedSubsliceBorrowFixture 2 4
  let ns := raw.namespaces[0]!
  let indexExpression : Expr := {
    loc := ⟨7⟩, typeId := ⟨1⟩, kind := .value (.integer index) }
  let secondBorrow : Expr := {
    ns.expressions[2]! with
    typeId := ⟨2⟩
    kind := .operation (.borrow .mutable ⟨5⟩) #[] #[] }
  { raw with namespaces := #[{
      ns with
      expressions := (ns.expressions.push indexExpression).set! 2 secondBorrow
      places := ns.places.push (.index ⟨0⟩ ⟨7⟩) }] }

#guard match validate #[schema] (projectedSubsliceIndexBorrowFixture 2) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard preparationHasDiagnosticAt (projectedSubsliceIndexBorrowFixture 1)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨2⟩

private def subsliceLoanFixture (sameRange : Bool) : RawUnit :=
  let ns := fixture.namespaces[0]!
  let secondStart := if sameRange then 0 else 2
  let secondStop := if sameRange then 2 else 4
  let sliceReference : Ty := .reference {
    profile := .rust, kind := .mutable, referent := ⟨4⟩, lifetime := ⟨0⟩ }
  { fixture with
    tables := { fixture.tables with types := fixture.tables.types ++ #[
      .vector ⟨1⟩ (some (.integer 4)),
      .vector ⟨1⟩ (some (.integer 2)), sliceReference] }
    namespaces := #[{
      ns with
      expressions := #[
        { loc := ⟨0⟩, typeId := ⟨3⟩,
          kind := .value (.vector #[.integer 1, .integer 2, .integer 3, .integer 4]) },
        { loc := ⟨1⟩, typeId := ⟨5⟩,
          kind := .operation (.borrow .mutable ⟨1⟩) #[] #[] },
        { loc := ⟨2⟩, typeId := ⟨5⟩,
          kind := .operation (.borrow .mutable ⟨2⟩) #[] #[] },
        { loc := ⟨3⟩, typeId := ⟨4⟩,
          kind := .operation (.read ⟨4⟩) #[] #[] },
        { loc := ⟨4⟩, typeId := ⟨4⟩, kind := .block #[⟨2⟩] (some ⟨3⟩) },
        { loc := ⟨5⟩, typeId := ⟨4⟩, kind := .letDecl ⟨1⟩ (some ⟨1⟩) ⟨4⟩ },
        { loc := ⟨6⟩, typeId := ⟨4⟩, kind := .letDecl ⟨0⟩ (some ⟨0⟩) ⟨5⟩ }]
      patterns := #[
        { loc := ⟨6⟩, typeId := ⟨3⟩, kind := .variable ⟨0⟩ },
        { loc := ⟨5⟩, typeId := ⟨5⟩, kind := .variable ⟨1⟩ }]
      places := #[
        .localVar ⟨0⟩, .subslice ⟨0⟩ 0 2 false,
        .subslice ⟨0⟩ secondStart secondStop false,
        .localVar ⟨1⟩, .deref ⟨3⟩]
      functions := #[{
        ns.functions[0]! with
        signature := { results := #[typeUse 4 6] }
        body := .structured ⟨6⟩
        locals := #[
          { id := ⟨0⟩, name := "values", type := typeUse 3 6,
            mutable := true, loc := ⟨6⟩ },
          { id := ⟨1⟩, name := "first", type := typeUse 5 5,
            mutable := false, loc := ⟨5⟩ }] }] }] }

#guard match validate #[schema] (subsliceLoanFixture false) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard preparationHasDiagnosticAt (subsliceLoanFixture true)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨2⟩

private def branchLoanFixture (sameBranch : Bool) : RawUnit :=
  let ns := conflictingLocalReadFixture.namespaces[0]!
  let condition : Expr := { loc := ⟨14⟩, typeId := ⟨3⟩, kind := .value (.bool true) }
  let holderRead : Expr := {
    loc := ⟨15⟩, typeId := ⟨1⟩, kind := .operation (.read ⟨2⟩) #[] #[] }
  let liveThen : Expr := {
    loc := ⟨16⟩, typeId := ⟨1⟩, kind := .block #[⟨4⟩, ⟨11⟩] (some ⟨0⟩) }
  let branch : Expr := {
    loc := ⟨17⟩
    typeId := ⟨1⟩
    kind := if sameBranch then .ifElse ⟨10⟩ ⟨12⟩ (some ⟨0⟩)
      else .ifElse ⟨10⟩ ⟨4⟩ (some ⟨11⟩) }
  { conflictingLocalReadFixture with
    tables := { fixture.tables with types := fixture.tables.types.push .bool }
    namespaces := #[{
      ns with
      expressions := (ns.expressions ++ #[condition, holderRead, liveThen, branch]).set! 5 {
        ns.expressions[5]! with kind := .block #[] (some ⟨13⟩) } }] }

#guard match validate #[schema] (branchLoanFixture false) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard preparationHasDiagnosticAt (branchLoanFixture true)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨4⟩

private def matchLoanFixture (sameArm : Bool) : RawUnit :=
  let raw := branchLoanFixture sameArm
  let ns := raw.namespaces[0]!
  let trueBody : ExprId := if sameArm then ⟨12⟩ else ⟨4⟩
  let falseBody : ExprId := if sameArm then ⟨0⟩ else ⟨11⟩
  let truePattern : Pattern := { loc := ⟨14⟩, typeId := ⟨3⟩, kind := .literal (.bool true) }
  let wildcardPattern : Pattern := { loc := ⟨14⟩, typeId := ⟨3⟩, kind := .wildcard }
  let matchKind : ExprKind := .match_ ⟨10⟩ #[
    { pattern := ⟨2⟩, body := trueBody },
    { pattern := ⟨3⟩, body := falseBody }]
  { raw with namespaces := #[{
      ns with
      expressions := ns.expressions.set! 13 { ns.expressions[13]! with kind := matchKind }
      patterns := ns.patterns ++ #[truePattern, wildcardPattern] }] }

#guard match validate #[schema] (matchLoanFixture false) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard preparationHasDiagnosticAt (matchLoanFixture true)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨4⟩

private def branchContinuationLoanFixture : RawUnit :=
  let raw := branchLoanFixture false
  let ns := raw.namespaces[0]!
  { raw with namespaces := #[{ ns with expressions := ns.expressions.set! 5 {
      ns.expressions[5]! with kind := .block #[⟨13⟩, ⟨11⟩] (some ⟨0⟩) } }] }

#guard preparationHasDiagnosticAt branchContinuationLoanFixture
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨4⟩

private def matchContinuationLoanFixture : RawUnit :=
  let raw := matchLoanFixture false
  let ns := raw.namespaces[0]!
  { raw with namespaces := #[{ ns with expressions := ns.expressions.set! 5 {
      ns.expressions[5]! with kind := .block #[⟨13⟩, ⟨11⟩] (some ⟨0⟩) } }] }

#guard preparationHasDiagnosticAt matchContinuationLoanFixture
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨4⟩

private def loopLoanFixture (holderAfterOwner : Bool) : RawUnit :=
  let ns := conflictingLocalReadFixture.namespaces[0]!
  let breakLoop : Expr := { loc := ⟨14⟩, typeId := ⟨3⟩, kind := .break_ 0 none }
  let statements := if holderAfterOwner then #[⟨4⟩, ⟨3⟩, ⟨10⟩]
    else #[⟨4⟩, ⟨10⟩]
  let loopBody : Expr := {
    loc := ⟨15⟩, typeId := ⟨3⟩, kind := .block statements none }
  let loop : Expr := { loc := ⟨16⟩, typeId := ⟨0⟩, kind := .loop none ⟨11⟩ }
  { conflictingLocalReadFixture with
    tables := { fixture.tables with types := fixture.tables.types.push .never }
    namespaces := #[{
      ns with
      expressions := (ns.expressions ++ #[breakLoop, loopBody, loop]).set! 5 {
        ns.expressions[5]! with kind := .block #[⟨12⟩] (some ⟨0⟩) } }] }

#guard match validate #[schema] (loopLoanFixture false) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard preparationHasDiagnosticAt (loopLoanFixture true)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨4⟩

private def aliasedLoanFixture (ownerReadFirst : Bool) : RawUnit :=
  let ns := conflictingLocalReadFixture.namespaces[0]!
  let aliasMove : Expr := {
    loc := ⟨14⟩, typeId := ⟨2⟩, kind := .operation (.move ⟨1⟩) #[] #[] }
  let aliasUse : Expr := {
    loc := ⟨15⟩, typeId := ⟨2⟩, kind := .operation (.move ⟨3⟩) #[] #[] }
  let statements := if ownerReadFirst then #[⟨4⟩, ⟨11⟩] else #[⟨11⟩, ⟨4⟩]
  let aliasBody : Expr := {
    loc := ⟨16⟩, typeId := ⟨1⟩, kind := .block statements (some ⟨0⟩) }
  let aliasLet : Expr := {
    loc := ⟨17⟩, typeId := ⟨1⟩, kind := .letDecl ⟨2⟩ (some ⟨10⟩) ⟨12⟩ }
  { conflictingLocalReadFixture with namespaces := #[{
      ns with
      expressions := (ns.expressions ++ #[aliasMove, aliasUse, aliasBody, aliasLet]).set! 6 {
        ns.expressions[6]! with kind := .letDecl ⟨1⟩ (some ⟨8⟩) ⟨13⟩ }
      patterns := ns.patterns.push {
        loc := ⟨17⟩, typeId := ⟨2⟩, kind := .variable ⟨2⟩ }
      places := ns.places.push (.localVar ⟨2⟩)
      functions := ns.functions.set! 0 {
        ns.functions[0]! with locals := ns.functions[0]!.locals.push {
          id := ⟨2⟩, name := "alias", type := typeUse 2 17,
          mutable := false, loc := ⟨17⟩ } } }] }

#guard preparationHasDiagnosticAt (aliasedLoanFixture true)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨4⟩

#guard match validate #[schema] (aliasedLoanFixture false) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

private def calledAliasLoanFixture (ownerReadFirst : Bool) : RawUnit :=
  let raw := aliasedLoanFixture ownerReadFirst
  let ns := raw.namespaces[0]!
  let callAlias : Expr := {
    loc := ⟨18⟩
    typeId := ⟨2⟩
    kind := .operation (.call (.function { namespaceId := ⟨0⟩, name := ⟨1⟩ }))
      #[] #[⟨10⟩] }
  { raw with namespaces := #[{
      ns with
      expressions := (ns.expressions.push callAlias).set! 13 {
        ns.expressions[13]! with kind := .letDecl ⟨2⟩ (some ⟨14⟩) ⟨12⟩ } }] }

#guard preparationHasDiagnosticAt (calledAliasLoanFixture true)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨4⟩

#guard match validate #[schema] (calledAliasLoanFixture false) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

private def nestedArgumentLoanFixture (ownerFirst : Bool) : RawUnit :=
  let ns := conflictingLocalReadFixture.namespaces[0]!
  let nestedOwner : Expr := {
    loc := ⟨14⟩, typeId := ⟨1⟩, kind := .block #[] (some ⟨4⟩) }
  let holderRead : Expr := {
    loc := ⟨15⟩, typeId := ⟨1⟩, kind := .operation (.read ⟨2⟩) #[] #[] }
  let arguments := if ownerFirst then #[⟨10⟩, ⟨11⟩] else #[⟨11⟩, ⟨10⟩]
  { conflictingLocalReadFixture with
    tables := { fixture.tables with types := fixture.tables.types.push (.tuple #[⟨1⟩, ⟨1⟩]) }
    namespaces := #[{
      ns with
      expressions := (ns.expressions ++ #[nestedOwner, holderRead])
        |>.set! 5 { ns.expressions[5]! with
          typeId := ⟨3⟩, kind := .operation (.primitive .tuple) #[] arguments }
        |>.set! 6 { ns.expressions[6]! with typeId := ⟨3⟩ }
        |>.set! 7 { ns.expressions[7]! with typeId := ⟨3⟩ }
      functions := ns.functions.set! 0 { ns.functions[0]! with
        signature := { ns.functions[0]!.signature with results := #[typeUse 3 10] } } }] }

#guard preparationHasDiagnosticAt (nestedArgumentLoanFixture true)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨4⟩

#guard match validate #[schema] (nestedArgumentLoanFixture false) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

private def reborrowedLoanFixture (ownerReadFirst : Bool) : RawUnit :=
  let raw := aliasedLoanFixture ownerReadFirst
  let ns := raw.namespaces[0]!
  { raw with namespaces := #[{ ns with expressions := ns.expressions.set! 10 {
      ns.expressions[10]! with kind := .operation (.borrow .mutable ⟨2⟩) #[] #[] } }] }

#guard preparationHasDiagnosticAt (reborrowedLoanFixture true)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨4⟩

#guard match validate #[schema] (reborrowedLoanFixture false) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

private def distinctReborrowLifetimeFixture : RawUnit :=
  let raw := reborrowedLoanFixture false
  let ns := raw.namespaces[0]!
  let childReference : Ty := .reference {
    profile := .rust, kind := .mutable, referent := ⟨1⟩, lifetime := ⟨1⟩ }
  { raw with
    tables := {
      raw.tables with
      lifetimes := raw.tables.lifetimes.push { kind := .local, loc := ⟨0⟩ }
      types := raw.tables.types.push childReference }
    namespaces := #[{
      ns with
      expressions := ns.expressions.set! 10 { ns.expressions[10]! with typeId := ⟨3⟩ }
        |>.set! 11 { ns.expressions[11]! with typeId := ⟨3⟩ }
      patterns := ns.patterns.set! 2 { ns.patterns[2]! with typeId := ⟨3⟩ }
      functions := ns.functions.set! 0 {
        ns.functions[0]! with locals := ns.functions[0]!.locals.set! 2 {
          ns.functions[0]!.locals[2]! with type := typeUse 3 17 } } }] }

#guard match validate #[schema] distinctReborrowLifetimeFixture with
  | .error _ => false
  | .ok checked => match prepareExecution #[semantics] checked with
      | .error _ => false
      | .ok executable => match executable.borrowCertificates.find? (·.functionId == ⟨0⟩) with
          | none => false
          | some certificate => certificate.lifetimeRelations.contains {
              longer := ⟨0⟩, shorter := ⟨1⟩ }

private def distinctCallLifetimeFixture : RawUnit :=
  let raw := fixture
  let ns := raw.namespaces[0]!
  let callerReference : Ty := .reference {
    profile := .rust, kind := .mutable, referent := ⟨1⟩, lifetime := ⟨1⟩ }
  { raw with
    tables := {
      raw.tables with
      lifetimes := raw.tables.lifetimes.push { kind := .local, loc := ⟨0⟩ }
      types := raw.tables.types.push callerReference }
    namespaces := #[{
      ns with
      expressions := ns.expressions.set! 1 { ns.expressions[1]! with typeId := ⟨3⟩ }
        |>.set! 8 { ns.expressions[8]! with typeId := ⟨3⟩ }
      patterns := ns.patterns.set! 1 { ns.patterns[1]! with typeId := ⟨3⟩ }
      functions := ns.functions.set! 0 {
        ns.functions[0]! with locals := ns.functions[0]!.locals.set! 1 {
          ns.functions[0]!.locals[1]! with type := typeUse 3 10 } } }] }

#guard match validate #[schema] distinctCallLifetimeFixture with
  | .error _ => false
  | .ok checked => match prepareExecution #[semantics] checked with
      | .error _ => false
      | .ok executable => match executable.borrowCertificates.find? (·.functionId == ⟨0⟩) with
          | none => false
          | some certificate =>
              certificate.lifetimeRelations.contains { longer := ⟨1⟩, shorter := ⟨0⟩ } &&
                certificate.lifetimeRelations.contains { longer := ⟨0⟩, shorter := ⟨1⟩ }

private def distinctBoundaryLifetimeFixture (explicitReturn : Bool) : RawUnit :=
  let ns := fixture.namespaces[0]!
  let declaredReference : Ty := .reference {
    profile := .rust, kind := .shared, referent := ⟨1⟩, lifetime := ⟨0⟩ }
  let valueReference : Ty := .reference {
    profile := .rust, kind := .shared, referent := ⟨1⟩, lifetime := ⟨1⟩ }
  let frozen : Expr := {
    loc := ⟨14⟩, typeId := ⟨4⟩, kind := .operation
      (.reference (.freeze true)) #[] #[⟨9⟩] }
  let returned : Expr := {
    loc := ⟨15⟩, typeId := ⟨5⟩, kind := .return_ #[⟨10⟩] }
  { fixture with
    tables := {
      fixture.tables with
      lifetimes := fixture.tables.lifetimes.push { kind := .local, loc := ⟨0⟩ }
      types := fixture.tables.types ++ #[declaredReference, valueReference, .never] }
    namespaces := #[{
      ns with
      expressions := ns.expressions ++ #[frozen, returned]
      functions := (ns.functions.set! 0 {
        ns.functions[0]! with body := .structured ⟨0⟩ }).set! 1 {
        ns.functions[1]! with
        signature := { ns.functions[1]!.signature with results := #[typeUse 3 13] }
        body := .structured (if explicitReturn then ⟨11⟩ else ⟨10⟩) } }] }

private def boundaryLifetimeRelations? (explicitReturn : Bool) :
    Option (Array LifetimeRelationFact) := do
  let checked ← (validate #[schema] (distinctBoundaryLifetimeFixture explicitReturn)).toOption
  let executable ← (prepareExecution #[semantics] checked).toOption
  let certificate ← executable.borrowCertificates.find? (·.functionId == ⟨1⟩)
  some certificate.lifetimeRelations

#guard (boundaryLifetimeRelations? false).any fun relations =>
  relations.contains { longer := ⟨1⟩, shorter := ⟨0⟩ } &&
    relations.contains { longer := ⟨0⟩, shorter := ⟨1⟩ }

#guard (boundaryLifetimeRelations? true).any fun relations =>
  relations.contains { longer := ⟨1⟩, shorter := ⟨0⟩ } &&
    relations.contains { longer := ⟨0⟩, shorter := ⟨1⟩ }

private def distinctAggregateBoundaryLifetimeFixture (explicitReturn : Bool) : RawUnit :=
  let raw := distinctBoundaryLifetimeFixture false
  let ns := raw.namespaces[0]!
  let aggregate : Expr := {
    loc := ⟨16⟩, typeId := ⟨7⟩, kind := .operation (.primitive .tuple) #[] #[⟨9⟩, ⟨0⟩] }
  let returned : Expr := {
    loc := ⟨17⟩, typeId := ⟨5⟩, kind := .return_ #[⟨12⟩] }
  { raw with
    tables := { raw.tables with types := raw.tables.types ++ #[
      .tuple #[⟨3⟩, ⟨1⟩], .tuple #[⟨4⟩, ⟨1⟩]] }
    namespaces := #[{
      ns with
      expressions := (ns.expressions ++ #[aggregate, returned]).set! 9 {
        ns.expressions[9]! with typeId := ⟨4⟩ }
      functions := (ns.functions.set! 0 {
        ns.functions[0]! with body := .structured ⟨0⟩ }).set! 1 {
        ns.functions[1]! with
        signature := {
          parameters := #[{ name := "reference", typeUse := typeUse 4 13 }]
          results := #[typeUse 6 13] }
        locals := #[{
          id := ⟨0⟩, name := "reference", type := typeUse 4 13,
          mutable := false, loc := ⟨13⟩ }]
        body := .structured (if explicitReturn then ⟨13⟩ else ⟨12⟩) } }] }

private def packedAggregateBoundaryLifetimeRelations? (explicitReturn : Bool) :
    Option (Array LifetimeRelationFact) := do
  let checked ← (validate #[schema]
    (distinctAggregateBoundaryLifetimeFixture explicitReturn)).toOption
  let executable ← (prepareExecution #[semantics] checked).toOption
  let certificate ← executable.borrowCertificates.find? (·.functionId == ⟨1⟩)
  some certificate.lifetimeRelations

#guard (packedAggregateBoundaryLifetimeRelations? false).any fun relations =>
  relations.contains { longer := ⟨1⟩, shorter := ⟨0⟩ } &&
    !relations.contains { longer := ⟨0⟩, shorter := ⟨1⟩ }

#guard (packedAggregateBoundaryLifetimeRelations? true).any fun relations =>
  relations.contains { longer := ⟨1⟩, shorter := ⟨0⟩ } &&
    !relations.contains { longer := ⟨0⟩, shorter := ⟨1⟩ }

private def invariantNestedReferenceBoundaryFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  let innerDeclared : Ty := .reference {
    profile := .rust, kind := .shared, referent := ⟨1⟩, lifetime := ⟨0⟩ }
  let innerValue : Ty := .reference {
    profile := .rust, kind := .shared, referent := ⟨1⟩, lifetime := ⟨1⟩ }
  let outerDeclared : Ty := .reference {
    profile := .rust, kind := .mutable, referent := ⟨3⟩, lifetime := ⟨0⟩ }
  let outerValue : Ty := .reference {
    profile := .rust, kind := .mutable, referent := ⟨4⟩, lifetime := ⟨0⟩ }
  { fixture with
    tables := {
      fixture.tables with
      lifetimes := fixture.tables.lifetimes.push { kind := .local, loc := ⟨0⟩ }
      types := fixture.tables.types ++ #[
        innerDeclared, innerValue, outerDeclared, outerValue] }
    namespaces := #[{
      ns with
      expressions := ns.expressions.set! 9 { ns.expressions[9]! with typeId := ⟨6⟩ }
      functions := (ns.functions.set! 0 {
        ns.functions[0]! with body := .structured ⟨0⟩ }).set! 1 {
        ns.functions[1]! with
        signature := {
          parameters := #[{ name := "reference", typeUse := typeUse 6 13 }]
          results := #[typeUse 5 13] }
        locals := #[{
          id := ⟨0⟩, name := "reference", type := typeUse 6 13,
          mutable := false, loc := ⟨13⟩ }] } }] }

#guard match validate #[schema] invariantNestedReferenceBoundaryFixture with
  | .error _ => false
  | .ok checked => match prepareExecution #[semantics] checked with
      | .error _ => false
      | .ok executable => match executable.borrowCertificates.find? (·.functionId == ⟨1⟩) with
          | none => false
          | some certificate =>
              certificate.lifetimeRelations.contains { longer := ⟨1⟩, shorter := ⟨0⟩ } &&
                certificate.lifetimeRelations.contains { longer := ⟨0⟩, shorter := ⟨1⟩ }

private def contravariantFunctionBoundaryFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  let declaredArgument : Ty := .reference {
    profile := .rust, kind := .shared, referent := ⟨1⟩, lifetime := ⟨0⟩ }
  let valueArgument : Ty := .reference {
    profile := .rust, kind := .shared, referent := ⟨1⟩, lifetime := ⟨1⟩ }
  { fixture with
    tables := {
      fixture.tables with
      lifetimes := fixture.tables.lifetimes.push { kind := .local, loc := ⟨0⟩ }
      types := fixture.tables.types ++ #[
        declaredArgument, valueArgument,
        .function #[⟨3⟩] ⟨1⟩ #[], .function #[⟨4⟩] ⟨1⟩ #[]] }
    namespaces := #[{
      ns with
      expressions := ns.expressions.set! 9 { ns.expressions[9]! with typeId := ⟨6⟩ }
      functions := (ns.functions.set! 0 {
        ns.functions[0]! with body := .structured ⟨0⟩ }).set! 1 {
        ns.functions[1]! with
        signature := {
          parameters := #[{ name := "function", typeUse := typeUse 6 13 }]
          results := #[typeUse 5 13] }
        locals := #[{
          id := ⟨0⟩, name := "function", type := typeUse 6 13,
          mutable := false, loc := ⟨13⟩ }] } }] }

#guard match validate #[schema] contravariantFunctionBoundaryFixture with
  | .error _ => false
  | .ok checked => match prepareExecution #[semantics] checked with
      | .error _ => false
      | .ok executable => match executable.borrowCertificates.find? (·.functionId == ⟨1⟩) with
          | none => false
          | some certificate =>
              certificate.lifetimeRelations.contains { longer := ⟨0⟩, shorter := ⟨1⟩ } &&
                !certificate.lifetimeRelations.contains { longer := ⟨1⟩, shorter := ⟨0⟩ }

private def distinctAggregateCallLifetimeFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  let callerReference : Ty := .reference {
    profile := .rust, kind := .mutable, referent := ⟨1⟩, lifetime := ⟨1⟩ }
  let callerMove : Expr := {
    loc := ⟨14⟩, typeId := ⟨5⟩, kind := .operation (.move ⟨0⟩) #[] #[] }
  let call : Expr := {
    loc := ⟨15⟩, typeId := ⟨5⟩, kind := .operation
      (.call (.function { namespaceId := ⟨0⟩, name := ⟨2⟩ })) #[] #[⟨10⟩] }
  let calleeMove : Expr := {
    loc := ⟨16⟩, typeId := ⟨4⟩, kind := .operation (.move ⟨0⟩) #[] #[] }
  let aggregateIdentity : FunctionDecl RawBody := {
    loc := ⟨17⟩
    name := ⟨2⟩
    profile := .rust
    signature := {
      parameters := #[{ name := "value", typeUse := typeUse 4 17 }]
      results := #[typeUse 4 17] }
    body := .structured ⟨12⟩
    origin := ⟨0⟩
    alignment := ⟨0⟩
    locals := #[{
      id := ⟨0⟩, name := "value", type := typeUse 4 17,
      mutable := false, loc := ⟨17⟩ }] }
  let aggregateCaller : FunctionDecl RawBody := {
    loc := ⟨18⟩
    name := ⟨3⟩
    profile := .rust
    signature := {
      parameters := #[{ name := "value", typeUse := typeUse 5 18 }]
      results := #[typeUse 4 18] }
    body := .structured ⟨11⟩
    origin := ⟨0⟩
    alignment := ⟨0⟩
    locals := #[{
      id := ⟨0⟩, name := "value", type := typeUse 5 18,
      mutable := false, loc := ⟨18⟩ }] }
  { fixture with
    tables := {
      fixture.tables with
      lifetimes := fixture.tables.lifetimes.push { kind := .local, loc := ⟨0⟩ }
      types := fixture.tables.types ++ #[
        callerReference, .tuple #[⟨2⟩, ⟨1⟩], .tuple #[⟨3⟩, ⟨1⟩]]
      names := fixture.tables.names ++ #[
        { namespaceId := ⟨0⟩, name := "aggregate_identity" },
        { namespaceId := ⟨0⟩, name := "aggregate_caller" }] }
    namespaces := #[{
      ns with
      expressions := ns.expressions ++ #[callerMove, call, calleeMove]
      functions := ns.functions ++ #[aggregateIdentity, aggregateCaller] }] }

#guard match validate #[schema] distinctAggregateCallLifetimeFixture with
  | .error _ => false
  | .ok checked => match prepareExecution #[semantics] checked with
      | .error _ => false
      | .ok executable => match executable.borrowCertificates.find? (·.functionId == ⟨3⟩) with
          | none => false
          | some certificate =>
              certificate.lifetimeRelations.contains { longer := ⟨1⟩, shorter := ⟨0⟩ } &&
                certificate.lifetimeRelations.contains { longer := ⟨0⟩, shorter := ⟨1⟩ }

private def aggregateBoundaryLifetimeFixture (explicitReturn : Bool) : RawUnit :=
  let raw := distinctBoundaryLifetimeFixture false
  let ns := raw.namespaces[0]!
  let packed : Expr := {
    loc := ⟨16⟩, typeId := ⟨7⟩,
    kind := .operation (.primitive .tuple) #[] #[⟨10⟩] }
  let returned : Expr := {
    loc := ⟨17⟩, typeId := ⟨5⟩, kind := .return_ #[⟨12⟩] }
  let callee := ns.functions[1]!
  { raw with
    tables := { raw.tables with types := raw.tables.types ++ #[
      .tuple #[⟨3⟩], .tuple #[⟨4⟩]] }
    namespaces := #[{
      ns with
      expressions := ns.expressions ++ #[packed, returned]
      functions := ns.functions.set! 1 { callee with
        signature := { callee.signature with results := #[typeUse 6 13] }
        body := .structured (if explicitReturn then ⟨13⟩ else ⟨12⟩) } }] }

private def aggregateBoundaryLifetimeRelations? (explicitReturn : Bool) :
    Option (Array LifetimeRelationFact) := do
  let checked ← (validate #[schema]
    (aggregateBoundaryLifetimeFixture explicitReturn)).toOption
  let executable ← (prepareExecution #[semantics] checked).toOption
  let certificate ← executable.borrowCertificates.find? (·.functionId == ⟨1⟩)
  some certificate.lifetimeRelations

#guard (aggregateBoundaryLifetimeRelations? false).any fun relations =>
  relations.contains { longer := ⟨1⟩, shorter := ⟨0⟩ }

#guard (aggregateBoundaryLifetimeRelations? true).any fun relations =>
  relations.contains { longer := ⟨1⟩, shorter := ⟨0⟩ }

private def assignedLoanFixture (ownerReadFirst : Bool) : RawUnit :=
  let ns := conflictingLocalReadFixture.namespaces[0]!
  let assignReference : Expr := {
    loc := ⟨14⟩, typeId := ⟨0⟩, kind := .assign ⟨1⟩ ⟨1⟩ }
  let statements := if ownerReadFirst then #[⟨10⟩, ⟨4⟩, ⟨3⟩]
    else #[⟨10⟩, ⟨3⟩, ⟨4⟩]
  let body : Expr := {
    loc := ⟨15⟩, typeId := ⟨1⟩, kind := .block statements (some ⟨0⟩) }
  { conflictingLocalReadFixture with namespaces := #[{
      ns with
      expressions := (ns.expressions ++ #[assignReference, body]).set! 7 {
        ns.expressions[7]! with kind := .letDecl ⟨0⟩ (some ⟨0⟩) ⟨11⟩ }
      functions := ns.functions.set! 0 {
        ns.functions[0]! with locals := ns.functions[0]!.locals.set! 1 {
          ns.functions[0]!.locals[1]! with mutable := true } } }] }

#guard preparationHasDiagnosticAt (assignedLoanFixture true)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨4⟩

#guard match validate #[schema] (assignedLoanFixture false) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

private def overwrittenLoanFixture : RawUnit :=
  let ns := conflictingLocalReadFixture.namespaces[0]!
  let secondBorrow : Expr := {
    loc := ⟨14⟩, typeId := ⟨2⟩, kind := .operation (.borrow .mutable ⟨3⟩) #[] #[] }
  let overwrite : Expr := {
    loc := ⟨15⟩, typeId := ⟨0⟩, kind := .assign ⟨1⟩ ⟨10⟩ }
  let holderRead : Expr := {
    loc := ⟨16⟩, typeId := ⟨1⟩, kind := .operation (.read ⟨2⟩) #[] #[] }
  let body : Expr := {
    loc := ⟨17⟩, typeId := ⟨1⟩, kind := .block #[⟨11⟩, ⟨4⟩] (some ⟨12⟩) }
  let secondOwnerLet : Expr := {
    loc := ⟨18⟩, typeId := ⟨1⟩, kind := .letDecl ⟨2⟩ (some ⟨2⟩) ⟨13⟩ }
  { conflictingLocalReadFixture with namespaces := #[{
      ns with
      expressions := (ns.expressions ++ #[secondBorrow, overwrite, holderRead, body,
        secondOwnerLet]).set! 6 {
          ns.expressions[6]! with kind := .letDecl ⟨1⟩ (some ⟨8⟩) ⟨14⟩ }
      patterns := ns.patterns.push {
        loc := ⟨18⟩, typeId := ⟨1⟩, kind := .variable ⟨2⟩ }
      places := ns.places.push (.localVar ⟨2⟩)
      functions := ns.functions.set! 0 {
        ns.functions[0]! with
        locals := (ns.functions[0]!.locals.set! 1 {
          ns.functions[0]!.locals[1]! with mutable := true }).push {
            id := ⟨2⟩, name := "second_owner", type := typeUse 1 18,
            mutable := true, loc := ⟨18⟩ } } }] }

#guard match validate #[schema] overwrittenLoanFixture with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

private def overwrittenLoanWithWriteFixture : RawUnit :=
  let ns := overwrittenLoanFixture.namespaces[0]!
  { overwrittenLoanFixture with namespaces := #[{ ns with
      expressions := ns.expressions.set! 11 {
        ns.expressions[11]! with kind := .operation (.write ⟨1⟩) #[] #[⟨10⟩] } }] }

#guard match validate #[schema] overwrittenLoanWithWriteFixture with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

private def overwrittenLoanWithPatternFixture : RawUnit :=
  let ns := overwrittenLoanFixture.namespaces[0]!
  { overwrittenLoanFixture with namespaces := #[{ ns with
      expressions := ns.expressions.set! 11 {
        ns.expressions[11]! with kind := .assignPattern ⟨1⟩ ⟨10⟩ } }] }

#guard match validate #[schema] overwrittenLoanWithPatternFixture with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

private def reassignedSamePlaceLoanFixture (kind : Nat) : RawUnit :=
  let ns := conflictingLocalReadFixture.namespaces[0]!
  let replacement : Expr := {
    loc := ⟨14⟩, typeId := ⟨2⟩,
    kind := .operation (.borrow .mutable ⟨0⟩) #[] #[] }
  let overwrite : Expr := {
    loc := ⟨15⟩
    typeId := ⟨0⟩
    kind := if kind == 0 then .assign ⟨1⟩ ⟨10⟩
      else if kind == 1 then .operation (.write ⟨1⟩) #[] #[⟨10⟩]
      else .assignPattern ⟨1⟩ ⟨10⟩ }
  let holderRead : Expr := {
    loc := ⟨16⟩, typeId := ⟨1⟩,
    kind := .operation (.read ⟨2⟩) #[] #[] }
  let body : Expr := {
    loc := ⟨17⟩, typeId := ⟨1⟩,
    kind := .block #[⟨11⟩] (some ⟨12⟩) }
  { conflictingLocalReadFixture with namespaces := #[{
      ns with
      expressions := (ns.expressions ++ #[replacement, overwrite, holderRead, body]).set! 5 body
      functions := ns.functions.set! 0 {
        ns.functions[0]! with locals := ns.functions[0]!.locals.set! 1 {
          ns.functions[0]!.locals[1]! with mutable := true } } }] }

#guard match validate #[schema] (reassignedSamePlaceLoanFixture 0) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard match validate #[schema] (reassignedSamePlaceLoanFixture 1) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard match validate #[schema] (reassignedSamePlaceLoanFixture 2) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

private def selectedCallResultFixture (readSelectedOwner : Bool) : RawUnit :=
  let raw := overwrittenLoanFixture
  let ns := raw.namespaces[0]!
  let secondReference : Ty := .reference {
    profile := .rust, kind := .mutable, referent := ⟨1⟩, lifetime := ⟨1⟩ }
  let resultRead : Expr := {
    loc := ⟨19⟩, typeId := ⟨1⟩, kind := .operation (.read ⟨5⟩) #[] #[] }
  let body : Expr := {
    loc := ⟨19⟩, typeId := ⟨1⟩, kind := .block #[⟨13⟩] (some ⟨15⟩) }
  let resultLet : Expr := {
    loc := ⟨19⟩, typeId := ⟨1⟩, kind := .letDecl ⟨3⟩ (some ⟨12⟩) ⟨16⟩ }
  let ownerRead : Expr := {
    ns.expressions[13]! with kind := .localVar (if readSelectedOwner then ⟨0⟩ else ⟨2⟩) }
  let callee := ns.functions[1]!
  { raw with
    tables := {
      raw.tables with
      lifetimes := raw.tables.lifetimes.push { kind := .local, loc := ⟨0⟩ }
      types := raw.tables.types.push secondReference }
    namespaces := #[{
      ns with
      expressions := (ns.expressions ++ #[resultRead, body, resultLet])
        |>.set! 6 { ns.expressions[6]! with
          kind := .letDecl ⟨1⟩ (some ⟨1⟩) ⟨14⟩ }
        |>.set! 10 { ns.expressions[10]! with typeId := ⟨3⟩ }
        |>.set! 11 { ns.expressions[11]! with
          typeId := ⟨2⟩, kind := .operation (.move ⟨1⟩) #[] #[] }
        |>.set! 12 { ns.expressions[12]! with
          typeId := ⟨2⟩, kind := .operation
            (.call (.function { namespaceId := ⟨0⟩, name := ⟨1⟩ })) #[] #[⟨11⟩, ⟨10⟩] }
        |>.set! 13 ownerRead
        |>.set! 14 { ns.expressions[14]! with
          kind := .letDecl ⟨2⟩ (some ⟨2⟩) ⟨17⟩ }
      patterns := ns.patterns.push {
        loc := ⟨19⟩, typeId := ⟨2⟩, kind := .variable ⟨3⟩ }
      places := ns.places ++ #[.localVar ⟨3⟩, .deref ⟨4⟩]
      functions := (ns.functions.set! 0 {
        ns.functions[0]! with locals := ns.functions[0]!.locals.push {
          id := ⟨3⟩, name := "result", type := typeUse 2 19,
          mutable := false, loc := ⟨19⟩ } }).set! 1 {
            callee with
            signature := { callee.signature with parameters := callee.signature.parameters.push {
              name := "other", typeUse := typeUse 3 13 } }
            locals := callee.locals.push {
              id := ⟨1⟩, name := "other", type := typeUse 3 13,
              mutable := false, loc := ⟨13⟩ } } }] }

#guard match validate #[schema] (selectedCallResultFixture false) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard preparationHasDiagnosticAt (selectedCallResultFixture true)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨17⟩

private def selectedTuplePatternLoanFixture (readSelectedOwner : Bool) : RawUnit :=
  let raw := selectedCallResultFixture false
  let ns := raw.namespaces[0]!
  let aggregate : Expr := {
    loc := ⟨19⟩, typeId := ⟨4⟩,
    kind := .operation (.primitive .tuple) #[] #[⟨1⟩, ⟨10⟩] }
  let ownerRead : Expr := {
    loc := ⟨19⟩, typeId := ⟨1⟩,
    kind := .localVar (if readSelectedOwner then ⟨0⟩ else ⟨2⟩) }
  let holderRead : Expr := {
    loc := ⟨19⟩, typeId := ⟨1⟩,
    kind := .operation (.read ⟨2⟩) #[] #[] }
  let body : Expr := {
    loc := ⟨19⟩, typeId := ⟨1⟩,
    kind := .block #[⟨19⟩] (some ⟨20⟩) }
  let aggregateLet : Expr := {
    loc := ⟨19⟩, typeId := ⟨1⟩,
    kind := .letDecl ⟨4⟩ (some ⟨18⟩) ⟨21⟩ }
  { raw with
    tables := { raw.tables with types := raw.tables.types.push (.tuple #[⟨2⟩, ⟨3⟩]) }
    namespaces := #[{
      ns with
      expressions := (ns.expressions ++ #[aggregate, ownerRead, holderRead, body, aggregateLet])
        |>.set! 7 { ns.expressions[7]! with
          kind := .letDecl ⟨0⟩ (some ⟨0⟩) ⟨14⟩ }
        |>.set! 14 { ns.expressions[14]! with
          kind := .letDecl ⟨2⟩ (some ⟨2⟩) ⟨22⟩ }
      patterns := (ns.patterns.set! 3 {
        ns.patterns[3]! with typeId := ⟨3⟩ }).push {
          loc := ⟨19⟩, typeId := ⟨4⟩, kind := .tuple #[⟨1⟩, ⟨3⟩] }
      functions := ns.functions.set! 0 {
        ns.functions[0]! with locals := ns.functions[0]!.locals.set! 3 {
          ns.functions[0]!.locals[3]! with type := typeUse 3 19 } } }] }

#guard match validate #[schema] (selectedTuplePatternLoanFixture false) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard preparationHasDiagnosticAt (selectedTuplePatternLoanFixture true)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨19⟩

private def selectedConstructorPatternLoanFixture (readSelectedOwner : Bool) : RawUnit :=
  let raw := selectedTuplePatternLoanFixture readSelectedOwner
  let ns := raw.namespaces[0]!
  let constructor : QualifiedRef := { namespaceId := ⟨0⟩, name := ⟨2⟩ }
  let operation : Operation := .call (.constructor constructor)
  let aggregate : Expr := {
    ns.expressions[18]! with kind := .operation operation #[] #[⟨1⟩, ⟨10⟩] }
  { raw with
    tables := {
      raw.tables with
      types := raw.tables.types.set! 4 (.nominal ⟨2⟩ #[])
      names := raw.tables.names ++ #[
        { namespaceId := ⟨0⟩, name := "ReferencePair" },
        { namespaceId := ⟨0⟩, name := "first" },
        { namespaceId := ⟨0⟩, name := "second" }] }
    namespaces := #[{
      ns with
      expressions := ns.expressions.set! 18 aggregate
      patterns := ns.patterns.set! 4 {
        ns.patterns[4]! with kind := .constructor ⟨2⟩ #[] none #[⟨1⟩, ⟨3⟩] }
      structs := #[{
        loc := ⟨19⟩, name := ⟨2⟩, fields := #[
          { loc := ⟨19⟩, name := ⟨3⟩, type := typeUse 2 19 },
          { loc := ⟨19⟩, name := ⟨4⟩, type := typeUse 3 19 }] }] }] }

#guard match validate #[schema] (selectedConstructorPatternLoanFixture false) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard preparationHasDiagnosticAt (selectedConstructorPatternLoanFixture true)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨19⟩

private def projectedTupleCarrierLoanFixture (readSelectedOwner : Bool) : RawUnit :=
  let raw := selectedTuplePatternLoanFixture readSelectedOwner
  let ns := raw.namespaces[0]!
  let index : Expr := {
    loc := ⟨19⟩, typeId := ⟨1⟩, kind := .value (.integer 0) }
  let selected : Expr := {
    loc := ⟨19⟩, typeId := ⟨2⟩, kind := .operation (.move ⟨7⟩) #[] #[] }
  { raw with namespaces := #[{
      ns with
      expressions := (ns.expressions ++ #[index, selected]).set! 21 {
        ns.expressions[21]! with kind := .block #[⟨19⟩, ⟨24⟩] (some ⟨0⟩) }
      patterns := ns.patterns.set! 4 {
        ns.patterns[4]! with kind := .variable ⟨4⟩ }
      places := ns.places ++ #[.localVar ⟨4⟩, .index ⟨6⟩ ⟨23⟩]
      functions := ns.functions.set! 0 {
        ns.functions[0]! with locals := ns.functions[0]!.locals.push {
          id := ⟨4⟩, name := "references", type := typeUse 4 19,
          mutable := false, loc := ⟨19⟩ } } }] }

#guard match validate #[schema] (projectedTupleCarrierLoanFixture false) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard preparationHasDiagnosticAt (projectedTupleCarrierLoanFixture true)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨19⟩

private def projectedStructCarrierLoanFixture (readSelectedOwner : Bool) : RawUnit :=
  let raw := projectedTupleCarrierLoanFixture readSelectedOwner
  let ns := raw.namespaces[0]!
  let constructor : QualifiedRef := { namespaceId := ⟨0⟩, name := ⟨2⟩ }
  { raw with
    tables := {
      raw.tables with
      types := raw.tables.types.set! 4 (.nominal ⟨2⟩ #[])
      names := raw.tables.names ++ #[
        { namespaceId := ⟨0⟩, name := "ReferencePair" },
        { namespaceId := ⟨0⟩, name := "first" },
        { namespaceId := ⟨0⟩, name := "second" }] }
    namespaces := #[{
      ns with
      expressions := ns.expressions.set! 18 {
        ns.expressions[18]! with
          kind := .operation (.call (.constructor constructor)) #[] #[⟨1⟩, ⟨10⟩] }
      places := ns.places.set! 7 (.field ⟨6⟩ ⟨⟨0⟩, ⟨2⟩⟩ ⟨3⟩)
      structs := #[{
        loc := ⟨19⟩, name := ⟨2⟩, fields := #[
          { loc := ⟨19⟩, name := ⟨3⟩, type := typeUse 2 19 },
          { loc := ⟨19⟩, name := ⟨4⟩, type := typeUse 3 19 }] }] }] }

#guard match validate #[schema] (projectedStructCarrierLoanFixture false) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard preparationHasDiagnosticAt (projectedStructCarrierLoanFixture true)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨19⟩

private def projectedEnumCarrierLoanFixture (readSelectedOwner : Bool) : RawUnit :=
  let raw := projectedStructCarrierLoanFixture readSelectedOwner
  let ns := raw.namespaces[0]!
  let constructor : QualifiedRef := { namespaceId := ⟨0⟩, name := ⟨2⟩ }
  { raw with
    tables := { raw.tables with names := raw.tables.names ++ #[
      { namespaceId := ⟨0⟩, name := "First" },
      { namespaceId := ⟨0⟩, name := "Empty" }] }
    namespaces := #[{
      ns with
      expressions := ns.expressions.set! 18 {
        ns.expressions[18]! with
          kind := .operation (.call (.constructor constructor (some "First"))) #[]
            #[⟨1⟩, ⟨10⟩] }
      places := (ns.places.set! 7 (.field ⟨8⟩ ⟨⟨0⟩, ⟨2⟩⟩ ⟨3⟩)).push
        (.downcast ⟨6⟩ ⟨5⟩)
      structs := #[{
        loc := ⟨19⟩, name := ⟨2⟩, variants := #[
          { loc := ⟨19⟩, name := ⟨5⟩, fields := #[
            { loc := ⟨19⟩, name := ⟨3⟩, type := typeUse 2 19 },
            { loc := ⟨19⟩, name := ⟨4⟩, type := typeUse 3 19 }] },
          { loc := ⟨19⟩, name := ⟨6⟩ }] }] }] }

#guard match validate #[schema] (projectedEnumCarrierLoanFixture false) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard preparationHasDiagnosticAt (projectedEnumCarrierLoanFixture true)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨19⟩

private def projectedCarrierMoveLoanFixture (readMovedOwner : Bool) : RawUnit :=
  let raw := projectedTupleCarrierLoanFixture readMovedOwner
  let ns := raw.namespaces[0]!
  let secondIndex : Expr := {
    loc := ⟨19⟩, typeId := ⟨1⟩, kind := .value (.integer 1) }
  let secondSelected : Expr := {
    loc := ⟨19⟩, typeId := ⟨3⟩, kind := .operation (.move ⟨8⟩) #[] #[] }
  { raw with namespaces := #[{
      ns with
      expressions := (ns.expressions ++ #[secondIndex, secondSelected]).set! 21 {
        ns.expressions[21]! with kind := .block #[⟨24⟩, ⟨19⟩, ⟨26⟩] (some ⟨0⟩) }
      places := ns.places.push (.index ⟨6⟩ ⟨25⟩) }] }

#guard match validate #[schema] (projectedCarrierMoveLoanFixture true) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard preparationHasDiagnosticAt (projectedCarrierMoveLoanFixture false)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨19⟩

private def projectedCarrierReplacementFixture (write readSibling : Bool) : RawUnit :=
  let raw := projectedCarrierMoveLoanFixture (!readSibling)
  let ns := raw.namespaces[0]!
  let replacement : Expr := {
    loc := ⟨19⟩, typeId := ⟨2⟩,
    kind := .operation (.borrow .mutable ⟨0⟩) #[] #[] }
  let overwrite : Expr := {
    loc := ⟨19⟩, typeId := ⟨0⟩,
    kind := if write then .operation (.write ⟨7⟩) #[] #[⟨27⟩]
      else .assign ⟨7⟩ ⟨27⟩ }
  let body : Expr := {
    ns.expressions[21]! with kind := .block #[⟨28⟩, ⟨19⟩, ⟨26⟩] (some ⟨0⟩) }
  { raw with namespaces := #[{
      ns with
      expressions := (ns.expressions ++ #[replacement, overwrite]).set! 21 body
      functions := ns.functions.set! 0 {
        ns.functions[0]! with locals := (ns.functions[0]!.locals.set! 4 {
          ns.functions[0]!.locals[4]! with mutable := true }) } }] }

#guard match validate #[schema] (projectedCarrierReplacementFixture false false) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard match validate #[schema] (projectedCarrierReplacementFixture true false) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard preparationHasDiagnosticAt (projectedCarrierReplacementFixture false true)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨19⟩

#guard preparationHasDiagnosticAt (projectedCarrierReplacementFixture true true)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨19⟩

private def conditionalCarrierLoanFixture (readSelectedOwner : Bool) : RawUnit :=
  let raw := projectedTupleCarrierLoanFixture readSelectedOwner
  let ns := raw.namespaces[0]!
  let condition : Expr := {
    loc := ⟨19⟩, typeId := ⟨5⟩, kind := .value (.bool true) }
  let selectedAggregate : Expr := {
    loc := ⟨19⟩, typeId := ⟨4⟩,
    kind := .ifElse ⟨25⟩ ⟨18⟩ (some ⟨18⟩) }
  { raw with
    tables := { raw.tables with types := raw.tables.types.push .bool }
    namespaces := #[{
      ns with
      expressions := (ns.expressions ++ #[condition, selectedAggregate]).set! 22 {
        ns.expressions[22]! with kind := .letDecl ⟨4⟩ (some ⟨26⟩) ⟨21⟩ } }] }

#guard match validate #[schema] (conditionalCarrierLoanFixture false) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard preparationHasDiagnosticAt (conditionalCarrierLoanFixture true)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨19⟩

private def conditionalTuplePatternCarrierFixture (readSelectedOwner : Bool) : RawUnit :=
  let raw := selectedTuplePatternLoanFixture readSelectedOwner
  let ns := raw.namespaces[0]!
  let condition : Expr := {
    loc := ⟨19⟩, typeId := ⟨5⟩, kind := .value (.bool true) }
  let selectedAggregate : Expr := {
    loc := ⟨19⟩, typeId := ⟨4⟩,
    kind := .ifElse ⟨23⟩ ⟨18⟩ (some ⟨18⟩) }
  { raw with
    tables := { raw.tables with types := raw.tables.types.push .bool }
    namespaces := #[{
      ns with
      expressions := (ns.expressions ++ #[condition, selectedAggregate]).set! 22 {
        ns.expressions[22]! with kind := .letDecl ⟨4⟩ (some ⟨24⟩) ⟨21⟩ } }] }

#guard match validate #[schema] (conditionalTuplePatternCarrierFixture false) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard preparationHasDiagnosticAt (conditionalTuplePatternCarrierFixture true)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨19⟩

private def conditionalConstructorPatternCarrierFixture
    (readSelectedOwner : Bool) : RawUnit :=
  let raw := selectedConstructorPatternLoanFixture readSelectedOwner
  let ns := raw.namespaces[0]!
  let condition : Expr := {
    loc := ⟨19⟩, typeId := ⟨5⟩, kind := .value (.bool true) }
  let selectedAggregate : Expr := {
    loc := ⟨19⟩, typeId := ⟨4⟩,
    kind := .ifElse ⟨23⟩ ⟨18⟩ (some ⟨18⟩) }
  { raw with
    tables := { raw.tables with types := raw.tables.types.push .bool }
    namespaces := #[{
      ns with
      expressions := (ns.expressions ++ #[condition, selectedAggregate]).set! 22 {
        ns.expressions[22]! with kind := .letDecl ⟨4⟩ (some ⟨24⟩) ⟨21⟩ } }] }

#guard match validate #[schema] (conditionalConstructorPatternCarrierFixture false) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard preparationHasDiagnosticAt (conditionalConstructorPatternCarrierFixture true)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨19⟩

private def matchTuplePatternCarrierFixture (readSelectedOwner : Bool) : RawUnit :=
  let raw := selectedTuplePatternLoanFixture readSelectedOwner
  let ns := raw.namespaces[0]!
  let scrutinee : Expr := {
    loc := ⟨19⟩, typeId := ⟨5⟩, kind := .value (.bool true) }
  let selectedAggregate : Expr := {
    loc := ⟨19⟩, typeId := ⟨4⟩,
    kind := .match_ ⟨23⟩ #[{ pattern := ⟨5⟩, body := ⟨18⟩ }] }
  { raw with
    tables := { raw.tables with types := raw.tables.types.push .bool }
    namespaces := #[{
      ns with
      expressions := (ns.expressions ++ #[scrutinee, selectedAggregate]).set! 22 {
        ns.expressions[22]! with kind := .letDecl ⟨4⟩ (some ⟨24⟩) ⟨21⟩ }
      patterns := ns.patterns.push {
        loc := ⟨19⟩, typeId := ⟨5⟩, kind := .wildcard } }] }

#guard match validate #[schema] (matchTuplePatternCarrierFixture false) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard preparationHasDiagnosticAt (matchTuplePatternCarrierFixture true)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨19⟩

private def matchConstructorPatternCarrierFixture
    (readSelectedOwner : Bool) : RawUnit :=
  let raw := selectedConstructorPatternLoanFixture readSelectedOwner
  let ns := raw.namespaces[0]!
  let scrutinee : Expr := {
    loc := ⟨19⟩, typeId := ⟨5⟩, kind := .value (.bool true) }
  let selectedAggregate : Expr := {
    loc := ⟨19⟩, typeId := ⟨4⟩,
    kind := .match_ ⟨23⟩ #[{ pattern := ⟨5⟩, body := ⟨18⟩ }] }
  { raw with
    tables := { raw.tables with types := raw.tables.types.push .bool }
    namespaces := #[{
      ns with
      expressions := (ns.expressions ++ #[scrutinee, selectedAggregate]).set! 22 {
        ns.expressions[22]! with kind := .letDecl ⟨4⟩ (some ⟨24⟩) ⟨21⟩ }
      patterns := ns.patterns.push {
        loc := ⟨19⟩, typeId := ⟨5⟩, kind := .wildcard } }] }

#guard match validate #[schema] (matchConstructorPatternCarrierFixture false) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard preparationHasDiagnosticAt (matchConstructorPatternCarrierFixture true)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨19⟩

private def nestedInitializerCarrierLoanFixture (readSelectedOwner : Bool) : RawUnit :=
  let raw := projectedTupleCarrierLoanFixture readSelectedOwner
  let ns := raw.namespaces[0]!
  let innerBody : Expr := {
    loc := ⟨19⟩, typeId := ⟨1⟩, kind := .block #[⟨24⟩] (some ⟨0⟩) }
  { raw with namespaces := #[{
      ns with
      expressions := (ns.expressions.push innerBody).set! 21 {
        ns.expressions[21]! with kind := .letDecl ⟨5⟩ (some ⟨19⟩) ⟨25⟩ }
      patterns := ns.patterns.push {
        loc := ⟨19⟩, typeId := ⟨1⟩, kind := .variable ⟨5⟩ }
      functions := ns.functions.set! 0 {
        ns.functions[0]! with locals := ns.functions[0]!.locals.push {
          id := ⟨5⟩, name := "read_value", type := typeUse 1 19,
          mutable := false, loc := ⟨19⟩ } } }] }

#guard match validate #[schema] (nestedInitializerCarrierLoanFixture false) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard preparationHasDiagnosticAt (nestedInitializerCarrierLoanFixture true)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨19⟩

private def projectedCallResultCarrierLoanFixture (readSelectedOwner : Bool) : RawUnit :=
  let raw := projectedTupleCarrierLoanFixture readSelectedOwner
  let ns := raw.namespaces[0]!
  let secondMove : Expr := {
    loc := ⟨13⟩, typeId := ⟨3⟩, kind := .operation (.move ⟨1⟩) #[] #[] }
  let packedResult : Expr := {
    loc := ⟨13⟩, typeId := ⟨4⟩,
    kind := .operation (.primitive .tuple) #[] #[⟨9⟩, ⟨25⟩] }
  let callee := ns.functions[1]!
  { raw with namespaces := #[{
      ns with
      expressions := (ns.expressions ++ #[secondMove, packedResult]).set! 18 {
        ns.expressions[18]! with kind := .operation (.call (.function {
          namespaceId := ⟨0⟩, name := ⟨1⟩ })) #[] #[⟨1⟩, ⟨10⟩] }
      functions := ns.functions.set! 1 {
        callee with
        signature := { callee.signature with results := #[typeUse 4 13] }
        body := .structured ⟨26⟩ } }] }

#guard match validate #[schema] (projectedCallResultCarrierLoanFixture false) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard preparationHasDiagnosticAt (projectedCallResultCarrierLoanFixture true)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨19⟩

private def projectedMultiResultCarrierLoanFixture (readSelectedOwner : Bool) : RawUnit :=
  let raw := projectedCallResultCarrierLoanFixture readSelectedOwner
  let ns := raw.namespaces[0]!
  let callee := ns.functions[1]!
  { raw with namespaces := #[{
      ns with functions := ns.functions.set! 1 {
        callee with signature := {
          callee.signature with results := #[typeUse 2 13, typeUse 3 13] } } }] }

#guard match validate #[schema] (projectedMultiResultCarrierLoanFixture false) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard preparationHasDiagnosticAt (projectedMultiResultCarrierLoanFixture true)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨19⟩

private def projectedStructCallResultCarrierFixture (readSelectedOwner : Bool) : RawUnit :=
  let raw := projectedCallResultCarrierLoanFixture readSelectedOwner
  let ns := raw.namespaces[0]!
  let pair : QualifiedRef := { namespaceId := ⟨0⟩, name := ⟨2⟩ }
  { raw with
    tables := {
      raw.tables with
      types := raw.tables.types.set! 4 (.nominal ⟨2⟩ #[])
      names := raw.tables.names ++ #[
        { namespaceId := ⟨0⟩, name := "CallPair" },
        { namespaceId := ⟨0⟩, name := "first" },
        { namespaceId := ⟨0⟩, name := "second" }] }
    namespaces := #[{
      ns with
      expressions := ns.expressions.set! 26 {
        ns.expressions[26]! with
          kind := .operation (.call (.constructor pair)) #[] #[⟨9⟩, ⟨25⟩] }
      places := ns.places.set! 7 (.field ⟨6⟩ ⟨⟨0⟩, ⟨2⟩⟩ ⟨3⟩)
      structs := #[{
        loc := ⟨13⟩, name := ⟨2⟩, fields := #[
          { loc := ⟨13⟩, name := ⟨3⟩, type := typeUse 2 13 },
          { loc := ⟨13⟩, name := ⟨4⟩, type := typeUse 3 13 }] }] }] }

#guard match validate #[schema] (projectedStructCallResultCarrierFixture false) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard preparationHasDiagnosticAt (projectedStructCallResultCarrierFixture true)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨19⟩

private def projectedEnumCallResultCarrierFixture (readSelectedOwner : Bool) : RawUnit :=
  let raw := projectedStructCallResultCarrierFixture readSelectedOwner
  let ns := raw.namespaces[0]!
  let pair : QualifiedRef := { namespaceId := ⟨0⟩, name := ⟨2⟩ }
  { raw with
    tables := { raw.tables with names := raw.tables.names ++ #[
      { namespaceId := ⟨0⟩, name := "Pair" },
      { namespaceId := ⟨0⟩, name := "Empty" }] }
    namespaces := #[{
      ns with
      expressions := ns.expressions.set! 26 {
        ns.expressions[26]! with kind := (.operation
          (.call (.constructor pair (some "Pair"))) #[] #[⟨9⟩, ⟨25⟩]) }
      places := (ns.places.set! 7 (.field ⟨8⟩ ⟨⟨0⟩, ⟨2⟩⟩ ⟨3⟩)).push (.downcast ⟨6⟩ ⟨5⟩)
      structs := #[{
        loc := ⟨13⟩, name := ⟨2⟩, variants := #[
          { loc := ⟨13⟩, name := ⟨5⟩, fields := #[
            { loc := ⟨13⟩, name := ⟨3⟩, type := typeUse 2 13 },
            { loc := ⟨13⟩, name := ⟨4⟩, type := typeUse 3 13 }] },
          { loc := ⟨13⟩, name := ⟨6⟩ }] }] }] }

#guard match validate #[schema] (projectedEnumCallResultCarrierFixture false) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard preparationHasDiagnosticAt (projectedEnumCallResultCarrierFixture true)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨19⟩

private def projectedRecursiveCallResultCarrierFixture
    (readSelectedOwner : Bool) : RawUnit :=
  let raw := projectedEnumCallResultCarrierFixture readSelectedOwner
  let ns := raw.namespaces[0]!
  { raw with
    tables := { raw.tables with names := raw.tables.names ++ #[
      { namespaceId := ⟨0⟩, name := "Recursive" },
      { namespaceId := ⟨0⟩, name := "next" }] }
    namespaces := #[{
      ns with
      structs := ns.structs.set! 0 {
        ns.structs[0]! with variants := ns.structs[0]!.variants.push {
          loc := ⟨13⟩, name := ⟨7⟩, fields := #[{
            loc := ⟨13⟩, name := ⟨8⟩, type := typeUse 4 13 }] } } }] }

#guard match validate #[schema] (projectedRecursiveCallResultCarrierFixture false) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard preparationHasDiagnosticAt (projectedRecursiveCallResultCarrierFixture true)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨19⟩

private def projectedGenericCallResultCarrierFixture (readSelectedOwner : Bool) : RawUnit :=
  let raw := projectedStructCallResultCarrierFixture readSelectedOwner
  let ns := raw.namespaces[0]!
  let pair : QualifiedRef := { namespaceId := ⟨0⟩, name := ⟨2⟩ }
  let parameterReference : Ty := .reference {
    profile := .rust, kind := .mutable, referent := ⟨1⟩, lifetime := ⟨2⟩ }
  { raw with
    tables := {
      raw.tables with
      lifetimes := raw.tables.lifetimes.push { kind := .parameter 0, loc := ⟨13⟩ }
      types := (raw.tables.types.set! 4 (.nominal ⟨2⟩ #[.lifetime ⟨0⟩])).push
        parameterReference }
    namespaces := #[{
      ns with
      expressions := ns.expressions.set! 26 {
        ns.expressions[26]! with kind := (.operation (.call (.constructor pair))
          #[.lifetime ⟨0⟩] #[⟨9⟩, ⟨25⟩]) }
      structs := ns.structs.set! 0 {
        ns.structs[0]! with
        generics := #[{ name := "a", kind := .lifetime, loc := ⟨13⟩ }]
        fields := ns.structs[0]!.fields.set! 0 {
          ns.structs[0]!.fields[0]! with type := typeUse 5 13 } } }] }

#guard match validate #[schema] (projectedGenericCallResultCarrierFixture false) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard preparationHasDiagnosticAt (projectedGenericCallResultCarrierFixture true)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨19⟩

private def projectedGenericTypeCallResultCarrierFixture
    (readSelectedOwner : Bool) : RawUnit :=
  let raw := projectedStructCallResultCarrierFixture readSelectedOwner
  let ns := raw.namespaces[0]!
  let pair : QualifiedRef := { namespaceId := ⟨0⟩, name := ⟨2⟩ }
  let parameterType : Ty := .typeParameter 0
  let referenceArgument := typeUse 2 13
  { raw with
    tables := {
      raw.tables with
      types := (raw.tables.types.set! 4
        (.nominal ⟨2⟩ #[.typeArg referenceArgument])).push parameterType }
    namespaces := #[{
      ns with
      expressions := ns.expressions.set! 26 {
        ns.expressions[26]! with kind := (.operation (.call (.constructor pair))
          #[.typeArg referenceArgument] #[⟨9⟩, ⟨25⟩]) }
      structs := ns.structs.set! 0 {
        ns.structs[0]! with
        generics := #[{ name := "T", kind := .typeArg, loc := ⟨13⟩ }]
        fields := ns.structs[0]!.fields.set! 0 {
          ns.structs[0]!.fields[0]! with type := typeUse 5 13 } } }] }

#guard match validate #[schema] (projectedGenericTypeCallResultCarrierFixture false) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard preparationHasDiagnosticAt (projectedGenericTypeCallResultCarrierFixture true)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨19⟩

private def predicatedCallResultFixture : RawUnit :=
  let raw := selectedCallResultFixture false
  let ns := raw.namespaces[0]!
  let callee := ns.functions[1]!
  { raw with namespaces := #[{ ns with functions := ns.functions.set! 1 {
      callee with signature := { callee.signature with predicates :=
        callee.signature.predicates.push <| .lifetimeOutlives ⟨1⟩ ⟨0⟩ } } }] }

#guard preparationHasDiagnosticAt predicatedCallResultFixture
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨17⟩

private def typeParameterCallResultFixture (readSelectedOwner : Bool) : RawUnit :=
  let raw := selectedCallResultFixture readSelectedOwner
  let ns := raw.namespaces[0]!
  let callee := ns.functions[1]!
  let typeArgument : GenericArgument := .typeArg (typeUse 2 19)
  let binder : GenericBinder := {
    name := "T", kind := .typeArg, loc := ⟨13⟩ }
  let call : Expr := {
    ns.expressions[12]! with kind := .operation (.call (.function {
      namespaceId := ⟨0⟩, name := ⟨1⟩ })) #[typeArgument] #[⟨11⟩, ⟨10⟩] }
  { raw with
    tables := { raw.tables with types := raw.tables.types.push (.typeParameter 0) }
    namespaces := #[{
      ns with
      expressions := (ns.expressions.set! 9 {
        ns.expressions[9]! with typeId := ⟨4⟩ }).set! 12 call
      functions := ns.functions.set! 1 { callee with
        signature := {
          callee.signature with
          generics := #[binder]
          parameters := callee.signature.parameters.set! 0 {
            callee.signature.parameters[0]! with typeUse := typeUse 4 13 }
          results := #[typeUse 4 13] }
        locals := callee.locals.set! 0 {
          callee.locals[0]! with type := typeUse 4 13 } } }] }

#guard match validate #[schema] (typeParameterCallResultFixture false) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard preparationHasDiagnosticAt (typeParameterCallResultFixture true)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨17⟩

private def nominalCallResultFixture (readSelectedOwner : Bool) : RawUnit :=
  let raw := selectedCallResultFixture readSelectedOwner
  let ns := raw.namespaces[0]!
  let firstBox : QualifiedRef := { namespaceId := ⟨0⟩, name := ⟨2⟩ }
  let secondBox : QualifiedRef := { namespaceId := ⟨0⟩, name := ⟨4⟩ }
  let firstArgument : Expr := {
    loc := ⟨19⟩, typeId := ⟨4⟩,
    kind := .operation (.call (.constructor firstBox)) #[] #[⟨11⟩] }
  let secondArgument : Expr := {
    loc := ⟨19⟩, typeId := ⟨5⟩,
    kind := .operation (.call (.constructor secondBox)) #[] #[⟨10⟩] }
  let call : Expr := {
    ns.expressions[12]! with typeId := ⟨4⟩, kind := .operation (.call (.function {
      namespaceId := ⟨0⟩, name := ⟨1⟩ })) #[] #[⟨18⟩, ⟨19⟩] }
  let callee := ns.functions[1]!
  { raw with
    tables := {
      raw.tables with
      types := raw.tables.types ++ #[.nominal ⟨2⟩ #[], .nominal ⟨4⟩ #[]]
      names := raw.tables.names ++ #[
        { namespaceId := ⟨0⟩, name := "FirstBox" },
        { namespaceId := ⟨0⟩, name := "first" },
        { namespaceId := ⟨0⟩, name := "SecondBox" },
        { namespaceId := ⟨0⟩, name := "second" }] }
    namespaces := #[{
      ns with
      expressions := (ns.expressions ++ #[firstArgument, secondArgument])
        |>.set! 9 { ns.expressions[9]! with typeId := ⟨4⟩ }
        |>.set! 12 call
        |>.set! 15 { ns.expressions[15]! with
          kind := .operation (.read ⟨6⟩) #[] #[] }
      patterns := ns.patterns.set! 3 { ns.patterns[3]! with typeId := ⟨4⟩ }
      places := (ns.places.set! 5 (.field ⟨4⟩ ⟨⟨0⟩, ⟨2⟩⟩ ⟨3⟩)).push (.deref ⟨5⟩)
      structs := #[
        { loc := ⟨13⟩, name := ⟨2⟩, fields := #[{
            loc := ⟨13⟩, name := ⟨3⟩, type := typeUse 2 13 }] },
        { loc := ⟨13⟩, name := ⟨4⟩, fields := #[{
            loc := ⟨13⟩, name := ⟨5⟩, type := typeUse 3 13 }] }]
      functions := (ns.functions.set! 0 {
        ns.functions[0]! with locals := ns.functions[0]!.locals.set! 3 {
          ns.functions[0]!.locals[3]! with type := typeUse 4 19 } }).set! 1 {
            callee with
            signature := {
              parameters := #[
                { name := "reference", typeUse := typeUse 4 13 },
                { name := "other", typeUse := typeUse 5 13 }]
              results := #[typeUse 4 13] }
            locals := (callee.locals.set! 0 {
              callee.locals[0]! with type := typeUse 4 13 }).set! 1 {
                callee.locals[1]! with type := typeUse 5 13 } } }] }

#guard match validate #[schema] (nominalCallResultFixture false) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard preparationHasDiagnosticAt (nominalCallResultFixture true)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨17⟩

private def recursiveNominalCallResultFixture (readSelectedOwner : Bool) : RawUnit :=
  let raw := nominalCallResultFixture readSelectedOwner
  let ns := raw.namespaces[0]!
  let firstBox : QualifiedRef := { namespaceId := ⟨0⟩, name := ⟨2⟩ }
  { raw with
    tables := { raw.tables with names := raw.tables.names ++ #[
      { namespaceId := ⟨0⟩, name := "Base" },
      { namespaceId := ⟨0⟩, name := "Recursive" },
      { namespaceId := ⟨0⟩, name := "next" }] }
    namespaces := #[{
      ns with
      expressions := ns.expressions.set! 18 {
        ns.expressions[18]! with
          kind := .operation (.call (.constructor firstBox (some "Base"))) #[] #[⟨11⟩] }
      places := (ns.places.set! 5 (.field ⟨7⟩ ⟨⟨0⟩, ⟨2⟩⟩ ⟨3⟩)).push (.downcast ⟨4⟩ ⟨6⟩)
      structs := ns.structs.set! 0 {
        ns.structs[0]! with fields := #[], variants := #[
          { loc := ⟨13⟩, name := ⟨6⟩, fields := #[{
              loc := ⟨13⟩, name := ⟨3⟩, type := typeUse 2 13 }] },
          { loc := ⟨13⟩, name := ⟨7⟩, fields := #[{
              loc := ⟨13⟩, name := ⟨8⟩, type := typeUse 4 13 }] }] } }] }

#guard match validate #[schema] (recursiveNominalCallResultFixture false) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard preparationHasDiagnosticAt (recursiveNominalCallResultFixture true)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨17⟩

private def genericRecursiveNominalCallResultFixture (readSelectedOwner : Bool) : RawUnit :=
  let raw := recursiveNominalCallResultFixture readSelectedOwner
  let ns := raw.namespaces[0]!
  let firstBox : QualifiedRef := { namespaceId := ⟨0⟩, name := ⟨2⟩ }
  let parameterReference : Ty := .reference {
    profile := .rust, kind := .mutable, referent := ⟨1⟩, lifetime := ⟨2⟩ }
  { raw with
    tables := {
      raw.tables with
      lifetimes := raw.tables.lifetimes.push { kind := .parameter 0, loc := ⟨13⟩ }
      types := (raw.tables.types.set! 4 (.nominal ⟨2⟩ #[.lifetime ⟨0⟩])) ++ #[
        parameterReference, .nominal ⟨2⟩ #[.lifetime ⟨2⟩]] }
    namespaces := #[{
      ns with
      expressions := ns.expressions.set! 18 {
        ns.expressions[18]! with kind := (.operation
          (.call (.constructor firstBox (some "Base"))) #[.lifetime ⟨0⟩] #[⟨11⟩]) }
      structs := ns.structs.set! 0 {
        ns.structs[0]! with
        generics := #[{ name := "a", kind := .lifetime, loc := ⟨13⟩ }],
        variants := (ns.structs[0]!.variants.set! 0 {
          ns.structs[0]!.variants[0]! with fields := #[{
            loc := ⟨13⟩, name := ⟨3⟩, type := typeUse 6 13 }] }).set! 1 {
              ns.structs[0]!.variants[1]! with fields := #[{
                loc := ⟨13⟩, name := ⟨8⟩, type := typeUse 7 13 }] } } }] }

#guard match validate #[schema] (genericRecursiveNominalCallResultFixture false) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard preparationHasDiagnosticAt (genericRecursiveNominalCallResultFixture true)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨17⟩

private def changingGenericRecursiveNominalFixture : RawUnit :=
  let raw := genericRecursiveNominalCallResultFixture false
  { raw with tables := { raw.tables with
      types := raw.tables.types.set! 7 (.nominal ⟨2⟩ #[.lifetime ⟨1⟩]) } }

#guard match validate #[schema] changingGenericRecursiveNominalFixture with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

private def aggregateCallParameterFixture (predicated : Bool) : RawUnit :=
  let raw := selectedCallResultFixture false
  let ns := raw.namespaces[0]!
  let aggregateArgument : Expr := {
    loc := ⟨19⟩, typeId := ⟨4⟩,
    kind := .operation (.primitive .tuple) #[] #[⟨10⟩] }
  let callKind : ExprKind := .operation
    (.call (.function { namespaceId := ⟨0⟩, name := ⟨1⟩ })) #[] #[⟨11⟩, ⟨18⟩]
  let callee := ns.functions[1]!
  let signature := { callee.signature with
    parameters := callee.signature.parameters.set! 1 {
      callee.signature.parameters[1]! with typeUse := typeUse 4 13 }
    predicates := if predicated then
      callee.signature.predicates.push <| .lifetimeOutlives ⟨1⟩ ⟨0⟩
      else callee.signature.predicates }
  { raw with
    tables := { raw.tables with types := raw.tables.types.push (.tuple #[⟨3⟩]) }
    namespaces := #[{
      ns with
      expressions := (ns.expressions.push aggregateArgument).set! 12 {
        ns.expressions[12]! with kind := callKind }
      functions := ns.functions.set! 1 { callee with
        signature
        locals := callee.locals.set! 1 {
          callee.locals[1]! with type := typeUse 4 13 } } }] }

#guard match validate #[schema] (aggregateCallParameterFixture false) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard preparationHasDiagnosticAt (aggregateCallParameterFixture true)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨17⟩

private def projectedTuplePatternLoanFixture : RawUnit :=
  let raw := selectedCallResultFixture false
  let ns := raw.namespaces[0]!
  let firstPattern : Pattern := {
    loc := ⟨19⟩, typeId := ⟨2⟩, kind := .variable ⟨3⟩ }
  let discardedPattern : Pattern := {
    loc := ⟨19⟩, typeId := ⟨3⟩, kind := .wildcard }
  { raw with
    tables := { raw.tables with types := raw.tables.types.push (.tuple #[⟨2⟩, ⟨3⟩]) }
    namespaces := #[{
      ns with
      expressions := ns.expressions.set! 12 { ns.expressions[12]! with
        typeId := ⟨4⟩, kind := .operation (.primitive .tuple) #[] #[⟨11⟩, ⟨10⟩] }
      patterns := (ns.patterns ++ #[firstPattern, discardedPattern]).set! 3 {
          ns.patterns[3]! with typeId := ⟨4⟩, kind := .tuple #[⟨4⟩, ⟨5⟩] } }] }

#guard match validate #[schema] projectedTuplePatternLoanFixture with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

private def projectedTupleAssignmentLoanFixture : RawUnit :=
  let raw := projectedTuplePatternLoanFixture
  let ns := raw.namespaces[0]!
  let assignment : Expr := {
    loc := ⟨19⟩, typeId := ⟨0⟩, kind := .assignPattern ⟨3⟩ ⟨12⟩ }
  { raw with namespaces := #[{
      ns with
      expressions := (ns.expressions.push assignment).set! 17 {
        ns.expressions[17]! with kind := .block #[⟨18⟩] (some ⟨16⟩) }
      functions := ns.functions.set! 0 {
        ns.functions[0]! with locals := ns.functions[0]!.locals.set! 3 {
          ns.functions[0]!.locals[3]! with mutable := true } } }] }

#guard match validate #[schema] projectedTupleAssignmentLoanFixture with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

private def projectedConstructorPatternLoanFixture : RawUnit :=
  let raw := selectedCallResultFixture false
  let ns := raw.namespaces[0]!
  let pair : QualifiedRef := { namespaceId := ⟨0⟩, name := ⟨2⟩ }
  let firstPattern : Pattern := {
    loc := ⟨19⟩, typeId := ⟨2⟩, kind := .variable ⟨3⟩ }
  let discardedPattern : Pattern := {
    loc := ⟨19⟩, typeId := ⟨3⟩, kind := .wildcard }
  { raw with
    tables := { raw.tables with
      types := raw.tables.types.push (.nominal ⟨2⟩ #[])
      names := raw.tables.names ++ #[
        { namespaceId := ⟨0⟩, name := "PairRefs" },
        { namespaceId := ⟨0⟩, name := "first" },
        { namespaceId := ⟨0⟩, name := "second" }] }
    namespaces := #[{
      ns with
      expressions := ns.expressions.set! 12 { ns.expressions[12]! with
        typeId := ⟨4⟩,
        kind := .operation (.call (.constructor pair)) #[] #[⟨11⟩, ⟨10⟩] }
      patterns := (ns.patterns ++ #[firstPattern, discardedPattern]).set! 3 {
        ns.patterns[3]! with typeId := ⟨4⟩, kind :=
          (.constructor ⟨2⟩ #[] none #[⟨4⟩, ⟨5⟩]) }
      structs := ns.structs.push {
        loc := ⟨19⟩, name := ⟨2⟩, fields := #[
          { loc := ⟨19⟩, name := ⟨3⟩, type := typeUse 2 19 },
          { loc := ⟨19⟩, name := ⟨4⟩, type := typeUse 3 19 }] } }] }

#guard match validate #[schema] projectedConstructorPatternLoanFixture with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

private def projectedConstructorFieldLoanFixture
    (selectSecond readFirstOwner : Bool) : RawUnit :=
  let raw := selectedCallResultFixture readFirstOwner
  let ns := raw.namespaces[0]!
  let pair : QualifiedRef := { namespaceId := ⟨0⟩, name := ⟨2⟩ }
  let selectedField : NameId := if selectSecond then ⟨4⟩ else ⟨3⟩
  { raw with
    tables := { raw.tables with
      types := raw.tables.types.push (.nominal ⟨2⟩ #[])
      names := raw.tables.names ++ #[
        { namespaceId := ⟨0⟩, name := "PairRefs" },
        { namespaceId := ⟨0⟩, name := "first" },
        { namespaceId := ⟨0⟩, name := "second" }] }
    namespaces := #[{
      ns with
      expressions := ns.expressions
        |>.set! 12 { ns.expressions[12]! with typeId := ⟨4⟩, kind :=
          (.operation (.call (.constructor pair)) #[] #[⟨11⟩, ⟨10⟩]) }
        |>.set! 15 { ns.expressions[15]! with kind := .operation (.read ⟨6⟩) #[] #[] }
      patterns := ns.patterns.set! 3 { ns.patterns[3]! with typeId := ⟨4⟩ }
      places := (ns.places.set! 5 (.field ⟨4⟩ ⟨⟨0⟩, ⟨2⟩⟩ selectedField)).push (.deref ⟨5⟩)
      structs := ns.structs.push {
        loc := ⟨19⟩, name := ⟨2⟩, fields := #[
          { loc := ⟨19⟩, name := ⟨3⟩, type := typeUse 2 19 },
          { loc := ⟨19⟩, name := ⟨4⟩, type := typeUse 3 19 }] }
      functions := ns.functions.set! 0 {
        ns.functions[0]! with locals := ns.functions[0]!.locals.set! 3 {
          ns.functions[0]!.locals[3]! with type := typeUse 4 19 } } }] }

#guard match validate #[schema] (projectedConstructorFieldLoanFixture false false) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard match validate #[schema] (projectedConstructorFieldLoanFixture true true) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard preparationHasDiagnosticAt (projectedConstructorFieldLoanFixture false true)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨17⟩

private def movedProjectedConstructorFieldLoanFixture
    (selectSecond readFirstOwner : Bool) : RawUnit :=
  let raw := projectedConstructorFieldLoanFixture selectSecond readFirstOwner
  let ns := raw.namespaces[0]!
  let selectedField : NameId := if selectSecond then ⟨4⟩ else ⟨3⟩
  let moveAggregate : Expr := {
    loc := ⟨19⟩, typeId := ⟨4⟩, kind := .operation (.move ⟨4⟩) #[] #[] }
  let aliasLet : Expr := {
    loc := ⟨19⟩, typeId := ⟨1⟩, kind := .letDecl ⟨4⟩ (some ⟨18⟩) ⟨16⟩ }
  { raw with namespaces := #[{
      ns with
      expressions := (ns.expressions ++ #[moveAggregate, aliasLet])
        |>.set! 17 { ns.expressions[17]! with
          kind := .letDecl ⟨3⟩ (some ⟨12⟩) ⟨19⟩ }
      patterns := ns.patterns.push {
        loc := ⟨19⟩, typeId := ⟨4⟩, kind := .variable ⟨4⟩ }
      places := (ns.places.push (.localVar ⟨4⟩))
        |>.set! 5 (.field ⟨7⟩ ⟨⟨0⟩, ⟨2⟩⟩ selectedField)
      functions := ns.functions.set! 0 {
        ns.functions[0]! with locals := ns.functions[0]!.locals.push {
          id := ⟨4⟩, name := "alias", type := typeUse 4 19,
          mutable := false, loc := ⟨19⟩ } } }] }

#guard match validate #[schema] (movedProjectedConstructorFieldLoanFixture false false) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard match validate #[schema] (movedProjectedConstructorFieldLoanFixture true true) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard preparationHasDiagnosticAt (movedProjectedConstructorFieldLoanFixture false true)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨17⟩

private def conditionalMovedProjectedConstructorFieldLoanFixture
    (selectSecond readFirstOwner : Bool) : RawUnit :=
  let raw := movedProjectedConstructorFieldLoanFixture selectSecond readFirstOwner
  let ns := raw.namespaces[0]!
  let condition : Expr := {
    loc := ⟨19⟩, typeId := ⟨5⟩, kind := .value (.bool true) }
  let selected : Expr := {
    loc := ⟨19⟩, typeId := ⟨4⟩, kind := .ifElse ⟨20⟩ ⟨18⟩ (some ⟨18⟩) }
  { raw with
    tables := { raw.tables with types := raw.tables.types.push .bool }
    namespaces := #[{ ns with
      expressions := (ns.expressions ++ #[condition, selected]).set! 19 {
        ns.expressions[19]! with kind := .letDecl ⟨4⟩ (some ⟨21⟩) ⟨16⟩ } }] }

#guard match validate #[schema]
    (conditionalMovedProjectedConstructorFieldLoanFixture false false) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard match validate #[schema]
    (conditionalMovedProjectedConstructorFieldLoanFixture true true) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard preparationHasDiagnosticAt
  (conditionalMovedProjectedConstructorFieldLoanFixture false true)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨17⟩

private def localAggregateCallFieldLoanFixture
    (selectSecond readFirstOwner : Bool) : RawUnit :=
  let raw := projectedConstructorFieldLoanFixture selectSecond readFirstOwner
  let ns := raw.namespaces[0]!
  let constructor := ns.expressions[12]!
  let call : Expr := {
    constructor with kind := (.operation
      (.call (.function { namespaceId := ⟨0⟩, name := ⟨1⟩ })) #[] #[⟨18⟩]) }
  let callee := ns.functions[1]!
  { raw with namespaces := #[{
      ns with
      expressions := (ns.expressions.push constructor)
        |>.set! 9 { ns.expressions[9]! with
          typeId := ⟨4⟩, kind := .operation (.move ⟨0⟩) #[] #[] }
        |>.set! 12 call
      functions := ns.functions.set! 1 {
        callee with
        signature := { callee.signature with
          parameters := #[{
            callee.signature.parameters[0]! with typeUse := typeUse 4 13 }]
          results := #[typeUse 4 13] }
        locals := #[{
          callee.locals[0]! with type := typeUse 4 13 }] } }] }

#guard match validate #[schema] (localAggregateCallFieldLoanFixture false false) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard match validate #[schema] (localAggregateCallFieldLoanFixture true true) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard preparationHasDiagnosticAt (localAggregateCallFieldLoanFixture false true)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨17⟩

private def crossNamespaceAggregateCallLoanFixture
    (selectSecond readFirstOwner : Bool) : RawUnit :=
  let raw := localAggregateCallFieldLoanFixture selectSecond readFirstOwner
  let source := raw.namespaces[0]!
  let callee := source.functions[1]!
  let targetName : NameId := ⟨5⟩
  let expressions := source.expressions.set! 12 {
    source.expressions[12]! with kind := (.operation
      (.call (.function { namespaceId := ⟨1⟩, name := targetName })) #[] #[⟨18⟩]) }
  let source := {
    source with
    expressions
    functions := #[source.functions[0]!, callee] }
  let target : Namespace RawBody := {
    loc := ⟨19⟩
    identity := ⟨1⟩
    profile := some .rust
    expressions
    patterns := source.patterns
    places := source.places
    functions := #[{ callee with name := targetName }] }
  { raw with
    tables := { raw.tables with
      namespaces := raw.tables.namespaces.push { segments := #["external"] }
      names := raw.tables.names.push {
        namespaceId := ⟨1⟩, name := "identity_pair" } }
    namespaces := #[source, target] }

#guard match validate #[schema] (crossNamespaceAggregateCallLoanFixture false false) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard match validate #[schema] (crossNamespaceAggregateCallLoanFixture true true) with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard preparationHasDiagnosticAt (crossNamespaceAggregateCallLoanFixture false true)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨17⟩

private def aggregateCallLifetimeRelationFixture : RawUnit :=
  let raw := aggregateCallParameterFixture false
  let ns := raw.namespaces[0]!
  let callee := ns.functions[1]!
  { raw with
    tables := { raw.tables with types := raw.tables.types.push (.tuple #[⟨2⟩]) }
    namespaces := #[{
      ns with
      expressions := ns.expressions.set! 13 {
        ns.expressions[13]! with kind := .value (.integer 0) }
      functions := ns.functions.set! 1 { callee with
        signature := { callee.signature with
          parameters := callee.signature.parameters.set! 1 {
            callee.signature.parameters[1]! with typeUse := typeUse 5 13 } }
        locals := callee.locals.set! 1 {
          callee.locals[1]! with type := typeUse 5 13 } } }] }

private def aggregateCallLifetimeRelations? : Option (Array LifetimeRelationFact) := do
  let checked ← (validate #[schema] aggregateCallLifetimeRelationFixture).toOption
  let executable ← (prepareExecution #[semantics] checked).toOption
  let certificate ← executable.borrowCertificates.find? (·.functionId == ⟨0⟩)
  some certificate.lifetimeRelations

#guard aggregateCallLifetimeRelations?.any fun relations =>
  relations.contains { longer := ⟨1⟩, shorter := ⟨0⟩ }

private def aggregateCallResultLifetimeFixture : RawUnit :=
  let raw := selectedCallResultFixture false
  let ns := raw.namespaces[0]!
  let packed : Expr := {
    loc := ⟨19⟩, typeId := ⟨4⟩,
    kind := .operation (.primitive .tuple) #[] #[⟨9⟩] }
  let dropResult : Expr := {
    loc := ⟨19⟩, typeId := ⟨0⟩,
    kind := .operation (.drop ⟨4⟩) #[] #[] }
  let callee := ns.functions[1]!
  { raw with
    tables := { raw.tables with types := raw.tables.types ++ #[
      .tuple #[⟨2⟩], .tuple #[⟨3⟩]] }
    namespaces := #[{
      ns with
      expressions := (ns.expressions ++ #[packed, dropResult])
        |>.set! 12 { ns.expressions[12]! with typeId := ⟨5⟩ }
        |>.set! 16 { ns.expressions[16]! with
          kind := .block #[⟨13⟩, ⟨19⟩] (some ⟨0⟩) }
      patterns := ns.patterns.set! 3 { ns.patterns[3]! with typeId := ⟨5⟩ }
      functions := (ns.functions.set! 0 {
        ns.functions[0]! with locals := ns.functions[0]!.locals.set! 3 {
          ns.functions[0]!.locals[3]! with type := typeUse 5 19 } }).set! 1 {
            callee with
            signature := { callee.signature with results := #[typeUse 4 13] }
            body := .structured ⟨18⟩ } }] }

private def aggregateCallResultLifetimeRelations? :
    Option (Array LifetimeRelationFact) := do
  let checked ← (validate #[schema] aggregateCallResultLifetimeFixture).toOption
  let executable ← (prepareExecution #[semantics] checked).toOption
  let certificate ← executable.borrowCertificates.find? (·.functionId == ⟨0⟩)
  some certificate.lifetimeRelations

#guard aggregateCallResultLifetimeRelations?.any fun relations =>
  relations.contains { longer := ⟨0⟩, shorter := ⟨1⟩ }

private def loanHolders? (raw : RawUnit) : Option (Array LocalId) := do
  let checked ← (validate #[schema] raw).toOption
  let executable ← (prepareExecution #[semantics] checked).toOption
  let certificate ← executable.borrowCertificates.find? (·.functionId == ⟨0⟩)
  let loan ← certificate.loans.find? (·.expression == ⟨1⟩)
  some loan.holders

#guard loanHolders? (aliasedLoanFixture false) == some #[⟨1⟩, ⟨2⟩]
#guard loanHolders? (assignedLoanFixture false) == some #[⟨1⟩]

private def escapingLocalLoanFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  let function := ns.functions[0]!
  let returnLoan : Expr := {
    loc := ⟨11⟩, typeId := ⟨2⟩, kind := .operation (.move ⟨1⟩) #[] #[] }
  let referenceBody : Expr := {
    ns.expressions[6]! with typeId := ⟨2⟩, kind := .letDecl ⟨1⟩ (some ⟨8⟩) ⟨10⟩ }
  let referenceRoot : Expr := { ns.expressions[7]! with typeId := ⟨2⟩ }
  let expressions := (ns.expressions.push returnLoan).set! 6 referenceBody |>.set! 7 referenceRoot
  { fixture with namespaces := #[{
      ns with
      expressions
      functions := ns.functions.set! 0 {
        function with signature := { results := #[typeUse 2 10] } } }] }

#guard preparationHasDiagnosticAt escapingLocalLoanFixture
  "LIR-SEMANTIC-BORROW-ESCAPE" ⟨10⟩

private def nonEscapingLocalLoanFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  let borrow : Expr := {
    loc := ⟨14⟩, typeId := ⟨2⟩, kind := .operation (.borrow .mutable ⟨3⟩) #[] #[] }
  let consume : Expr := {
    loc := ⟨15⟩, typeId := ⟨2⟩, kind := .operation (.move ⟨4⟩) #[] #[] }
  let body : Expr := {
    loc := ⟨16⟩, typeId := ⟨2⟩, kind := .block #[⟨11⟩] (some ⟨9⟩) }
  let referenceLet : Expr := {
    loc := ⟨17⟩, typeId := ⟨2⟩, kind := .letDecl ⟨2⟩ (some ⟨10⟩) ⟨12⟩ }
  let valueLet : Expr := {
    loc := ⟨18⟩, typeId := ⟨2⟩, kind := .letDecl ⟨3⟩ (some ⟨0⟩) ⟨13⟩ }
  { fixture with namespaces := #[{
      ns with
      expressions := ns.expressions ++ #[borrow, consume, body, referenceLet, valueLet]
      patterns := ns.patterns ++ #[
        { loc := ⟨17⟩, typeId := ⟨2⟩, kind := .variable ⟨2⟩ },
        { loc := ⟨18⟩, typeId := ⟨1⟩, kind := .variable ⟨1⟩ }]
      places := ns.places ++ #[.localVar ⟨1⟩, .localVar ⟨2⟩]
      functions := ns.functions.set! 1 {
        ns.functions[1]! with
        body := .structured ⟨14⟩
        locals := ns.functions[1]!.locals ++ #[
          { id := ⟨1⟩, name := "value", type := typeUse 1 18,
            mutable := true, loc := ⟨18⟩ },
          { id := ⟨2⟩, name := "loan", type := typeUse 2 17,
            mutable := false, loc := ⟨17⟩ }] } }] }

#guard match validate #[schema] nonEscapingLocalLoanFixture with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

private def dereferenceConsumptionFixture (drop : Bool) : RawUnit :=
  let ns := fixture.namespaces[0]!
  let expression := if drop then
    { ns.expressions[3]! with kind := .operation (.drop ⟨2⟩) #[] #[] }
  else
    { ns.expressions[3]! with
      typeId := ⟨1⟩, kind := .operation (.move ⟨2⟩) #[] #[] }
  { fixture with namespaces := #[{
      ns with expressions := ns.expressions.set! 3 expression }] }

#guard preparationHasDiagnosticAt (dereferenceConsumptionFixture false)
  "LIR-SEMANTIC-BORROW-DEREF-CONSUME" ⟨3⟩

#guard preparationHasDiagnosticAt (dereferenceConsumptionFixture true)
  "LIR-SEMANTIC-BORROW-DEREF-CONSUME" ⟨3⟩

private def valueOperationFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  let sharedReference : Ty := .reference {
    profile := .rust, kind := .shared, referent := ⟨1⟩, lifetime := ⟨0⟩ }
  let expressions := ns.expressions ++ #[
    { loc := ⟨14⟩, typeId := ⟨3⟩,
      kind := .operation (.reference (.freeze true)) #[] #[⟨1⟩] },
    { loc := ⟨15⟩, typeId := ⟨1⟩,
      kind := .operation (.reference .dereference) #[] #[⟨10⟩] },
    { loc := ⟨16⟩, typeId := ⟨0⟩,
      kind := .operation (.reference .mutate) #[] #[⟨1⟩, ⟨2⟩] },
    { loc := ⟨17⟩, typeId := ⟨1⟩,
      kind := .operation (.reference .dereference) #[] #[⟨1⟩] },
    { loc := ⟨18⟩, typeId := ⟨1⟩, kind := .block #[⟨11⟩, ⟨12⟩] (some ⟨13⟩) },
    { loc := ⟨19⟩, typeId := ⟨1⟩, kind := .letDecl ⟨1⟩ (some ⟨1⟩) ⟨14⟩ },
    { loc := ⟨20⟩, typeId := ⟨1⟩, kind := .letDecl ⟨0⟩ (some ⟨0⟩) ⟨15⟩ }]
  let function := {
    ns.functions[0]! with
    loc := ⟨21⟩
    name := ⟨2⟩
    body := .structured ⟨16⟩ }
  { fixture with
    tables := {
      fixture.tables with
      locations := (Array.range 24).map fun index => {
        primary := some { file := ⟨0⟩, startByte := index, endByte := index + 1 } }
      types := fixture.tables.types.push sharedReference
      names := fixture.tables.names.push { namespaceId := ⟨0⟩, name := "value_operations" } }
    namespaces := #[{
      ns with
      expressions
      functions := ns.functions.push function }] }

private def valueExecutable? : Option ExecutableUnit := do
  let checked ← (validate #[schema] valueOperationFixture).toOption
  (prepareExecution #[semantics] checked).toOption

#guard match valueExecutable? with
  | some executable => match LeanerIR.Interpreter.run executable 32
      { namespaceId := ⟨0⟩, functionId := ⟨2⟩ } #[] with
    | .ok (state, outcome) =>
        outcome.value == .returned #[.integer 9] && state.pending.isEmpty
    | .error _ => false
  | none => false

private def badFreezeResultFixture : RawUnit :=
  let ns := valueOperationFixture.namespaces[0]!
  { valueOperationFixture with namespaces := #[{
      ns with expressions := ns.expressions.set! 10 {
        ns.expressions[10]! with typeId := ⟨2⟩ } }] }

#guard validationHasDiagnosticAt badFreezeResultFixture "LIR-SEMANTIC-TYPE" ⟨14⟩

private def immutableMutationFixture : RawUnit :=
  let ns := valueOperationFixture.namespaces[0]!
  { valueOperationFixture with namespaces := #[{
      ns with expressions := ns.expressions.set! 12 {
        ns.expressions[12]! with kind :=
          (.operation (.reference .mutate) #[] #[⟨10⟩, ⟨2⟩]) } }] }

#guard validationHasDiagnosticAt immutableMutationFixture "LIR-SEMANTIC-TYPE" ⟨16⟩

private def dynamicIndexFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  let indexExpression : Expr := {
    loc := ⟨14⟩, typeId := ⟨1⟩, kind := .block #[] (some ⟨2⟩) }
  let expressions := ns.expressions.push indexExpression |>.set! 4 {
    ns.expressions[4]! with kind := .operation (.read ⟨4⟩) #[] #[] }
  let expressions := expressions ++ #[
    { loc := ⟨14⟩, typeId := ⟨3⟩, kind := .value (.vector #[]) },
    { loc := ⟨14⟩, typeId := ⟨0⟩, kind := .assign ⟨3⟩ ⟨11⟩ }]
  let expressions := expressions.set! 5 {
    expressions[5]! with kind := .block #[⟨3⟩, ⟨12⟩] (some ⟨4⟩) }
  { fixture with
    tables := { fixture.tables with types := fixture.tables.types.push (.vector ⟨1⟩) }
    namespaces := #[{
      ns with
      expressions
      places := (ns.places.push (.localVar ⟨2⟩)).push (.index ⟨3⟩ ⟨10⟩)
      functions := ns.functions.set! 0 { ns.functions[0]! with
        locals := ns.functions[0]!.locals.push {
          id := ⟨2⟩, name := "items", type := { typeId := ⟨3⟩, loc := ⟨14⟩ }, loc := ⟨14⟩ } } }] }

#guard preparationHasCode dynamicIndexFixture "LIR-SEMANTIC-PLACE-INDEX"

private def projectedMoveFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with namespaces := #[{
      ns with expressions := ns.expressions.set! 4 {
        ns.expressions[4]! with kind := .operation (.move ⟨2⟩) #[] #[] } }] }

#guard preparationHasCode projectedMoveFixture "LIR-SEMANTIC-PLACE-CONSUME"

private def badBorrowResultFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with namespaces := #[{
      ns with expressions := ns.expressions.set! 1 {
        ns.expressions[1]! with typeId := ⟨1⟩ } }] }

#guard validationHasCode badBorrowResultFixture "LIR-SEMANTIC-TYPE"

private def badWriteArityFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with namespaces := #[{
      ns with expressions := ns.expressions.set! 3 {
        ns.expressions[3]! with kind := .operation (.write ⟨2⟩) #[] #[] } }] }

#guard validationHasCode badWriteArityFixture "LIR-SEMANTIC-ARITY"

private def immutableBorrowFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  let function := ns.functions[0]!
  { fixture with namespaces := #[{
      ns with functions := ns.functions.set! 0 {
        function with locals := function.locals.set! 0 {
          function.locals[0]! with mutable := false } } }] }

#guard preparationHasDiagnosticAt immutableBorrowFixture "LIR-SEMANTIC-TYPE" ⟨1⟩

private def badWriteTypeFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with
    tables := { fixture.tables with types := fixture.tables.types.push .bool }
    namespaces := #[{
      ns with
      expressions := ns.expressions.set! 2 {
        ns.expressions[2]! with typeId := ⟨3⟩, kind := .value (.bool true) } }] }


#guard validationHasDiagnosticAt badWriteTypeFixture "LIR-SEMANTIC-TYPE" ⟨3⟩

private def badReadTypeFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with namespaces := #[{
      ns with expressions := ns.expressions.set! 4 {
        ns.expressions[4]! with typeId := ⟨0⟩ } }] }

#guard validationHasDiagnosticAt badReadTypeFixture "LIR-SEMANTIC-TYPE" ⟨4⟩

private def badDirectCallArgumentFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with namespaces := #[{
      ns with expressions := ns.expressions.set! 8 {
        ns.expressions[8]! with kind := (.operation
          (.call (.function { namespaceId := ⟨0⟩, name := ⟨1⟩ })) #[] #[⟨0⟩]) } }] }

#guard validationHasDiagnosticAt badDirectCallArgumentFixture "LIR-SEMANTIC-TYPE" ⟨11⟩

private def prepared : ExecutableUnit := executable?.get (by native_decide)
private def valuePrepared : ExecutableUnit := valueExecutable?.get (by native_decide)

private theorem successfulRunHasDerivation (executable : ExecutableUnit)
    (fuel : Nat) (function : FunctionHandle) (arguments : Array RuntimeValue)
    (success : (LeanerIR.Interpreter.run executable fuel function arguments).isOk) :
    ∃ (finalState : RuntimeState) (outcome : LocatedOutcome),
      BigStep.EvalFunction executable function {} arguments finalState outcome.value := by
  generalize result_eq : LeanerIR.Interpreter.run executable fuel function arguments = result
  cases result with
  | error error => simp [result_eq, Except.isOk, Except.toBool] at success
  | ok result =>
      exact ⟨result.1, result.2,
        LeanerIR.Proofs.Interpreter.run_sound executable fuel function arguments {}
          result.1 result.2 result_eq⟩

example : ∃ (finalState : RuntimeState) (outcome : LocatedOutcome),
    BigStep.EvalFunction prepared { namespaceId := ⟨0⟩, functionId := ⟨0⟩ }
      {} #[] finalState outcome.value := by
  apply successfulRunHasDerivation prepared 32
    { namespaceId := ⟨0⟩, functionId := ⟨0⟩ } #[]
  native_decide

example : ∃ (finalState : RuntimeState) (outcome : LocatedOutcome),
    BigStep.EvalFunction valuePrepared { namespaceId := ⟨0⟩, functionId := ⟨2⟩ }
      {} #[] finalState outcome.value := by
  apply successfulRunHasDerivation valuePrepared 32
    { namespaceId := ⟨0⟩, functionId := ⟨2⟩ } #[]
  native_decide

end LeanerIR.Tests.References
