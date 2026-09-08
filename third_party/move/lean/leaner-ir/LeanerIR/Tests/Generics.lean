-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Interpreter
import LeanerIR.Validation.Check

namespace LeanerIR.Tests.Generics

open LeanerIR
open LeanerIR.Import
open LeanerIR.Validation

private def config : ProfileConfig := { profile := .rust, name := "rust-test" }
private def schema : ProfileSchema := { profile := .rust, name := "rust-test" }
private def semantics : SemanticProfile := {
  profile := .rust, name := "rust-test", classify := fun _ _ => none }

private def typeUse (typeId loc : Nat) : TypeUse := { typeId := ⟨typeId⟩, loc := ⟨loc⟩ }

private def typeBinder : GenericBinder := {
  name := "T", kind := .typeArg, abilities := #[.copy], loc := ⟨0⟩ }

private def typeArgument (typeId loc : Nat) : GenericArgument :=
  .typeArg (typeUse typeId loc)

private def functionRef (name : Nat) : QualifiedRef := {
  namespaceId := ⟨0⟩, name := ⟨name⟩ }

private def parameter (name : String) (loc : Nat) : Parameter := {
  name, typeUse := typeUse 3 loc }

private def localDecl (id : Nat) (name : String) (loc : Nat) : LocalDecl := {
  id := ⟨id⟩, name, type := typeUse 3 loc, loc := ⟨loc⟩ }

/-! One generic body is reused by direct calls and closures. `Box<T>` also
checks constructor/destructor substitution and direct field-place typing;
none of these declarations are monomorphized in LIR. -/
private def fixture : RawUnit where
  tables := {
    files := #[{ name := "generics.rs" }]
    locations := (Array.range 32).map fun index => {
      primary := some { file := ⟨0⟩, startByte := index, endByte := index + 1 } }
    origins := #[{ kind := .rustMir, location := ⟨0⟩ }]
    alignments := #[{
      source := ⟨0⟩, trust := .checked, description := "generic fixture" }]
    types := #[
      .unit,
      .bool,
      .integer (.bits 8) false,
      .typeParameter 0,
      .nominal ⟨3⟩ #[typeArgument 1 0],
      .nominal ⟨3⟩ #[typeArgument 2 0],
      .function #[⟨1⟩] ⟨1⟩,
      .function #[⟨2⟩] ⟨1⟩,
      .nominal ⟨3⟩ #[],
      .bytes,
      .nominal ⟨3⟩ #[typeArgument 4 13],
      .nominal ⟨3⟩ #[typeArgument 3 20],
      .character,
      .nominal ⟨3⟩ #[typeArgument 12 24]]
    namespaces := #[{ segments := #["test", "Generics"] }]
    names := #[
      { namespaceId := ⟨0⟩, name := "direct" },
      { namespaceId := ⟨0⟩, name := "identity" },
      { namespaceId := ⟨0⟩, name := "box_main" },
      { namespaceId := ⟨0⟩, name := "Box" },
      { namespaceId := ⟨0⟩, name := "value" },
      { namespaceId := ⟨0⟩, name := "closure_main" },
      { namespaceId := ⟨0⟩, name := "first" },
      { namespaceId := ⟨0⟩, name := "destruct_box" },
      { namespaceId := ⟨0⟩, name := "forward" }] }
  profiles := #[config]
  namespaces := #[{
    loc := ⟨0⟩
    identity := ⟨0⟩
    profile := some .rust
    expressions := #[
      { loc := ⟨0⟩, typeId := ⟨1⟩, kind := .value (.bool true) },
      { loc := ⟨1⟩, typeId := ⟨1⟩,
        kind := .operation (.call (.function (functionRef 1)))
          #[typeArgument 1 1] #[⟨0⟩] },
      { loc := ⟨2⟩, typeId := ⟨4⟩,
        kind := .operation (.call (.constructor (functionRef 3)))
          #[typeArgument 1 2] #[⟨0⟩] },
      { loc := ⟨3⟩, typeId := ⟨1⟩,
        kind := .operation (.call (.destructor (functionRef 3)))
          #[typeArgument 1 3] #[⟨2⟩] },
      { loc := ⟨4⟩, typeId := ⟨1⟩,
        kind := .operation (.read ⟨1⟩) #[] #[] },
      { loc := ⟨5⟩, typeId := ⟨1⟩,
        kind := .letDecl ⟨0⟩ (some ⟨2⟩) ⟨4⟩ },
      { loc := ⟨6⟩, typeId := ⟨1⟩, kind := .value (.bool false) },
      { loc := ⟨7⟩, typeId := ⟨6⟩,
        kind := .operation (.call (.closure (functionRef 6)))
          #[typeArgument 1 7] #[⟨0⟩] },
      { loc := ⟨8⟩, typeId := ⟨1⟩,
        kind := .operation (.call .invoke) #[] #[⟨7⟩, ⟨6⟩] },
      { loc := ⟨9⟩, typeId := ⟨3⟩, kind := .localVar ⟨0⟩ },
      { loc := ⟨10⟩, typeId := ⟨3⟩, kind := .localVar ⟨0⟩ },
      { loc := ⟨19⟩, typeId := ⟨3⟩,
        kind := .operation (.primitive .copyValue) #[] #[⟨9⟩] },
      { loc := ⟨20⟩, typeId := ⟨3⟩,
        kind := .operation (.call (.function (functionRef 1)))
          #[typeArgument 3 20] #[⟨9⟩] }]
    patterns := #[{
      loc := ⟨11⟩, typeId := ⟨4⟩, kind := .variable ⟨0⟩ }]
    places := #[.localVar ⟨0⟩,
      .field ⟨0⟩ { namespaceId := ⟨0⟩, name := ⟨3⟩ } ⟨4⟩]
    structs := #[{
      loc := ⟨12⟩
      name := ⟨3⟩
      generics := #[typeBinder]
      fields := #[{
        loc := ⟨12⟩, name := ⟨4⟩, type := typeUse 3 12 }] }]
    functions := #[
      {
        loc := ⟨13⟩
        name := ⟨0⟩
        profile := .rust
        signature := { results := #[typeUse 1 13] }
        body := .structured ⟨1⟩
        origin := ⟨0⟩
        alignment := ⟨0⟩
      },
      {
        loc := ⟨14⟩
        name := ⟨1⟩
        profile := .rust
        signature := {
          generics := #[typeBinder]
          parameters := #[parameter "value" 14]
          results := #[typeUse 3 14] }
        body := .structured ⟨11⟩
        origin := ⟨0⟩
        alignment := ⟨0⟩
        locals := #[localDecl 0 "value" 14]
      },
      {
        loc := ⟨15⟩
        name := ⟨2⟩
        profile := .rust
        signature := { results := #[typeUse 1 15] }
        body := .structured ⟨5⟩
        origin := ⟨0⟩
        alignment := ⟨0⟩
        locals := #[{
          id := ⟨0⟩, name := "box", type := typeUse 4 15,
          loc := ⟨15⟩ }]
      },
      {
        loc := ⟨16⟩
        name := ⟨5⟩
        profile := .rust
        signature := { results := #[typeUse 1 16] }
        body := .structured ⟨8⟩
        origin := ⟨0⟩
        alignment := ⟨0⟩
      },
      {
        loc := ⟨17⟩
        name := ⟨6⟩
        profile := .rust
        signature := {
          generics := #[typeBinder]
          parameters := #[parameter "capture" 17, parameter "argument" 17]
          results := #[typeUse 3 17] }
        body := .structured ⟨10⟩
        origin := ⟨0⟩
        alignment := ⟨0⟩
        locals := #[localDecl 0 "capture" 17, localDecl 1 "argument" 17]
      },
      {
        loc := ⟨18⟩
        name := ⟨7⟩
        profile := .rust
        signature := { results := #[typeUse 1 18] }
        body := .structured ⟨3⟩
        origin := ⟨0⟩
        alignment := ⟨0⟩
      },
      {
        loc := ⟨20⟩
        name := ⟨8⟩
        profile := .rust
        signature := {
          generics := #[typeBinder]
          parameters := #[parameter "value" 20]
          results := #[typeUse 3 20] }
        body := .structured ⟨12⟩
        origin := ⟨0⟩
        alignment := ⟨0⟩
        locals := #[
          localDecl 0 "value" 20,
          { id := ⟨1⟩, name := "boxed", type := typeUse 11 20, loc := ⟨20⟩ }]
      }] }]

private def executable? : Option ExecutableUnit := do
  let checked ← (validate #[schema] fixture).toOption
  (prepareExecution #[semantics] checked).toOption

private def handle (functionId : Nat) : FunctionHandle := {
  namespaceId := ⟨0⟩, functionId := ⟨functionId⟩ }

#guard match executable? with
  | some executable =>
      [0, 2, 3, 5].all fun functionId =>
        match Interpreter.run executable 32 (handle functionId) #[] with
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

private def validationHasDiagnosticAt (raw : RawUnit) (code : String)
    (loc : Option LocId) : Bool :=
  match validate #[schema] raw with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == code && diagnostic.primary == loc
  | .ok _ => false

private def badDirectArgumentFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with namespaces := #[{
      ns with expressions := ns.expressions.set! 0 {
        ns.expressions[0]! with typeId := ⟨2⟩, kind := .value (.integer 1) } }] }

#guard validationHasDiagnosticAt badDirectArgumentFixture "LIR-SEMANTIC-TYPE" (some ⟨1⟩)

private def badConstructorResultFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with namespaces := #[{
      ns with expressions := ns.expressions.set! 2 {
        ns.expressions[2]! with typeId := ⟨5⟩ } }] }

