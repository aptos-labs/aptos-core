-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Denote.TableFootprint

namespace LeanerIR.Tests.TableFootprint

open Proofs.Denote SnapshotValue

private def innerOwner : StructHandle := ⟨⟨0⟩, 0⟩
private def outerOwner : StructHandle := ⟨⟨0⟩, 1⟩
private def innerType : NTy :=
  .struct innerOwner (.cons .bool (.cons .bool .nil)) (.cons .address .nil)
private def outerType : NTy :=
  .struct outerOwner (.cons .bool (.cons innerType .nil)) (.cons .address .nil)

variable {unit : Validation.ValidatedUnit}

-- An absent slot is still observed. Its later allocation must not be hidden
-- by treating the original footprint as empty.
example (registered : tableOwner unit innerOwner = true) :
    footprint (unit := unit) (fun _ _ => none) innerType ("missing", ()) =
      [(TableMemory.resource innerType .bool .bool, .address "missing")] := by
  simp [footprint, innerType, registered, TableMemory.handleFields, TableMemory.read,
    RuntimeValue.field, RuntimeValue.asString]

-- A Table containing another Table reads both typed slots, even though its
-- physical owner contains only an address field.
example (memory : Memory unit)
    (innerRegistered : tableOwner unit innerOwner = true)
    (outerRegistered : tableOwner unit outerOwner = true) :
    let inner := TableMemory.write memory innerType .bool .bool "inner" #[(true, false, ())]
    let before := TableMemory.write inner outerType .bool innerType "outer"
      #[(true, ("inner", ()), ())]
    footprint before outerType ("outer", ()) =
      [(TableMemory.resource outerType .bool innerType, .address "outer"),
       (TableMemory.resource innerType .bool .bool, .address "inner")] := by
  simp only [innerOwner, outerOwner] at innerRegistered outerRegistered
  simp [footprint, innerType, outerType, innerRegistered, outerRegistered,
    TableMemory.handleFields, TableMemory.read, TableMemory.write, TableMemory.resource,
    Memory.set, RuntimeValue.field, RuntimeValue.asString,
    innerOwner, outerOwner]

-- Preserving just the outer slot does not preserve its nested observation.
example (memory : Memory unit)
    (innerRegistered : tableOwner unit innerOwner = true)
    (outerRegistered : tableOwner unit outerOwner = true) :
    let inner := TableMemory.write memory innerType .bool .bool "inner" #[(true, false, ())]
    let before := TableMemory.write inner outerType .bool innerType "outer"
      #[(true, ("inner", ()), ())]
    let after := TableMemory.write before innerType .bool .bool "inner" #[(true, true, ())]
    before (TableMemory.resource outerType .bool innerType) (.address "outer") =
        after (TableMemory.resource outerType .bool innerType) (.address "outer") ∧
      ¬AgreesOn before after (footprint before outerType ("outer", ())) := by
  simp only [innerOwner, outerOwner] at innerRegistered outerRegistered
  dsimp only
  constructor
  · simp [TableMemory.write, TableMemory.resource, innerType, outerType, Memory.set,
      innerOwner, outerOwner]
  · intro agreement
    have unchanged := agreement (TableMemory.resource innerType .bool .bool)
      (.address "inner") (by
        simp [footprint, innerType, outerType, innerRegistered, outerRegistered,
          TableMemory.handleFields, TableMemory.read, TableMemory.write, TableMemory.resource,
          Memory.set, RuntimeValue.field, RuntimeValue.asString, innerOwner, outerOwner])
    simp [TableMemory.write, TableMemory.resource, innerType, outerType, Memory.set,
      innerOwner, outerOwner] at unchanged
    have different := congrArg Array.toList unchanged
    simp only [List.cons.injEq, and_true] at different
    cases congrArg (fun entry => entry.2.1) different

-- A write to another handle preserves the entire selected observation.
example (memory : Memory unit) (registered : tableOwner unit innerOwner = true)
    (handle other : String) (distinct : handle ≠ other)
    (replacement : TableMemory.Contents unit .bool .bool) :
    observe memory innerType (handle, ()) =
      observe (TableMemory.write memory innerType .bool .bool other replacement)
        innerType (handle, ()) := by
  apply observe_eq_of_frame memory _ innerType (handle, ())
    [(TableMemory.resource innerType .bool .bool, .address other)]
    (PreservesOutside.set memory _ _ _)
  intro slot member
  simp [footprint, innerType, registered, TableMemory.handleFields, TableMemory.read,
    RuntimeValue.field, RuntimeValue.asString] at member
  subst slot
  simpa [innerType] using distinct

-- Shape alone is not registration. An ordinary address-bearing structure
-- does not grant access to a Table slot.
example (memory : Memory unit) (ordinary : tableOwner unit innerOwner = false) :
    footprint memory innerType ("lookalike", ()) = [] := by
  simp [footprint, footprintRow, innerType, ordinary]

-- Contents writes do not grant permission to rewrite the allocator's history.
example (before after : Memory unit) (handle : String)
    (frame : PreservesOutside before after
      [(TableMemory.resource innerType .bool .bool, .address handle)]) :
    before TableMemory.allocationResource .unit = after TableMemory.allocationResource .unit := by
  apply frame
  simp [TableMemory.resource, TableMemory.allocationResource]

-- Equal physical handle strings do not alias distinct phantom instances.
example (memory : Memory unit) (registered : tableOwner unit innerOwner = true) :
    footprint memory innerType ("same", ()) ≠
      footprint memory
        (.struct innerOwner (.cons .bool (.cons .address .nil)) (.cons .address .nil))
        ("same", ()) := by
  simp [footprint, innerType, registered, TableMemory.handleFields, TableMemory.read,
    TableMemory.resource, RuntimeValue.field, RuntimeValue.asString]

-- Encoded contract inputs are checked before the footprint is collected.
example (memory : Memory unit) :
    footprintRuntime? (Skolems.runtime unit) memory innerType .unit = none := rfl

end LeanerIR.Tests.TableFootprint
