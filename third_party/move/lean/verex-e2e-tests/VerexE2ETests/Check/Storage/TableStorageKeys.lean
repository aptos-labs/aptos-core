-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexE2ETests.CheckSupport

/-! A table's contents are held in native storage at the slot of its owner's
type and handle (`tableSlot?`). The owner type may be named in both caller
and callee namespaces; both resolve to one slot, in the declaring namespace.
A shared reference to a table denotes the owner's type and names no slot of
its own. -/

verex module 0x42::table_storage_owner where
  @[intrinsic_map]
  struct Table {K} {V} has Store where
    handle : Address

  public fun inspect(t : &Table<u64, u64>) -> Unit := ()
  public fun inspect_bool(t : &Table<Bool, u64>) -> Unit := ()

verex module 0x42::table_storage_caller where
  use 0x42::table_storage_owner

  public fun inspect(t : &table_storage_owner::Table<u64, u64>) -> Unit :=
    table_storage_owner::inspect(t)

open VerexIR VerexIR.Proofs.Denote in
run_cmd do
  let some unit := VerexSL.registeredUnit? (← Lean.getEnv) `«0x42».table_storage_caller
    | throwError "missing Table storage caller"
  let mut slots : Array (NTy × GlobalKey) := #[]
  let mut callerAlias := false
  let mut sharedReference := false
  for n in [:unit.namespaces.size] do
    let ns := unit.namespaces[n]!
    for i in [:ns.tables.types.size] do
      let some owner := ntyOf unit ⟨n⟩ ⟨i⟩ | continue
      let .struct source _ _ := owner | continue
      unless tableOwner unit source do continue
      unless owner.paramFree do
        unless (tableSlot? unit owner "first-handle").isNone do
          throwError "an open Table type names a runtime slot"
        continue
      let some slot := tableSlot? unit owner "first-handle"
        | throwError "a closed Table type has no runtime slot"
      unless slot.namespaceId == source.namespaceId do
        throwError "a Table's slot is not in its declaring namespace"
      unless slot.key == .address "first-handle" do throwError "a Table slot changed its handle"
      unless tableSlot? unit owner "second-handle" != some slot do
        throwError "different allocation handles resolved to the same slot"
      if n != source.namespaceId.index then callerAlias := true
      if let .reference reference := ns.tables.types[i]! then
        if reference.kind == .shared then sharedReference := true
      match slots.find? (·.1 == owner) with
      | some (_, known) =>
          unless known == slot do throwError "one Table type resolved to two slots"
      | none => slots := slots.push (owner, slot)
  unless !slots.isEmpty && callerAlias && sharedReference do
    throwError "the Table storage-key regression did not exercise its required cases"
