-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Types
import VerexIR.Proofs.Maps.Table

/-!
# Tables in native storage

A table is a value of its owner's type: its handle among the declared fields
and its contents in a hidden last field (`designs/intrinsic-maps.md`, "Owned
Table values"). Native storage holds each table's entries at the slot of its
owner's type and handle (`tableSlot?`), beside the physical values, and the
allocation history of handles at a fixed resource. This module relates the
tables a frame's values hold to that storage; `Resources.lean` relates a whole
memory to it.
-/

namespace VerexIR.Proofs.Denote.TableMemory

open VerexIR.Validation

variable {unit : ValidatedUnit}

/-- The fixed logical slot of allocation history. Its address elements and
empty scope are independent of a callee's type parameters, and the collection
carrier imposes no Move vector length bound. -/
def allocationResource : ResourceType := ⟨.address, .nil, .collection⟩

def encodeAllocated (handles : Array String) : allocationResource.carrier unit := handles

theorem encodeAllocated_injective (left right : Array String)
    (equal : encodeAllocated (unit := unit) left = encodeAllocated right) : left = right := by
  exact equal

/-- The tables of a frame's values held in native storage: each at the slot
of its owner's type, resolved at the frame, and its handle. -/
def TablesIn (unit : ValidatedUnit) [Θ : Skolems unit]
    (tables : List (NTy × String × RuntimeValue)) (storage : NativeTableStorage) : Prop :=
  TablesAt (fun owner handle => tableSlot? unit (Θ.resolve owner) handle)
    storage.contents.lookup tables

theorem TablesIn.append_left [Skolems unit] {left right : List (NTy × String × RuntimeValue)}
    {storage : NativeTableStorage} (held : TablesIn unit (left ++ right) storage) :
    TablesIn unit left storage :=
  TablesAt.append_left held

theorem TablesIn.append_right [Skolems unit] {left right : List (NTy × String × RuntimeValue)}
    {storage : NativeTableStorage} (held : TablesIn unit (left ++ right) storage) :
    TablesIn unit right storage :=
  TablesAt.append_right held

theorem TablesIn.append [Skolems unit] {left right : List (NTy × String × RuntimeValue)}
    {storage : NativeTableStorage} (heldLeft : TablesIn unit left storage)
    (heldRight : TablesIn unit right storage) : TablesIn unit (left ++ right) storage :=
  fun table member => (List.mem_append.mp member).elim (heldLeft table) (heldRight table)

theorem TablesIn.nil [Skolems unit] (storage : NativeTableStorage) : TablesIn unit [] storage :=
  fun _ member => nomatch member

/-- Tables read at an induced frame are the tables read at the caller's frame
with their owners at the type arguments. -/
theorem TablesIn.instantiate (θ : TypeArgs) (Θ : Skolems unit)
    (tables : List (NTy × String × RuntimeValue)) (storage : NativeTableStorage) :
    @TablesIn unit (Skolems.instantiate θ Θ) tables storage ↔
      @TablesIn unit Θ (tables.map (tableSubst θ)) storage := by
  simp only [TablesIn, TablesAt, List.forall_mem_map, tableSubst, Skolems.resolve_instantiate]

end VerexIR.Proofs.Denote.TableMemory

namespace VerexIR.Proofs.Denote

/-! ## Identity under the model operations

A table owner's identity is its handle, which the model operations keep
(`Maps.field_zero_update`), so a specification's equality holds between a
table and its functional updates. -/

theorem NTy.identityRuntime_build (source : StructHandle) (arguments rest : NRow)
    (hidden : rest.holdsContents = true) (layout : Maps.Layout) (h : layout.handle = true)
    (handle : RuntimeValue) (entries : Maps.Entries) :
    NTy.identityRuntime (.struct source arguments (.cons .address rest))
        (layout.build handle entries) =
      .nominal layout.owner layout.variant #[handle] := by
  have field := Maps.Layout.field_zero_build layout handle entries h
  unfold Maps.Layout.build at field ⊢
  rw [NTy.identityRuntime_handle _ _ _ hidden, field]

@[simp, vir_denote_norm] theorem NTy.identityRuntime_update (source : StructHandle)
    (arguments rest : NRow) (hidden : rest.holdsContents = true) (layout : Maps.Layout)
    (h : layout.handle = true) (discipline : Maps.Discipline) (map key value : RuntimeValue) :
    NTy.identityRuntime (.struct source arguments (.cons .address rest))
        (Maps.update layout discipline map key value) =
      .nominal layout.owner layout.variant #[map.field 0] := by
  unfold Maps.update
  rw [NTy.identityRuntime_build _ _ _ hidden _ h]
  simp [Maps.Layout.handleOf, h]

@[simp, vir_denote_norm] theorem NTy.identityRuntime_remove (source : StructHandle)
    (arguments rest : NRow) (hidden : rest.holdsContents = true) (layout : Maps.Layout)
    (h : layout.handle = true) (discipline : Maps.Discipline) (map key : RuntimeValue) :
    NTy.identityRuntime (.struct source arguments (.cons .address rest))
        (Maps.remove layout discipline map key) =
      .nominal layout.owner layout.variant #[map.field 0] := by
  unfold Maps.remove
  rw [NTy.identityRuntime_build _ _ _ hidden _ h]
  simp [Maps.Layout.handleOf, h]

@[simp, vir_denote_norm] theorem NTy.identityRuntime_updateAll (source : StructHandle)
    (arguments rest : NRow) (hidden : rest.holdsContents = true) (layout : Maps.Layout)
    (h : layout.handle = true) (discipline : Maps.Discipline) (map keys values : RuntimeValue) :
    NTy.identityRuntime (.struct source arguments (.cons .address rest))
        (Maps.updateAll layout discipline map keys values) =
      .nominal layout.owner layout.variant #[map.field 0] := by
  unfold Maps.updateAll
  rw [NTy.identityRuntime_build _ _ _ hidden _ h]
  simp [Maps.Layout.handleOf, h]

@[simp, vir_denote_norm] theorem NTy.identityRuntime_takeEntries (source : StructHandle)
    (arguments rest : NRow) (hidden : rest.holdsContents = true) (layout : Maps.Layout)
    (h : layout.handle = true) (map : RuntimeValue) (count : Int) :
    NTy.identityRuntime (.struct source arguments (.cons .address rest))
        (Maps.takeEntries layout map count) =
      .nominal layout.owner layout.variant #[map.field 0] := by
  unfold Maps.takeEntries
  rw [NTy.identityRuntime_build _ _ _ hidden _ h]
  simp [Maps.Layout.handleOf, h]

@[simp, vir_denote_norm] theorem NTy.identityRuntime_replaceKey (source : StructHandle)
    (arguments rest : NRow) (hidden : rest.holdsContents = true) (layout : Maps.Layout)
    (h : layout.handle = true) (map old new : RuntimeValue) :
    NTy.identityRuntime (.struct source arguments (.cons .address rest))
        (Maps.replaceKey layout map old new) =
      .nominal layout.owner layout.variant #[map.field 0] := by
  unfold Maps.replaceKey
  rw [NTy.identityRuntime_build _ _ _ hidden _ h]
  simp [Maps.Layout.handleOf, h]

end VerexIR.Proofs.Denote
