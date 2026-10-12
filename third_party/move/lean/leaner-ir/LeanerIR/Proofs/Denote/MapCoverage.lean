-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Maps
import LeanerIR.Proofs.Denote.Types
open LeanerIR LeanerIR.Proofs LeanerIR.Proofs.Denote
namespace LeanerIR.Proofs.Denote

private theorem length_le_of_subset {α : Type} [BEq α] [LawfulBEq α]
    {left right : List α} (distinct : left.Nodup) (subset : left ⊆ right) :
    left.length ≤ right.length := by
  induction left generalizing right with
  | nil => simp
  | cons head tail ih =>
    obtain ⟨absent, distinct⟩ := List.nodup_cons.mp distinct
    have present := subset (List.mem_cons_self ..)
    have contained : tail ⊆ right.erase head := by
      intro item mem
      exact (List.mem_erase_of_ne (a := item) (b := head) (by intro eq; subst item; exact absent mem)).mpr
        (subset (List.mem_cons_of_mem _ mem))
    have bound := ih distinct contained
    have length := List.length_erase_of_mem present
    have nonempty : 0 < right.length := List.length_pos_of_mem present
    simp only [List.length_cons]
    omega

private theorem mem_of_subset_of_length {α : Type} [BEq α] [LawfulBEq α]
    {left right : List α} (distinct : left.Nodup) (subset : left ⊆ right)
    (cardinality : left.length = right.length) {item : α} (present : item ∈ right) : item ∈ left := by
  classical
  apply Classical.byContradiction
  intro absent
  have contained : left ⊆ right.erase item := by
    intro x mem
    exact (List.mem_erase_of_ne (a := x) (b := item) (by intro eq; subst x; exact absent mem)).mpr (subset mem)
  have bound := length_le_of_subset distinct contained
  have length := List.length_erase_of_mem present
  have nonempty : 0 < right.length := List.length_pos_of_mem present
  omega

