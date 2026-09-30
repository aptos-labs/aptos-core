-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Validation.Check

/-!
# Static typing at preparation

Preparation runs the static checker (`Validation/StaticTyping.lean`) and a
prepared unit carries its acceptance. A unit assembled without validation
is checked all the same: preparation is the trust boundary.
-/

namespace LeanerIR.Tests.StaticTyping

open LeanerIR
open LeanerIR.Import
open LeanerIR.Validation

private def config : ProfileConfig := { profile := .rust, name := "rust-test" }
private def schema : ProfileSchema := { profile := .rust, name := "rust-test" }
private def semantics : SemanticProfile := {
  profile := .rust, name := "rust-test", classify := fun _ _ => none }

private def typeUse (typeId loc : Nat) : TypeUse := { typeId := ⟨typeId⟩, loc := ⟨loc⟩ }

/-- `answer(flag : bool) -> bool` returns its parameter; `forever() -> bool`
runs a loop only `return` leaves. -/
private def fixture : RawUnit where
  tables := {
    files := #[{ name := "static.lir" }]
    locations := (Array.range 8).map fun index => {
      primary := some { file := ⟨0⟩, startByte := index, endByte := index + 1 } }
    origins := #[{ kind := .leanerSource, location := ⟨0⟩ }]
    alignments := #[{
      source := ⟨0⟩, trust := .checked, description := "static typing fixture" }]
    types := #[.unit, .bool, .never]
    namespaces := #[{ segments := #["test", "StaticTyping"] }]
    names := #[
      { namespaceId := ⟨0⟩, name := "answer" },
      { namespaceId := ⟨0⟩, name := "forever" }] }
  profiles := #[config]
  namespaces := #[{
    loc := ⟨0⟩
    identity := ⟨0⟩
    profile := some .rust
    expressions := #[
      { loc := ⟨0⟩, typeId := ⟨1⟩, kind := .localVar ⟨0⟩ },
      { loc := ⟨1⟩, typeId := ⟨1⟩, kind := .value (.bool true) },
      { loc := ⟨2⟩, typeId := ⟨2⟩, kind := .return_ #[⟨1⟩] },
      { loc := ⟨3⟩, typeId := ⟨0⟩, kind := .loop none ⟨2⟩ }]
    functions := #[
      { loc := ⟨4⟩
        name := ⟨0⟩
        profile := .rust
        signature := {
          parameters := #[{ name := "flag", typeUse := typeUse 1 4 }]
          results := #[typeUse 1 4] }
        locals := #[{ id := ⟨0⟩, name := "flag", type := typeUse 1 4, loc := ⟨4⟩ }]
        body := .structured ⟨0⟩
        origin := ⟨0⟩
        alignment := ⟨0⟩ },
      { loc := ⟨5⟩
        name := ⟨1⟩
        profile := .rust
        signature := { results := #[typeUse 1 5] }
        body := .structured ⟨3⟩
        origin := ⟨0⟩
        alignment := ⟨0⟩ }] }]