#guard validationHasDiagnosticAt badConstructorResultFixture "LIR-SEMANTIC-TYPE" (some ⟨2⟩)

private def badDestructorResultFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with namespaces := #[{
      ns with expressions := ns.expressions.set! 3 {
        ns.expressions[3]! with typeId := ⟨2⟩ } }] }

#guard validationHasDiagnosticAt badDestructorResultFixture "LIR-SEMANTIC-TYPE" (some ⟨3⟩)

private def badFieldResultFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with namespaces := #[{
      ns with expressions := ns.expressions.set! 4 {
        ns.expressions[4]! with typeId := ⟨2⟩ } }] }

#guard validationHasDiagnosticAt badFieldResultFixture "LIR-SEMANTIC-TYPE" (some ⟨4⟩)

private def badClosureTypeFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with namespaces := #[{
      ns with expressions := ns.expressions.set! 7 {
        ns.expressions[7]! with typeId := ⟨7⟩ } }] }

#guard validationHasDiagnosticAt badClosureTypeFixture "LIR-SEMANTIC-TYPE" (some ⟨7⟩)

private def badAbilityFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with namespaces := #[{
      ns with expressions := ns.expressions.set! 1 {
        ns.expressions[1]! with kind := (.operation (.call (.function (functionRef 1)))
          #[typeArgument 4 20] #[⟨0⟩]) } }] }

#guard validationHasDiagnosticAt badAbilityFixture "LIR-SEMANTIC-ABILITY" (some ⟨20⟩)

private def genericConstructorPatternFixture (childType : Nat := 1)
    (argumentType : Nat := 1) (fieldCount : Nat := 1) : RawUnit :=
  let ns := fixture.namespaces[0]!
  let child : PatternId := ⟨ns.patterns.size⟩
  let constructor : PatternId := ⟨ns.patterns.size + 1⟩
  let root : ExprId := ⟨ns.expressions.size⟩
  let childPattern : Pattern :=
    { loc := ⟨21⟩, typeId := ⟨childType⟩, kind := .wildcard }
  let constructorPattern : Pattern :=
    { loc := ⟨21⟩, typeId := ⟨4⟩,
      kind := .constructor ⟨3⟩ #[typeArgument argumentType 21] none
        (Array.replicate fieldCount child) }
  { fixture with namespaces := #[{ ns with
      patterns := (ns.patterns.push childPattern).push constructorPattern
      expressions := ns.expressions.push {
        loc := ⟨21⟩, typeId := ⟨1⟩,
        kind := .match_ ⟨2⟩ #[{ pattern := constructor, body := ⟨0⟩ }] }
      functions := ns.functions.set! 0 { ns.functions[0]! with body := .structured root } }] }

private def genericConstructorPatternPrepares (raw : RawUnit) : Bool :=
  match validate #[schema] raw with
  | .error _ => false
  | .ok checked => (prepareExecution #[semantics] checked).isOk

#guard genericConstructorPatternPrepares genericConstructorPatternFixture
#guard validationHasDiagnosticAt (genericConstructorPatternFixture (childType := 2))
  "LIR-SEMANTIC-TYPE" (some ⟨21⟩)
#guard validationHasDiagnosticAt (genericConstructorPatternFixture (argumentType := 2))
  "LIR-SEMANTIC-TYPE" (some ⟨21⟩)
#guard validationHasDiagnosticAt (genericConstructorPatternFixture (fieldCount := 0))
  "LIR-SEMANTIC-ARITY" (some ⟨21⟩)

private def genericFieldSelectFixture (resultType : Nat := 1) : RawUnit :=
  let ns := fixture.namespaces[0]!
  let root : ExprId := ⟨ns.expressions.size⟩
  { fixture with namespaces := #[{ ns with
      expressions := ns.expressions.push {
        loc := ⟨22⟩, typeId := ⟨resultType⟩,
        kind := .operation (.data (.select (functionRef 3) "value")) #[] #[⟨2⟩] }
      functions := ns.functions.set! 0 { ns.functions[0]! with body := .structured root } }] }

#guard genericConstructorPatternPrepares genericFieldSelectFixture
#guard validationHasDiagnosticAt (genericFieldSelectFixture 2)
  "LIR-SEMANTIC-TYPE" (some ⟨22⟩)

private def genericCharacterFieldSelectFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  let value : ExprId := ⟨ns.expressions.size⟩
  let boxed : ExprId := ⟨ns.expressions.size + 1⟩
  let root : ExprId := ⟨ns.expressions.size + 2⟩
  { fixture with namespaces := #[{ ns with
      expressions := ((ns.expressions.push {
          loc := ⟨24⟩, typeId := ⟨12⟩, kind := .value (.character 0x1f980) }).push {
          loc := ⟨24⟩, typeId := ⟨13⟩,
          kind := .operation (.call (.constructor (functionRef 3)))
            #[typeArgument 12 24] #[value] }).push {
          loc := ⟨24⟩, typeId := ⟨12⟩,
          kind := .operation (.data (.select (functionRef 3) "value")) #[] #[boxed] }
      functions := ns.functions.set! 0 { ns.functions[0]! with
        signature := { results := #[typeUse 12 24] }
        body := .structured root } }] }

#guard genericConstructorPatternPrepares genericCharacterFieldSelectFixture

private def genericFieldUpdateFixture (badReplacement : Bool := false) : RawUnit :=
  let ns := fixture.namespaces[0]!
  let replacement : ExprId := if badReplacement then ⟨ns.expressions.size⟩ else ⟨0⟩
  let root : ExprId := if badReplacement then ⟨ns.expressions.size + 1⟩
    else ⟨ns.expressions.size⟩
  let expressions := if badReplacement then ns.expressions.push {
      loc := ⟨23⟩, typeId := ⟨2⟩, kind := .value (.integer 7) }
    else ns.expressions
  { fixture with namespaces := #[{ ns with
      expressions := expressions.push {
        loc := ⟨23⟩, typeId := ⟨4⟩,
        kind := .operation (.data (.updateField (functionRef 3) "value")) #[]
          #[⟨2⟩, replacement] }
      functions := ns.functions.set! 0 { ns.functions[0]! with
        signature := { results := #[typeUse 4 13] }
        body := .structured root } }] }

#guard genericConstructorPatternPrepares genericFieldUpdateFixture
#guard validationHasDiagnosticAt (genericFieldUpdateFixture true)
  "LIR-SEMANTIC-TYPE" (some ⟨23⟩)

private def withDirectResultType (typeId : Nat) : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with namespaces := #[{
      ns with functions := ns.functions.set! 0 {
        ns.functions[0]! with signature := { results := #[typeUse typeId 13] } } }] }

#guard validationHasDiagnosticAt (withDirectResultType 8) "LIR-SEMANTIC-ARITY" (some ⟨13⟩)
#guard validationHasDiagnosticAt (withDirectResultType 10) "LIR-SEMANTIC-ABILITY" (some ⟨13⟩)

/- A nominal type whose name does not denote a struct or enum is rejected by
validation itself; the interned type table is authoritative semantic data. -/
private def danglingNominalTypeFixture : RawUnit :=
  { fixture with tables := { fixture.tables with
      types := fixture.tables.types.push (.nominal ⟨4⟩ #[]) } }

#guard validationHasDiagnosticAt danglingNominalTypeFixture "LIR-SEMANTIC-TARGET" none

private def traitReferenceFixture (traitName : Nat := 9)
    (arguments : Array GenericArgument := #[typeArgument 3 25])
    (callerAbilities : Array Ability := #[.copy]) : RawUnit :=
  let ns := fixture.namespaces[0]!
  let callerBinder : GenericBinder := {
    name := "U"
    kind := .typeArg
    abilities := callerAbilities
    predicates := #[.implements ⟨3⟩ {
      trait := { namespaceId := ⟨0⟩, name := ⟨traitName⟩ }
      arguments }]
    loc := ⟨25⟩ }
  { fixture with
    tables := { fixture.tables with
      types := fixture.tables.types.push (.typeParameter 0)
      names := fixture.tables.names ++ #[
        { namespaceId := ⟨0⟩, name := "CopyLike" },
        { namespaceId := ⟨0⟩, name := "NotATrait" }] }
    namespaces := #[{ ns with
      traits := #[{
        id := ⟨0⟩
        loc := ⟨25⟩
        name := ⟨9⟩
        generics := #[typeBinder] }]
      functions := ns.functions.set! 0 { ns.functions[0]! with
        signature := { ns.functions[0]!.signature with generics := #[callerBinder] } } }] }

#guard genericConstructorPatternPrepares traitReferenceFixture
#guard validationHasDiagnosticAt (traitReferenceFixture (arguments := #[]))
  "LIR-SEMANTIC-ARITY" (some ⟨25⟩)
#guard validationHasDiagnosticAt (traitReferenceFixture (traitName := 10))
  "LIR-SEMANTIC-TARGET" (some ⟨25⟩)
#guard validationHasDiagnosticAt (traitReferenceFixture (callerAbilities := #[]))
  "LIR-SEMANTIC-ABILITY" (some ⟨25⟩)

/- Expression annotations are not declaration-scoped during raw arena bounds
checking, so semantic preparation must reject their free or wrong-kind generic
parameters before constructing an executable unit. -/
private def expressionGenericScopeFixture (wrongKind : Bool := false) : RawUnit :=
  let ns := fixture.namespaces[0]!
  let generics : Array GenericBinder := if wrongKind then #[{
      name := "a", kind := .lifetime, loc := ⟨26⟩ }] else #[]
  { fixture with namespaces := #[{ ns with
      expressions := ns.expressions.set! 0 {
        ns.expressions[0]! with typeId := ⟨3⟩ }
      functions := ns.functions.set! 0 { ns.functions[0]! with
        signature := { ns.functions[0]!.signature with generics } } }] }

#guard validationHasDiagnosticAt expressionGenericScopeFixture
  "LIR-SEMANTIC-GENERIC-SCOPE" (some ⟨0⟩)
#guard validationHasDiagnosticAt (expressionGenericScopeFixture true)
  "LIR-SEMANTIC-GENERIC-SCOPE" (some ⟨0⟩)

private def expressionLifetimeScopeFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with
    tables := { fixture.tables with
      lifetimes := #[{ loc := ⟨26⟩, kind := .parameter 0 }]
      types := fixture.tables.types.push (.reference {
        profile := .rust, kind := .shared, referent := ⟨1⟩, lifetime := ⟨0⟩ }) }
    namespaces := #[{ ns with expressions := ns.expressions.set! 0 {
      ns.expressions[0]! with typeId := ⟨14⟩ } }] }

#guard validationHasDiagnosticAt expressionLifetimeScopeFixture
  "LIR-SEMANTIC-GENERIC-SCOPE" (some ⟨0⟩)

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
    BigStep.EvalFunction prepared (handle 2) {} #[] finalState outcome.value := by
  apply successfulRunHasDerivation (handle 2)
  native_decide

example : ∃ (finalState : RuntimeState) (outcome : LocatedOutcome),
    BigStep.EvalFunction prepared (handle 3) {} #[] finalState outcome.value := by
  apply successfulRunHasDerivation (handle 3)
  native_decide

example : ∃ (finalState : RuntimeState) (outcome : LocatedOutcome),
    BigStep.EvalFunction prepared (handle 5) {} #[] finalState outcome.value := by
  apply successfulRunHasDerivation (handle 5)
  native_decide

end LeanerIR.Tests.Generics
