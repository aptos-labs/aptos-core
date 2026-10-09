-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Denote.SnapshotValue

namespace LeanerIR.Tests.SnapshotValue

open Proofs.Denote SnapshotValue

private def innerOwner : StructHandle := ⟨⟨0⟩, 0⟩
private def outerOwner : StructHandle := ⟨⟨0⟩, 1⟩
private def innerType : NTy :=
  .struct innerOwner (.cons .bool (.cons .bool .nil)) (.cons .address .nil)
private def outerType : NTy :=
  .struct outerOwner (.cons .bool (.cons innerType .nil)) (.cons .address .nil)
private def innerPhysical : RuntimeValue := .nominal innerOwner none #[.address "inner"]
private def outerPhysical : RuntimeValue := .nominal outerOwner none #[.address "outer"]
private def innerArguments : TypeArgs := ⟨.cons innerType .nil, by decide⟩

-- Two functional snapshots of one inner Table coexist as different values of
-- one outer Table. A single memory attached to the outer handle cannot encode
-- this specification value. Projecting either child preserves its observation.
private def oldInner : Value := .table innerPhysical (some [(.bool true, .scalar (.bool false))])
private def newInner : Value := .table innerPhysical (some [(.bool true, .scalar (.bool true))])
private def parent : Value := .table outerPhysical
  (some [(.bool false, oldInner), (.bool true, newInner)])

example : oldInner.set? (.bool true) (.scalar (.bool true)) = some newInner := by
  simp [oldInner, newInner, innerPhysical, Value.set?, Value.setEntries, Value.refreshLength]

example : (parent.lookup? (.bool false)).bind (·.lookup? (.bool true)) =
    some (.scalar (.bool false)) ∧
    (parent.lookup? (.bool true)).bind (·.lookup? (.bool true)) = some (.scalar (.bool true)) := by
  simp [parent, oldInner, newInner, Value.lookup?]

example : Value.SameIdentity oldInner newInner ∧ oldInner ≠ newInner := by
  constructor
  · exact Value.sameIdentity_table _ _ _
  · simp [oldInner, newInner]

example : ¬Value.SameIdentity oldInner
    (.table (.nominal innerOwner none #[.address "different-allocation"])
      (some [(.bool true, .scalar (.bool false))])) := by
  simp [Value.SameIdentity, Value.identity, oldInner, innerPhysical]

example : ((Value.aggregate .vector [oldInner, newInner]).field 0).lookup? (.bool true) =
    some (.scalar (.bool false)) ∧
    ((Value.aggregate .vector [oldInner, newInner]).field 1).lookup? (.bool true) =
      some (.scalar (.bool true)) := by
  simp [Value.field, Value.lookup?, oldInner, newInner]

-- Cached length follows functional insertion/removal without changing identity.
private def cachedEmpty : Value :=
  .table (.nominal innerOwner none #[.address "cached", .integer 0]) (some [])

example : ((cachedEmpty.set? (.bool true) (.scalar (.integer 7))).map (·.field 1)) =
    some (.scalar (.integer 1)) := by
  simp [cachedEmpty, Value.set?, Value.setEntries, Value.refreshLength, Value.field, RuntimeValue.field]

example : ((cachedEmpty.set? (.bool true) (.scalar (.integer 7))).bind
      (·.remove? (.bool true))) = some cachedEmpty := by
  simp [cachedEmpty, Value.set?, Value.remove?, Value.setEntries, Value.refreshLength]

-- Observation uses explicit memories, with no program point or ambient state.
-- The outer Table is unchanged; only its nested Table's storage differs.
example (unit : Validation.ValidatedUnit) (memory : Memory unit)
    (innerRegistered : tableOwner unit innerOwner = true)
    (outerRegistered : tableOwner unit outerOwner = true) :
    let innerBefore := TableMemory.write memory innerType .bool .bool "inner" #[(true, false, ())]
    let before := TableMemory.write innerBefore outerType .bool innerType "outer"
      #[(true, ("inner", ()), ())]
    let after := TableMemory.write before innerType .bool .bool "inner" #[(true, true, ())]
    let oldView := observe before outerType ("outer", ())
    let newView := observe after outerType ("outer", ())
    (oldView.lookup? (.bool true)).bind (·.lookup? (.bool true)) = some (.scalar (.bool false)) ∧
      (newView.lookup? (.bool true)).bind (·.lookup? (.bool true)) = some (.scalar (.bool true)) := by
  simp only [innerOwner, outerOwner] at innerRegistered outerRegistered
  simp [outerType, innerType, observe, outerRegistered, innerRegistered, TableMemory.handleFields,
    TableMemory.read, TableMemory.write, TableMemory.resource, Memory.set, Value.lookup?,
    RuntimeValue.field, RuntimeValue.asString, innerOwner, outerOwner]

-- A struct with the same physical layout is not a Table without a registry binding.
example (unit : Validation.ValidatedUnit) (memory : Memory unit)
    (ordinary : tableOwner unit innerOwner = false) :
    observe memory innerType ("inner", ()) =
      .aggregate (.nominal innerOwner none) [.scalar (.address "inner")] := by
  simp [innerType, observe, observeRow, ordinary]

-- Missing storage does not silently turn into an empty allocated Table.
example (unit : Validation.ValidatedUnit) (memory : Memory unit)
    (registered : tableOwner unit innerOwner = true)
    (missing : TableMemory.read memory innerType .bool .bool "inner" = none) :
    observe memory innerType ("inner", ()) = .table innerPhysical none := by
  dsimp [innerType] at missing
  simpa [innerType, innerPhysical, TableMemory.handleFields, RuntimeValue.field, RuntimeValue.asString,
    missing] using observe_table memory innerOwner .bool .bool (.cons .address .nil)
      ("inner", ()) registered rfl

-- Instantiating a generic parameter reveals the Table's contents rather than
-- leaving the parameter as an opaque handle-shaped scalar.
example (unit : Validation.ValidatedUnit) (memory : Memory unit)
    (registered : tableOwner unit innerOwner = true) :
    let frame := Skolems.instantiate innerArguments (Skolems.runtime unit)
    let state := TableMemory.write memory innerType .bool .bool "inner" #[(true, false, ())]
    observeInFrame frame state (.param 0) ("inner", ()) = oldInner := by
  change observe (TableMemory.write memory innerType .bool .bool "inner" #[(true, false, ())])
    innerType ("inner", ()) = oldInner
  simp [innerType, observe, registered, TableMemory.handleFields, TableMemory.read,
    TableMemory.write, TableMemory.resource, Memory.set, RuntimeValue.field,
    RuntimeValue.asString, oldInner, innerPhysical]

end LeanerIR.Tests.SnapshotValue
