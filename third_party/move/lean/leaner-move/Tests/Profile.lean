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

private def arithmeticError : ProfileValue :=
  { profile := .move, tag := "runtime.arithmetic_error" }

#guard (schema.checkOperation arithmeticError).isEmpty
#guard !(schema.checkOperation { arithmeticError with payload := "unexpected" }).isEmpty
#guard semantics.classify .throw_ arithmeticError == some .executable
#guard (semantics.classify .operation arithmeticError).isNone
#guard semantics.rollbackThrow LeanerIR.moveArithmeticError

private def vectorError : ProfileValue :=
  { profile := .move, tag := "runtime.vector_error" }

#guard (schema.checkOperation vectorError).isEmpty
#guard !(schema.checkOperation { vectorError with payload := "unexpected" }).isEmpty
#guard semantics.classify .throw_ vectorError == some .executable
#guard (semantics.classify .operation vectorError).isNone
#guard semantics.rollbackThrow (.profile vectorError)
#guard semantics.rollbackThrow .abort
#guard !semantics.rollbackThrow .panic
#guard !semantics.rollbackThrow (.profile { vectorError with tag := "runtime.unknown" })

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

-- A closure with `store` names a public or `@[persistent]` function.
leaner module 0x42::stored_closure_targets where
  public fun visible(x : u64) -> u64 := x
  @[persistent]
  fun persistent(x : u64) -> u64 := x
  fun stored() -> Vector<Fn(u64) -> u64 has Copy, Drop, Store> :=
    vector<Fn(u64) -> u64 has Copy, Drop, Store>[
      function[Fn(u64) -> u64 has Copy, Drop, Store](visible),
      function[Fn(u64) -> u64 has Copy, Drop, Store](persistent)]

/--
error: LIR-CLOSURE-STORE: a closure with `store` must target a function its profile lets a stored closure name
-/
#guard_msgs in
leaner module 0x42::stored_private_closure where
  fun hidden(x : u64) -> u64 := x
  fun stored() -> Fn(u64) -> u64 has Copy, Drop, Store :=
    function[Fn(u64) -> u64 has Copy, Drop, Store](hidden)

-- A dependency interface cannot silently erase a field's write frame and
-- let its caller assume the default empty frame. Owned declarations retain it.
private def framedDependency : LeanerMove.Frontend.Xast.Module := {
  (default : LeanerMove.Frontend.Xast.Module) with
  address := "0x42", name := "dependency"
  structs := [{ (default : LeanerMove.Frontend.Xast.Struct) with
    name := "Writer"
    fields := [{ name := "action", doc := "", ty := .function (.tuple []) (.tuple []) [] }]
    «spec» := .mk none [] [] none [.mk default "action" [] [] [] true false] none }] }

#guard match LeanerMove.Frontend.LIR.Encode.package
    { modules := [], dependencies := [framedDependency] } with
  | .error message => message.startsWith "modifies_of on fields of dependency"
  | .ok _ => false

#guard match LeanerMove.Frontend.LIR.Encode.package
    { modules := [framedDependency], dependencies := [] } with
  | .ok unit => unit.namespaces[0]!.structs[0]!.contract.parameterFrames.size == 1
  | .error _ => false

end LeanerIR.Move.Tests
