-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Types
import VerexIR.Proofs.Maps.Table

/-!
# Table allocation and identity

A table is a value of its owner's type: its handle among the declared fields
and its contents in a hidden last field (`doc/designs/intrinsic-maps.md`, "Owned
Table values"). The handles allocated so far are a slot of the memory
(`allocationResource`), as the runtime keeps them (`RuntimeState.tableHandles`);
the model operations keep a table's identity.
-/

namespace VerexIR.Proofs.Denote.TableMemory

open VerexIR.Validation

variable {unit : ValidatedUnit} [FunctionCarriers unit]

/-- The fixed logical slot of allocation history. Its address elements and
empty scope are independent of a callee's type parameters, and the collection
carrier imposes no Move vector length bound. -/
def allocationResource : ResourceType := ⟨.address, .nil, .collection⟩

def encodeAllocated (handles : Array String) : allocationResource.carrier unit := handles

theorem encodeAllocated_injective (left right : Array String)
    (equal : encodeAllocated (unit := unit) left = encodeAllocated right) : left = right := by
  exact equal

end VerexIR.Proofs.Denote.TableMemory

namespace VerexIR.Proofs.Denote

/-! ## Identity under the model operations

A table owner's identity is its handle, which the model operations keep
(`Maps.field_zero_update`), so a specification's equality holds between a
table and its functional updates. -/

theorem NTy.identityRuntime_build (source : StructHandle) (arguments rest : NRow)
    (hidden : rest.holdsContents = true) (layout : Maps.Layout) (h : layout.handle = true)
    (handle : RuntimeValue) (slots : List RuntimeValue) (entries : Maps.Entries) :
    NTy.identityRuntime (.struct source arguments (.cons .address rest))
        (layout.build handle entries slots) =
      .nominal layout.owner layout.variant #[handle] := by
  have field := Maps.Layout.field_zero_build layout handle slots entries h
  unfold Maps.Layout.build at field ⊢
  rw [NTy.identityRuntime_handle _ _ _ hidden, field]

@[simp] theorem NTy.identityRuntime_update (source : StructHandle)
    (arguments rest : NRow) (hidden : rest.holdsContents = true) (layout : Maps.Layout)
    (h : layout.handle = true) (discipline : Maps.Discipline) (map key value : RuntimeValue) :
    NTy.identityRuntime (.struct source arguments (.cons .address rest))
        (Maps.update layout discipline map key value) =
      .nominal layout.owner layout.variant #[map.field 0] := by
  unfold Maps.update
  rw [NTy.identityRuntime_build _ _ _ hidden _ h]
  simp [Maps.Layout.handleOf, h]

@[simp] theorem NTy.identityRuntime_remove (source : StructHandle)
    (arguments rest : NRow) (hidden : rest.holdsContents = true) (layout : Maps.Layout)
    (h : layout.handle = true) (discipline : Maps.Discipline) (map key : RuntimeValue) :
    NTy.identityRuntime (.struct source arguments (.cons .address rest))
        (Maps.remove layout discipline map key) =
      .nominal layout.owner layout.variant #[map.field 0] := by
  unfold Maps.remove
  rw [NTy.identityRuntime_build _ _ _ hidden _ h]
  simp [Maps.Layout.handleOf, h]

@[simp] theorem NTy.identityRuntime_updateAll (source : StructHandle)
    (arguments rest : NRow) (hidden : rest.holdsContents = true) (layout : Maps.Layout)
    (h : layout.handle = true) (discipline : Maps.Discipline) (map keys values : RuntimeValue) :
    NTy.identityRuntime (.struct source arguments (.cons .address rest))
        (Maps.updateAll layout discipline map keys values) =
      .nominal layout.owner layout.variant #[map.field 0] := by
  unfold Maps.updateAll
  rw [NTy.identityRuntime_build _ _ _ hidden _ h]
  simp [Maps.Layout.handleOf, h]

@[simp] theorem NTy.identityRuntime_takeEntries (source : StructHandle)
    (arguments rest : NRow) (hidden : rest.holdsContents = true) (layout : Maps.Layout)
    (h : layout.handle = true) (map : RuntimeValue) (count : Int) :
    NTy.identityRuntime (.struct source arguments (.cons .address rest))
        (Maps.takeEntries layout map count) =
      .nominal layout.owner layout.variant #[map.field 0] := by
  unfold Maps.takeEntries
  rw [NTy.identityRuntime_build _ _ _ hidden _ h]
  simp [Maps.Layout.handleOf, h]

@[simp] theorem NTy.identityRuntime_replaceKey (source : StructHandle)
    (arguments rest : NRow) (hidden : rest.holdsContents = true) (layout : Maps.Layout)
    (h : layout.handle = true) (map old new : RuntimeValue) :
    NTy.identityRuntime (.struct source arguments (.cons .address rest))
        (Maps.replaceKey layout map old new) =
      .nominal layout.owner layout.variant #[map.field 0] := by
  unfold Maps.replaceKey
  rw [NTy.identityRuntime_build _ _ _ hidden _ h]
  simp [Maps.Layout.handleOf, h]

end VerexIR.Proofs.Denote