private def validated? : Option ValidatedUnit := (validate #[schema] fixture).toOption

/-- The validated unit with one namespace replaced, assembled without
validation. -/
private def assembled (unit : ValidatedUnit) (ns : ValidatedNamespace) : ValidatedUnit :=
  Internal.mkValidatedUnit unit.tables unit.profiles #[ns] unit.dependencies unit.evidence
    unit.indexes unit.structurizationWitnesses unit.resolution
    unit.initializationCertificates unit.borrowCertificates

private def rejectedStatically (unit : ValidatedUnit) : Bool :=
  match prepareExecution #[semantics] unit with
  | .error diagnostics => diagnostics.any (·.code == "LIR-STATIC-TYPE")
  | .ok _ => false

-- A typed unit prepares, the loop that only `return` leaves included.
#guard match validated? with
  | some unit => (prepareExecution #[semantics] unit).isOk &&
      StaticTyping.checkUnit unit none
  | none => false

-- A parameter read claiming another type than its local's is refused.
#guard match validated? with
  | some unit =>
      let ns := unit.namespaces[0]!
      rejectedStatically (assembled unit { ns with
        expressions := ns.expressions.set! 0 { ns.expressions[0]! with typeId := ⟨0⟩ } })
  | none => false

-- A body whose value is not the declared result is refused.
#guard match validated? with
  | some unit =>
      let ns := unit.namespaces[0]!
      rejectedStatically (assembled unit { ns with
        expressions := ns.expressions.set! 0 { ns.expressions[0]! with
          kind := .value .unit, typeId := ⟨0⟩ } })
  | none => false

-- A loop a `break` leaves produces a value, so its type must be the body's.
#guard match validated? with
  | some unit =>
      let ns := unit.namespaces[0]!
      rejectedStatically (assembled unit { ns with
        expressions := ns.expressions.set! 2 { ns.expressions[2]! with
          kind := .break_ 0 none } })
  | none => false

-- A namespace away from its index is refused.
#guard match validated? with
  | some unit =>
      let ns := unit.namespaces[0]!
      rejectedStatically (assembled unit { ns with identity := ⟨1⟩ })
  | none => false

/-! A generic frame's global key reads its instantiation of `Box<T>`: the
caller `main` runs `has_box<u64>`, so `Box<u64>` must be interned. -/

private def moveConfig : ProfileConfig := { profile := .move, name := "move-test" }
private def moveSchema : ProfileSchema := { profile := .move, name := "move-test" }

private def genericGlobalFixture (interned : Bool) : RawUnit where
  tables := {
    files := #[{ name := "instantiation.move" }]
    locations := (Array.range 8).map fun index => {
      primary := some { file := ⟨0⟩, startByte := index, endByte := index + 1 } }
    origins := #[{ kind := .moveSource, location := ⟨0⟩ }]
    alignments := #[{
      source := ⟨0⟩, trust := .checked, description := "instantiation fixture" }]
    types := #[
      .unit, .bool, .address, .typeParameter 0,
      .nominal ⟨2⟩ #[.typeArg (typeUse 3 0)],
      .integer (.bits 64) false] ++
      (if interned then #[.nominal ⟨2⟩ #[.typeArg (typeUse 5 0)]] else #[])
    namespaces := #[{ segments := #["test", "Instantiation"] }]
    names := #[
      { namespaceId := ⟨0⟩, name := "has_box" },
      { namespaceId := ⟨0⟩, name := "main" },
      { namespaceId := ⟨0⟩, name := "Box" },
      { namespaceId := ⟨0⟩, name := "value" }] }
  profiles := #[moveConfig]
  namespaces := #[{
    loc := ⟨0⟩
    identity := ⟨0⟩
    profile := some .move
    expressions := #[
      { loc := ⟨0⟩, typeId := ⟨2⟩, kind := .localVar ⟨0⟩ },
      { loc := ⟨1⟩, typeId := ⟨1⟩, kind := .operation (.global .contains)
          #[.typeArg (typeUse 4 1)] #[⟨0⟩] },
      { loc := ⟨2⟩, typeId := ⟨2⟩, kind := .localVar ⟨0⟩ },
      { loc := ⟨3⟩, typeId := ⟨1⟩, kind := .operation
          (.call (.function { namespaceId := ⟨0⟩, name := ⟨0⟩ })) #[.typeArg (typeUse 5 3)]
          #[⟨2⟩] }]
    structs := #[{
      loc := ⟨4⟩
      name := ⟨2⟩
      generics := #[{ name := "T", kind := .typeArg, abilities := #[.store], loc := ⟨4⟩ }]
      fields := #[{ loc := ⟨4⟩, name := ⟨3⟩, type := typeUse 3 4 }]
      abilities := #[.key] }]
    functions := #[
      { loc := ⟨5⟩
        name := ⟨0⟩
        profile := .move
        signature := {
          generics := #[{ name := "T", kind := .typeArg, abilities := #[.store], loc := ⟨5⟩ }]
          parameters := #[{ name := "address", typeUse := typeUse 2 5 }]
          results := #[typeUse 1 5] }
        locals := #[{ id := ⟨0⟩, name := "address", type := typeUse 2 5, loc := ⟨5⟩ }]
        body := .structured ⟨1⟩
        origin := ⟨0⟩
        alignment := ⟨0⟩ },
      { loc := ⟨6⟩
        name := ⟨1⟩
        profile := .move
        signature := {
          parameters := #[{ name := "address", typeUse := typeUse 2 6 }]
          results := #[typeUse 1 6] }
        locals := #[{ id := ⟨0⟩, name := "address", type := typeUse 2 6, loc := ⟨6⟩ }]
        body := .structured ⟨3⟩
        origin := ⟨0⟩
        alignment := ⟨0⟩ }] }]

-- With `Box<u64>` interned, the frame of `has_box<u64>` is faithful.
#guard match validate #[moveSchema] (genericGlobalFixture true) with
  | .ok unit => StaticTyping.checkUnit unit none
  | .error _ => false

-- Without it, the frame would key the global by the symbolic `Box<T>`.
#guard match validate #[moveSchema] (genericGlobalFixture false) with
  | .ok unit => !StaticTyping.checkUnit unit none
  | .error _ => false

end LeanerIR.Tests.StaticTyping
