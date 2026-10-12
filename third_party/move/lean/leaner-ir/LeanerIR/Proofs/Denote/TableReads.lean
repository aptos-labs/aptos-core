-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Denote.SnapshotValue
import LeanerIR.Proofs.Denote.TableOperations

/-! Shared Table lookup observes the selected typed value at the same memory
as its owner, including nested Table contents. These laws connect the logical
snapshot roles to the typed lookup already related to runtime storage. -/

namespace LeanerIR.Proofs.Denote.SnapshotValue

variable {unit : Validation.ValidatedUnit}

theorem lookup_observe_table (memory : Memory unit) (source : StructHandle)
    (key element : NTy) (fields : NRow)
    (table : @HList (Carriers.runtime unit) fields)
    (query : @NTy.carrier (Carriers.runtime unit) key)
    (registered : tableOwner unit source = true) (layout : TableMemory.handleFields fields = true) :
    let owner := NTy.struct source (.cons key (.cons element .nil)) fields
    let handle := ((@NTy.encode (Carriers.runtime unit) owner table).field 0).asString
    (observe memory owner table).lookup? (@NTy.encode (Carriers.runtime unit) key query) =
      ((TableMemory.read memory owner key element handle).bind
        (fun contents => TableMemory.lookup? key element contents query)).map
          (observe memory element) := by
  dsimp only
  rw [observe_table memory source key element fields table registered layout]
  cases stored : TableMemory.read memory
      (.struct source (.cons key (.cons element .nil)) fields) key element
      ((@NTy.encode (Carriers.runtime unit)
        (.struct source (.cons key (.cons element .nil)) fields) table).field 0).asString with
  | none => simp only [stored, Option.map_none, Option.bind_none, Value.lookup?]
  | some contents =>
      simp only [stored, Option.map_some, Option.bind_some, Value.lookup?,
        List.find?_map, Function.comp_def]
      simp only [Array.find?_toList, Array.find?_eq_bind_findIdx?_getElem?]
      unfold TableMemory.lookup? TableMemory.index?
      cases indexed : contents.findIdx? (fun entry =>
          @NTy.encode (Carriers.runtime unit) key entry.1 ==
            @NTy.encode (Carriers.runtime unit) key query) with
      | none => rfl
      | some index =>
          simp only [Option.bind_eq_bind, Option.bind_some]
          cases contents[index]? <;> rfl

end LeanerIR.Proofs.Denote.SnapshotValue
