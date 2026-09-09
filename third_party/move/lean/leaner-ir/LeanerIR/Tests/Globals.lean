-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Interpreter
import LeanerIR.Validation.Check

namespace LeanerIR.Tests.Globals

open LeanerIR
open LeanerIR.Import
open LeanerIR.Validation

private def config : ProfileConfig := { profile := .move, name := "move-test" }
private def schema : ProfileSchema := { profile := .move, name := "move-test" }
private def semantics : SemanticProfile := {
  profile := .move
  name := "move-test"
  classify := fun _ _ => none
  rollbackThrow := fun kind => kind == .abort }

private def typeUse (typeId loc : Nat) : TypeUse := { typeId := ⟨typeId⟩, loc := ⟨loc⟩ }

private def fixture : RawUnit where
  tables := {
    files := #[{ name := "globals.move" }]
    locations := (Array.range 32).map fun index => {
      primary := some { file := ⟨0⟩, startByte := index, endByte := index + 1 } }
    origins := #[{ kind := .moveSource, location := ⟨0⟩ }]
    alignments := #[{ source := ⟨0⟩, trust := .checked, description := "global fixture" }]
    lifetimes := #[{ kind := .inference, loc := ⟨0⟩ }]
    types := #[
      .unit,
      .bool,
      .integer (.bits 64) false,
      .reference { profile := .move, kind := .mutable, referent := ⟨5⟩, lifetime := ⟨0⟩ },
      .address,
      .nominal ⟨2⟩ #[],
      .reference { profile := .move, kind := .mutable, referent := ⟨2⟩, lifetime := ⟨0⟩ }]
    namespaces := #[{ segments := #["test", "Globals"] }]
    names := #[
      { namespaceId := ⟨0⟩, name := "roundtrip" },
      { namespaceId := ⟨0⟩, name := "duplicate" },
      { namespaceId := ⟨0⟩, name := "Resource" },
      { namespaceId := ⟨0⟩, name := "value" },
      { namespaceId := ⟨0⟩, name := "focused" }] }
  profiles := #[config]
  namespaces := #[{
    loc := ⟨0⟩
    identity := ⟨0⟩
    profile := some .move
    expressions := #[
      { loc := ⟨0⟩, typeId := ⟨4⟩, kind := .value (.address "0x18") },
      { loc := ⟨1⟩, typeId := ⟨5⟩, kind := .operation
          (.call (.constructor { namespaceId := ⟨0⟩, name := ⟨2⟩ })) #[] #[⟨15⟩] },
      { loc := ⟨2⟩, typeId := ⟨0⟩, kind := .operation (.global .publish)
          #[.typeArg (typeUse 5 2)] #[⟨0⟩, ⟨1⟩] },
      { loc := ⟨3⟩, typeId := ⟨1⟩, kind := .operation (.global .contains)
          #[.typeArg (typeUse 5 3)] #[⟨0⟩] },
      { loc := ⟨4⟩, typeId := ⟨0⟩, kind := .operation .assert #[] #[⟨3⟩] },
      { loc := ⟨5⟩, typeId := ⟨3⟩, kind := .operation (.global (.borrow .mutable))
          #[.typeArg (typeUse 5 5)] #[⟨0⟩] },
      { loc := ⟨6⟩, typeId := ⟨2⟩, kind := .value (.integer 9) },
      { loc := ⟨7⟩, typeId := ⟨0⟩, kind := .operation (.write ⟨2⟩) #[] #[⟨6⟩] },
      { loc := ⟨8⟩, typeId := ⟨2⟩, kind := .operation (.read ⟨2⟩) #[] #[] },
      { loc := ⟨9⟩, typeId := ⟨2⟩, kind := .block #[⟨7⟩] (some ⟨8⟩) },
      { loc := ⟨10⟩, typeId := ⟨2⟩, kind := .letDecl ⟨0⟩ (some ⟨5⟩) ⟨9⟩ },
      { loc := ⟨11⟩, typeId := ⟨5⟩, kind := .operation (.global .take)
          #[.typeArg (typeUse 5 11)] #[⟨0⟩] },
      { loc := ⟨12⟩, typeId := ⟨5⟩, kind := .block #[⟨2⟩, ⟨4⟩, ⟨10⟩] (some ⟨11⟩) },
      { loc := ⟨13⟩, typeId := ⟨0⟩, kind := .operation (.global .publish)
          #[.typeArg (typeUse 5 13)] #[⟨0⟩, ⟨1⟩] },
      { loc := ⟨14⟩, typeId := ⟨0⟩, kind := .block #[⟨2⟩] (some ⟨13⟩) },
      { loc := ⟨15⟩, typeId := ⟨2⟩, kind := .value (.integer 7) },
      /- A field-focused reborrow: the whole resource is borrowed into a
      holder, the field is reborrowed as a place through it, and the write
      lands through that second loan.  What the holder exports when the
      frame dies is the value the focused hole received. -/
      { loc := ⟨19⟩, typeId := ⟨0⟩, kind := .operation (.global .publish)
          #[.typeArg (typeUse 5 19)] #[⟨0⟩, ⟨1⟩] },
      { loc := ⟨20⟩, typeId := ⟨3⟩, kind := .operation (.global (.borrow .mutable))
          #[.typeArg (typeUse 5 20)] #[⟨0⟩] },
      { loc := ⟨21⟩, typeId := ⟨6⟩, kind := .operation (.borrow .mutable ⟨2⟩) #[] #[] },
      { loc := ⟨22⟩, typeId := ⟨2⟩, kind := .value (.integer 9) },
      { loc := ⟨23⟩, typeId := ⟨0⟩, kind := .operation (.write ⟨4⟩) #[] #[⟨19⟩] },
      { loc := ⟨24⟩, typeId := ⟨5⟩, kind := .operation (.global .take)
          #[.typeArg (typeUse 5 24)] #[⟨0⟩] },
      { loc := ⟨25⟩, typeId := ⟨0⟩, kind := .block #[⟨20⟩] none },
      { loc := ⟨26⟩, typeId := ⟨0⟩, kind := .letDecl ⟨1⟩ (some ⟨18⟩) ⟨22⟩ },
      { loc := ⟨27⟩, typeId := ⟨0⟩, kind := .letDecl ⟨0⟩ (some ⟨17⟩) ⟨23⟩ },
      { loc := ⟨28⟩, typeId := ⟨5⟩, kind := .block #[⟨16⟩, ⟨24⟩] (some ⟨21⟩) }]
    patterns := #[
      { loc := ⟨15⟩, typeId := ⟨3⟩, kind := .variable ⟨0⟩ },
      { loc := ⟨21⟩, typeId := ⟨6⟩, kind := .variable ⟨1⟩ }]
    places := #[.localVar ⟨0⟩, .deref ⟨0⟩,
      .field ⟨1⟩ { namespaceId := ⟨0⟩, name := ⟨2⟩ } ⟨3⟩,
      .localVar ⟨1⟩, .deref ⟨3⟩]
    structs := #[{
      loc := ⟨18⟩
      name := ⟨2⟩
      fields := #[{ loc := ⟨18⟩, name := ⟨3⟩, type := typeUse 2 18 }]
      abilities := #[.key] }]
    functions := #[
      {
        loc := ⟨16⟩
        name := ⟨0⟩
        profile := .move
        signature := { results := #[typeUse 5 16] }
        body := .structured ⟨12⟩
        origin := ⟨0⟩
        alignment := ⟨0⟩
        locals := #[{
          id := ⟨0⟩, name := "reference", type := typeUse 3 16,
          mutable := false, loc := ⟨16⟩ }]
      },
      {
        loc := ⟨17⟩
        name := ⟨1⟩
        profile := .move
        signature := {}
        body := .structured ⟨14⟩
        origin := ⟨0⟩
        alignment := ⟨0⟩
      },
      {
        loc := ⟨29⟩
        name := ⟨4⟩
        profile := .move
        signature := { results := #[typeUse 5 29] }
        body := .structured ⟨25⟩
        origin := ⟨0⟩
        alignment := ⟨0⟩
        locals := #[
          { id := ⟨0⟩, name := "holder", type := typeUse 3 29,
            mutable := false, loc := ⟨29⟩ },
          { id := ⟨1⟩, name := "focus", type := typeUse 6 29,
            mutable := false, loc := ⟨29⟩ }]
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

/- Global ownership changes use the same live-loan conflict check as local
consumption. Keep the later read so this is not an already-ended loan. -/
private def takeWhileBorrowedFixture (shared : Bool) : RawUnit :=
  let ns := fixture.namespaces[0]!
  let kind : BorrowKind := if shared then .immutable else .mutable
  let types := fixture.tables.types.set! 3
    (.reference {
      profile := .move
      kind := if shared then .shared else .mutable
      referent := ⟨5⟩
      lifetime := ⟨0⟩ })
  let expressions := (ns.expressions.set! 5 {
    ns.expressions[5]! with
      kind := .operation (.global (.borrow kind)) #[.typeArg (typeUse 5 5)] #[⟨0⟩] }).set! 9 {
        ns.expressions[9]! with kind := .block #[⟨11⟩] (some ⟨8⟩) }
  { fixture with
    tables := { fixture.tables with types }
    namespaces := #[{ ns with expressions, functions := #[ns.functions[0]!] }] }

#guard preparationHasDiagnosticAt (takeWhileBorrowedFixture false)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨11⟩
#guard preparationHasDiagnosticAt (takeWhileBorrowedFixture true)
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨11⟩

private def publishWhileBorrowedFixture : RawUnit :=
  let raw := takeWhileBorrowedFixture true
  let ns := raw.namespaces[0]!
  { raw with namespaces := #[{ ns with expressions := ns.expressions.set! 9 {
      ns.expressions[9]! with kind := .block #[⟨13⟩] (some ⟨8⟩) } }] }

#guard preparationHasDiagnosticAt publishWhileBorrowedFixture
  "LIR-SEMANTIC-BORROW-CONFLICT" ⟨13⟩

private def badPublishValueFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with namespaces := #[{
      ns with expressions := ns.expressions.set! 1 {
        ns.expressions[1]! with typeId := ⟨1⟩, kind := .value (.bool true) } }] }

#guard validationHasDiagnosticAt badPublishValueFixture "LIR-SEMANTIC-TYPE" ⟨2⟩

private def badAddressFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with namespaces := #[{
      ns with expressions := ns.expressions.set! 0 {
        ns.expressions[0]! with typeId := ⟨1⟩, kind := .value (.bool true) } }] }

#guard validationHasDiagnosticAt badAddressFixture "LIR-SEMANTIC-TYPE" ⟨2⟩

private def badResourceAbilityFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with namespaces := #[{
      ns with structs := #[{ ns.structs[0]! with abilities := #[] }] }] }

#guard validationHasDiagnosticAt badResourceAbilityFixture "LIR-SEMANTIC-ABILITY" ⟨2⟩

private def badTakeResultFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with namespaces := #[{
      ns with expressions := ns.expressions.set! 11 {
        ns.expressions[11]! with typeId := ⟨0⟩ } }] }

#guard validationHasDiagnosticAt badTakeResultFixture "LIR-SEMANTIC-TYPE" ⟨11⟩

private def handle (id : Nat) : FunctionHandle := {
  namespaceId := ⟨0⟩, functionId := ⟨id⟩ }

#guard match executable? with
  | none => false
  | some executable => match LeanerIR.Interpreter.run executable 48 (handle 0) #[] with
    | .ok (state, outcome) =>
        outcome.value == .returned #[.nominal
          { namespaceId := ⟨0⟩, structId := 0 } none #[.integer 9]] &&
          state.globals.entries.isEmpty
    | .error _ => false

#guard match executable? with
  | none => false
  | some executable => match LeanerIR.Interpreter.run executable 32 (handle 1) #[] with
    | .ok (state, outcome) =>
        outcome.value == .threw .abort #[] && state.globals.entries.isEmpty
    | .error _ => false

-- The focused write reaches the published resource: the loan that held the
-- field settles into its holder before the holder writes back.
#guard match executable? with
  | none => false
  | some executable => match LeanerIR.Interpreter.run executable 48 (handle 2) #[] with
    | .ok (state, outcome) =>
        outcome.value == .returned #[.nominal
          { namespaceId := ⟨0⟩, structId := 0 } none #[.integer 9]] &&
          state.globals.entries.isEmpty && state.pending.isEmpty
    | .error _ => false

private def prepared : ExecutableUnit := executable?.get (by native_decide)

private theorem successfulRunHasDerivation (function : FunctionHandle) (fuel : Nat)
    (success : (LeanerIR.Interpreter.run prepared fuel function #[]).isOk) :
    ∃ (finalState : RuntimeState) (outcome : LocatedOutcome),
      BigStep.EvalFunction prepared function #[] {} #[] finalState outcome.value := by
  generalize result_eq : LeanerIR.Interpreter.run prepared fuel function #[] = result
  cases result with
  | error error => simp [result_eq, Except.isOk, Except.toBool] at success
  | ok result =>
      exact ⟨result.1, result.2,
        LeanerIR.Proofs.Interpreter.run_sound prepared fuel function #[] {}
          result.1 result.2 result_eq⟩

example : ∃ (finalState : RuntimeState) (outcome : LocatedOutcome),
    BigStep.EvalFunction prepared (handle 0) #[] {} #[] finalState outcome.value := by
  apply successfulRunHasDerivation (handle 0) 48
  native_decide

example : ∃ (finalState : RuntimeState) (outcome : LocatedOutcome),
    BigStep.EvalFunction prepared (handle 1) #[] {} #[] finalState outcome.value := by
  apply successfulRunHasDerivation (handle 1) 32
  native_decide

end LeanerIR.Tests.Globals
