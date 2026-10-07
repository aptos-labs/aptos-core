-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/-! Logical Table types may be named in both caller and callee namespaces.
They select one physical contents slot, independent of the spelling used by
the caller. Shared-reference carrier erasure must not add another slot. -/

leaner module 0x42::table_storage_owner where
  @[intrinsic_map]
  struct Table {K} {V} has Store where
    handle : Address

  public fun inspect(t : &Table<u64, u64>) -> Unit := ()
  public fun inspect_bool(t : &Table<Bool, u64>) -> Unit := ()

leaner module 0x42::table_storage_caller where
  use 0x42::table_storage_owner

  public fun inspect(t : &table_storage_owner::Table<u64, u64>) -> Unit :=
    table_storage_owner::inspect(t)

open LeanerIR LeanerIR.Proofs.Denote in
run_cmd do
  let some unit := LeanerLang.registeredUnit? (← Lean.getEnv) `«0x42».table_storage_caller
    | throwError "missing Table storage caller"
  let mut checked : Nat := 0
  let mut callerAlias := false
  let mut sharedReference := false
  for n in [:unit.namespaces.size] do
    let ns := unit.namespaces[n]!
    for i in [:ns.tables.types.size] do
      let source := TableMemory.resourceOf unit ⟨n⟩ ⟨i⟩
      if let .reference reference := ns.tables.types[i]! then
        if reference.kind == .shared &&
            (TableMemory.resourceOf unit ⟨n⟩ reference.referent).isSome then
          sharedReference := true
          unless source.isNone do throwError "a shared reference names a Table resource"
      let some contents := source | continue
      let resolved := TableMemory.slotOf? unit ⟨n⟩ ⟨i⟩ "first-handle"
      unless contents.type.paramFree && contents.arguments.paramFree do
        unless resolved.isNone do throwError "an open Table template names a runtime slot"
        continue
      let some slot := resolved | throwError "a closed Table type has no runtime slot"
      checked := checked + 1
      unless TableMemory.runtimeResourceOf unit slot.namespaceId slot.typeId == some contents do
        throwError "Table slot resolution changed its logical contents resource"
      unless slot.key == .address "first-handle" do throwError "Table slot resolution changed its handle"
      unless (TableMemory.slotOf? unit ⟨n⟩ ⟨i⟩ "second-handle").isSome &&
          TableMemory.slotOf? unit ⟨n⟩ ⟨i⟩ "second-handle" != resolved do
        throwError "different allocation handles resolved to the same slot"
      let mut physicalNames : Nat := 0
      for otherNamespace in [:unit.namespaces.size] do
        let other := unit.namespaces[otherNamespace]!
        for otherType in [:other.tables.types.size] do
          if TableMemory.runtimeResourceOf unit ⟨otherNamespace⟩ ⟨otherType⟩ == some contents then
            physicalNames := physicalNames + 1
          if TableMemory.resourceOf unit ⟨otherNamespace⟩ ⟨otherType⟩ == some contents then
            unless TableMemory.slotOf? unit ⟨otherNamespace⟩ ⟨otherType⟩ "first-handle" == resolved do
              throwError "caller and callee resolved different Table slots"
            if otherNamespace != n then callerAlias := true
      unless physicalNames == 1 do throwError "a logical Table resource has {physicalNames} physical names"
  unless checked > 0 && callerAlias && sharedReference do
    throwError "the Table storage-key regression did not exercise its required cases"
