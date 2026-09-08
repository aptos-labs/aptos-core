-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerMove

namespace LeanerIR.Move.Tests

open LeanerIR
open LeanerIR.Import
open LeanerIR.Validation

private def tables (kind : ExprKind) : Tables × Array Expr :=
  let files := #[{ name := "profile.move" }]
  let locations := #[{ primary := some { file := ⟨0⟩, startByte := 0, endByte := 1 } }]
  let origins := #[{ kind := .moveSource, location := ⟨0⟩ }]
  let alignments := #[{ source := ⟨0⟩, trust := .checked, description := "Move fixture" }]
  let types := #[.address]
  let namespaces := #[{ segments := #["0x1", "Profile"] }]
  let names := #[{ namespaceId := ⟨0⟩, name := "f" }]
  let table : Tables := { files, locations, origins, alignments, types, namespaces, names }
  let expressions : Array Expr := #[{
    loc := ⟨0⟩
    typeId := ⟨0⟩
    kind }]
  (table, expressions)

private def unit (kind : ExprKind) : RawUnit :=
  let (table, expressions) := tables kind
  { tables := table
    profiles := #[config]
    namespaces := #[{
      loc := ⟨0⟩
      identity := ⟨0⟩
      profile := some .move
      expressions
      functions := #[{
        loc := ⟨0⟩
        name := ⟨0⟩
        profile := .move
        signature := { results := #[{ typeId := ⟨0⟩, loc := ⟨0⟩ }] }
        body := .structured ⟨0⟩
        origin := ⟨0⟩
        alignment := ⟨0⟩ }] }] }

#guard match validate (unit (.value (.address "0x1"))) with
  | .ok checked => checked.indexes.functionCounts == #[1]
  | .error _ => false

#guard semanticInventoryComplete

private def coreExecutableUnit : RawUnit :=
  let base := unit (.value (.address "0x1"))
  let ns := base.namespaces[0]!
  { base with
    tables := { base.tables with types := #[.bool] }
    namespaces := #[{
      ns with
      expressions := #[{ loc := ⟨0⟩, typeId := ⟨0⟩, kind := .value (.bool true) }]
      functions := #[{ ns.functions[0]! with
        signature := { results := #[{ typeId := ⟨0⟩, loc := ⟨0⟩ }] }
        body := .structured ⟨0⟩ }] }] }

#guard match validate coreExecutableUnit with
  | .ok checked => (prepareExecution #[semantics] checked).isOk
  | .error _ => false

#guard match validate coreExecutableUnit with
  | .ok checked => (prepareVerification #[semantics] checked).isOk
  | .error _ => false

#guard match validate (unit (.operation (.profile {
    profile := .move, tag := "definitelyNotMove" }) #[] #[])) with
  | .error diagnostics => diagnostics.any (·.code == "LIR-MOVE-TAG")
  | .ok _ => false

#guard (schema.checkType { profile := .move, tag := "vector" }).any (·.code == "LIR-MOVE-TAG")

#guard (schema.checkOperation { profile := .move, tag := "add" }).any (·.code == "LIR-MOVE-TAG")

#guard match validate { unit (.value (.address "0x1")) with
    profiles := #[{ config with version := 2 }] } with
  | .error diagnostics => diagnostics.any (·.code == "LIR-PROFILE-VERSION")
  | .ok _ => false

end LeanerIR.Move.Tests