/-- A distinct vector covering a map of the same size contains every map key. -/
theorem map_key_mem_of_coverage (map : RuntimeValue) (keys : Array String)
    (covered : ∀ i : Int, 0 ≤ i → i < keys.size →
      Maps.hasKey map (.address ((Option.map Codec.address.encode keys[i.toNat]?).getD .unit).asString) = true)
    (cardinality : Maps.size map = keys.size)
    (distinct : ∀ i : Int, 0 ≤ i → i < keys.size → ∀ j : Int, 0 ≤ j → j < keys.size →
      ((Option.map Codec.address.encode keys[i.toNat]?).getD .unit).asString =
      ((Option.map Codec.address.encode keys[j.toNat]?).getD .unit).asString → i = j)
    (key : String) (present : Maps.hasKey map (.address key) = true) : key ∈ keys := by
  have simple (i : Nat) (within : i < keys.size) :
      ((Option.map Codec.address.encode keys[(i : Int).toNat]?).getD .unit).asString = keys[i] := by
    simp [Array.getElem?_eq_getElem within, Codec.address, RuntimeValue.asString]
  have nodup : (keys.toList.map RuntimeValue.address).Nodup := by
    apply List.pairwise_iff_getElem.mpr
    intro i j hi hj lt eq
    simp only [List.length_map, Array.length_toList] at hi hj
    have eq' : keys[i] = keys[j] := by simpa using eq
    have := distinct i (by omega) (by omega) j (by omega) (by omega)
      (by simpa only [simple i hi, simple j hj] using eq')
    omega
  have subset : keys.toList.map RuntimeValue.address ⊆ (Maps.entriesOf map).map Prod.fst := by
    intro item mem
    obtain ⟨value, mem, rfl⟩ := List.mem_map.mp mem
    have mem : value ∈ keys := by simpa using mem
    obtain ⟨i, hi, rfl⟩ := Array.getElem_of_mem mem
    have found := covered i (by omega) (by omega)
    rw [simple i hi] at found
    obtain ⟨value, mem⟩ := (Maps.containsB_iff _ _).mp found
    exact List.mem_map.mpr ⟨(_, value), mem, rfl⟩
  have mem : RuntimeValue.address key ∈ (Maps.entriesOf map).map Prod.fst := by
    obtain ⟨value, mem⟩ := (Maps.containsB_iff _ _).mp present
    exact List.mem_map.mpr ⟨(_, value), mem, rfl⟩
  have card : (keys.toList.map RuntimeValue.address).length = ((Maps.entriesOf map).map Prod.fst).length := by
    simp only [List.length_map, Array.length_toList]
    simp only [Maps.size] at cardinality
    omega
  have := mem_of_subset_of_length nodup subset card mem
  simpa using this

/-- A present key cannot be absent from a complete distinct enumeration. -/
theorem map_key_search_absurd (map : RuntimeValue) (keys : Array String)
    (covered : ∀ i : Int, 0 ≤ i → i < keys.size →
      Maps.hasKey map (.address ((Option.map Codec.address.encode keys[i.toNat]?).getD .unit).asString) = true)
    (cardinality : Maps.size map = keys.size)
    (distinct : ∀ i : Int, 0 ≤ i → i < keys.size → ∀ j : Int, 0 ≤ j → j < keys.size →
      ((Option.map Codec.address.encode keys[i.toNat]?).getD .unit).asString =
      ((Option.map Codec.address.encode keys[j.toNat]?).getD .unit).asString → i = j)
    (key : String) (present : Maps.hasKey map (.address key) = true)
    (missing : ∀ i : Int, 0 ≤ i → i < keys.size → ¬keys[i.toNat]? = some key) : False := by
  have mem := map_key_mem_of_coverage map keys covered cardinality distinct key present
  obtain ⟨i, hi, eq⟩ := Array.getElem_of_mem mem
  apply missing i (by omega) (by omega)
  simp [Array.getElem?_eq_getElem hi, eq]

/-- Removing a matching vector element and map key preserves coverage. The
intermediate update models a mutable borrow ending before the map removal. -/
theorem map_keys_after_remove (map : RuntimeValue) (keys : Array String)
    (covered : ∀ i : Int, 0 ≤ i → i < keys.size →
      Maps.hasKey map (.address ((Option.map Codec.address.encode keys[i.toNat]?).getD .unit).asString) = true)
    (distinct : ∀ i : Int, 0 ≤ i → i < keys.size → ∀ j : Int, 0 ≤ j → j < keys.size →
      ((Option.map Codec.address.encode keys[i.toNat]?).getD .unit).asString =
      ((Option.map Codec.address.encode keys[j.toNat]?).getD .unit).asString → i = j)
    (valid : Maps.Valid .sequence map)
    (index : Nat) (within : index < keys.size) (key : String) (atIndex : keys[index]? = some key)
    (layout : Maps.Layout) (value middle next : RuntimeValue)
    (updated : middle = Maps.update layout .sequence map (.address key) value)
    (removed : next = Maps.remove layout .sequence middle (.address key)) :
    ∀ i : Int, 0 ≤ i → i < (keys.size - 1 : Nat) →
      Maps.hasKey next (.address ((Option.map Codec.address.encode (keys.eraseIdx index within)[i.toNat]?).getD .unit).asString) = true := by
  intro i low high
  let previousIndex := if i.toNat < index then i.toNat else i.toNat + 1
  have bound : previousIndex < keys.size := by dsimp [previousIndex]; split <;> omega
  have different : previousIndex ≠ index := by dsimp [previousIndex]; split <;> omega
  have read : (keys.eraseIdx index within)[i.toNat]? = keys[previousIndex]? := by
    rw [Array.getElem?_eraseIdx within]
    split <;> rename_i h <;> simp [previousIndex, h]
  have keyRead : ((Option.map Codec.address.encode keys[index]?).getD .unit).asString = key := by
    simp [atIndex, Codec.address, RuntimeValue.asString]
  have differs : key ≠ ((Option.map Codec.address.encode keys[previousIndex]?).getD .unit).asString := by
    intro eq
    have equal := distinct index (by omega) (by omega) previousIndex (by omega) (by omega)
      (by simpa only [Int.toNat_natCast, keyRead] using eq)
    omega
  have present := covered previousIndex (by omega) (by omega)
  simp only [Int.toNat_natCast] at present
  rw [read, removed, updated, Maps.hasKey_remove layout (Maps.valid_update layout valid _ _)]
  simp [Maps.hasKey_update, differs, present]
end LeanerIR.Proofs.Denote
