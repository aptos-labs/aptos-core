-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Maps.Model
import VerexIR.Proofs.Denote.Types
open VerexIR VerexIR.Proofs VerexIR.Proofs.Denote
namespace VerexIR.Proofs.Denote

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

variable {K V : Type} [DecidableEq K]

/-- A distinct vector covering a map of the same size contains every map
key. A vector's element is read as a clause reads it, its fallback outside
the vector. -/
theorem map_key_mem_of_coverage (map : SpecMap K V) (keys : Array K) (fallback : K)
    (covered : ∀ i : Int, 0 ≤ i → i < keys.size →
      SpecMap.hasKey map (keys[i.toNat]?.getD fallback) = true)
    (cardinality : SpecMap.size map = keys.size)
    (distinct : ∀ i : Int, 0 ≤ i → i < keys.size → ∀ j : Int, 0 ≤ j → j < keys.size →
      keys[i.toNat]?.getD fallback = keys[j.toNat]?.getD fallback → i = j)
    (key : K) (present : SpecMap.hasKey map key = true) : key ∈ keys := by
  have simple (i : Nat) (within : i < keys.size) :
      keys[(i : Int).toNat]?.getD fallback = keys[i] := by
    simp [Array.getElem?_eq_getElem within]
  have nodup : keys.toList.Nodup := by
    apply List.pairwise_iff_getElem.mpr
    intro i j hi hj lt eq
    simp only [Array.length_toList] at hi hj
    have eq' : keys[i] = keys[j] := by simpa using eq
    have := distinct i (by omega) (by omega) j (by omega) (by omega)
      (by simpa only [simple i hi, simple j hj] using eq')
    omega
  have subset : keys.toList ⊆ map.map Prod.fst := by
    intro item mem
    have mem : item ∈ keys := by simpa using mem
    obtain ⟨i, hi, rfl⟩ := Array.getElem_of_mem mem
    have found := covered i (by omega) (by omega)
    rw [simple i hi] at found
    obtain ⟨value, mem⟩ := (SpecMap.containsB_iff _ _).mp found
    exact List.mem_map.mpr ⟨(_, value), mem, rfl⟩
  have mem : key ∈ map.map Prod.fst := by
    obtain ⟨value, mem⟩ := (SpecMap.containsB_iff _ _).mp present
    exact List.mem_map.mpr ⟨(_, value), mem, rfl⟩
  have card : keys.toList.length = (map.map Prod.fst).length := by
    simp only [List.length_map, Array.length_toList]
    simp only [SpecMap.size] at cardinality
    omega
  have := mem_of_subset_of_length nodup subset card mem
  simpa using this

/-- A present key cannot be absent from a complete distinct enumeration. -/
theorem map_key_search_absurd (map : SpecMap K V) (keys : Array K) (fallback : K)
    (covered : ∀ i : Int, 0 ≤ i → i < keys.size →
      SpecMap.hasKey map (keys[i.toNat]?.getD fallback) = true)
    (cardinality : SpecMap.size map = keys.size)
    (distinct : ∀ i : Int, 0 ≤ i → i < keys.size → ∀ j : Int, 0 ≤ j → j < keys.size →
      keys[i.toNat]?.getD fallback = keys[j.toNat]?.getD fallback → i = j)
    (key : K) (present : SpecMap.hasKey map key = true)
    (missing : ∀ i : Int, 0 ≤ i → i < keys.size → ¬keys[i.toNat]? = some key) : False := by
  have mem := map_key_mem_of_coverage map keys fallback covered cardinality distinct key present
  obtain ⟨i, hi, eq⟩ := Array.getElem_of_mem mem
  apply missing i (by omega) (by omega)
  simp [Array.getElem?_eq_getElem hi, eq]

/-- Removing a matching vector element and map key preserves coverage. The
intermediate update models a mutable borrow ending before the map removal. -/
theorem map_keys_after_remove (map : SpecMap K V) (keys : Array K) (fallback : K)
    (covered : ∀ i : Int, 0 ≤ i → i < keys.size →
      SpecMap.hasKey map (keys[i.toNat]?.getD fallback) = true)
    (distinct : ∀ i : Int, 0 ≤ i → i < keys.size → ∀ j : Int, 0 ≤ j → j < keys.size →
      keys[i.toNat]?.getD fallback = keys[j.toNat]?.getD fallback → i = j)
    (valid : SpecMap.Valid .sequence map)
    (index : Nat) (within : index < keys.size) (key : K) (atIndex : keys[index]? = some key)
    (value : V) (middle next : SpecMap K V)
    (updated : middle = SpecMap.update .sequence map key value)
    (removed : next = SpecMap.remove .sequence middle key) :
    ∀ i : Int, 0 ≤ i → i < (keys.size - 1 : Nat) →
      SpecMap.hasKey next ((keys.eraseIdx index within)[i.toNat]?.getD fallback) = true := by
  intro i low high
  let previousIndex := if i.toNat < index then i.toNat else i.toNat + 1
  have bound : previousIndex < keys.size := by dsimp [previousIndex]; split <;> omega
  have different : previousIndex ≠ index := by dsimp [previousIndex]; split <;> omega
  have read : (keys.eraseIdx index within)[i.toNat]? = keys[previousIndex]? := by
    rw [Array.getElem?_eraseIdx within]
    split <;> rename_i h <;> simp [previousIndex, h]
  have keyRead : keys[index]?.getD fallback = key := by simp [atIndex]
  have differs : key ≠ keys[previousIndex]?.getD fallback := by
    intro eq
    have equal := distinct index (by omega) (by omega) previousIndex (by omega) (by omega)
      (by simpa only [Int.toNat_natCast, keyRead] using eq)
    omega
  have present := covered previousIndex (by omega) (by omega)
  simp only [Int.toNat_natCast] at present
  rw [read, removed, updated, SpecMap.hasKey_remove (SpecMap.valid_update valid _ _)]
  simp [SpecMap.hasKey_update, differs, present]
end VerexIR.Proofs.Denote
