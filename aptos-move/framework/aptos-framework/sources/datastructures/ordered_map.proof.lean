-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- The loop preserves the map's enumeration by removing its first entry.
-- Use the map position laws directly on the prepared loop obligations.
verify test_verify_drain_symbolic by
  all_goals grind

-- The constructor's abort condition is exactly unequal lengths or duplicate keys.
verify test_aborts_if_new_from_1 by
  all_goals simp_all [LeanerIR.Maps.AbortsNewFrom, LeanerIR.Maps.elementsOf]

verify test_verify_borrow_front_key by
  all_goals first
    | exact ⟨⟨1, by decide⟩, ⟨0, by decide, by decide, by decide, rfl⟩, rfl⟩
    | exact ⟨⟨2, by decide⟩, ⟨1, by decide, by decide, by decide, rfl⟩, rfl⟩
    | exact ⟨⟨3, by decide⟩, ⟨2, by decide, by decide, by decide, rfl⟩, rfl⟩
    | simp_all [lir_denote_norm, LeanerIR.Maps.Valid, LeanerIR.Maps.Ascending,
        LeanerIR.RuntimeValue.order_integer, compare, compareOfLessAndEq]
  all_goals leaner_denote_canonical_families
  all_goals simp_all [lir_denote_norm]

verify test_verify_borrow_back_key by
  all_goals first
    | exact ⟨⟨1, by decide⟩, ⟨0, by decide, by decide, by decide, rfl⟩, rfl⟩
    | exact ⟨⟨2, by decide⟩, ⟨1, by decide, by decide, by decide, rfl⟩, rfl⟩
    | exact ⟨⟨3, by decide⟩, ⟨2, by decide, by decide, by decide, rfl⟩, rfl⟩
    | simp_all [lir_denote_norm, LeanerIR.Maps.Valid, LeanerIR.Maps.Ascending,
        LeanerIR.RuntimeValue.order_integer, compare, compareOfLessAndEq]
  all_goals leaner_denote_canonical_families
  all_goals simp_all [lir_denote_norm]

-- The prepared rank and ordering facts establish the three-key enumeration.
verify ground_enum_123 by
  all_goals grind

-- The ground tests construct the same three keys with different values.
-- Compute that constructor once, before symbolic map-position reasoning.
open LeanerIR LeanerIR.Maps in
@[lir_denote_norm] theorem build_three (layout : Layout) (ranks : ValueRanks)
    (v1 v2 v3 : RuntimeValue) :
    updateAll layout (.ordered ranks) (empty layout)
      (.vector #[.integer 1, .integer 2, .integer 3]) (.vector #[v1, v2, v3]) =
      layout.build [(.integer 1, v1), (.integer 2, v2), (.integer 3, v3)] := by
  simp [updateAll, elementsOf, empty, Discipline.set, ordSet, compare, compareOfLessAndEq]

open LeanerIR LeanerIR.Maps in
@[lir_denote_norm] theorem size_built (layout : Layout) (entries : Entries) :
    size (layout.build entries) = entries.length := by simp [size]

open LeanerIR LeanerIR.Maps in
@[lir_denote_norm] theorem hasKey_built_nil (layout : Layout) (key : RuntimeValue) :
    hasKey (layout.build []) key = false := by simp only [hasKey, entriesOf_build]; rfl

open LeanerIR LeanerIR.Maps in
@[lir_denote_norm] theorem hasKey_built_cons (layout : Layout)
    (entry : RuntimeValue × RuntimeValue) (entries : Entries) (key : RuntimeValue) :
    hasKey (layout.build (entry :: entries)) key =
      (decide (entry.1 = key) || hasKey (layout.build entries) key) := by
  by_cases same : entry.1 = key <;> simp [hasKey, containsB, same]

verify test_verify_remove_or_none by
  all_goals simp_all [lir_denote_norm, LeanerIR.Maps.size, LeanerIR.Maps.hasKey,
    LeanerIR.Maps.containsB, LeanerIR.Maps.remove, LeanerIR.Maps.Discipline.del,
    LeanerIR.Maps.ordDel, LeanerIR.Maps.AbortsNewFrom, LeanerIR.Maps.elementsOf,
    LeanerIR.Maps.Valid, LeanerIR.Maps.Ascending, LeanerIR.RuntimeValue.order_integer,
    compare, compareOfLessAndEq]
  all_goals leaner_denote_canonical_families
  all_goals simp_all [lir_denote_norm]

-- The model uses Nodup; the Move contract states duplicate integer indices.

theorem duplicate_indices {α : Type} (xs : List α) :
    ¬xs.Nodup ↔ ∃ (i j : Nat) (hi : i < xs.length) (hj : j < xs.length),
      i ≠ j ∧ xs[i] = xs[j] := by
  classical
  rw [List.nodup_iff_pairwise_ne, List.pairwise_iff_getElem]
  constructor
  · intro h
    simp only [Classical.not_forall, Classical.not_not] at h
    obtain ⟨i, j, hi, hj, lt, eq⟩ := h
    exact ⟨i, j, hi, hj, by omega, eq⟩
  · rintro ⟨i, j, hi, hj, ne, eq⟩ h
    by_cases lt : i < j
    · exact h i j hi hj lt eq
    · exact h j i hj hi (by omega) eq.symm

open LeanerIR LeanerIR.Proofs LeanerIR.Proofs.Denote in
theorem duplicate_u64 (keys : SpecVector (SpecInt (.bits 64) false)) :
    ¬(keys.values.map (Codec.specInt (.bits 64) false).encode).toList.Nodup ↔
      ∃ i : Int, 0 ≤ i ∧ i < keys.values.size ∧
        ∃ j : Int, 0 ≤ j ∧ j < keys.values.size ∧ i ≠ j ∧
          ((keys.values[i.toNat]?.map (Codec.specInt (.bits 64) false).encode).getD .unit).asInt =
          ((keys.values[j.toNat]?.map (Codec.specInt (.bits 64) false).encode).getD .unit).asInt := by
  rw [duplicate_indices]
  simp only [Array.length_toList, Array.size_map, Array.getElem_toList, Array.getElem_map]
  constructor
  · rintro ⟨i, j, hi, hj, ne, eq⟩
    refine ⟨i, by omega, by omega, j, by omega, by omega, by omega, ?_⟩
    simpa [hi, hj, Codec.specInt] using eq
  · rintro ⟨i, ilo, ihi, j, jlo, jhi, ne, eq⟩
    have hi : i.toNat < keys.values.size := by omega
    have hj : j.toNat < keys.values.size := by omega
    refine ⟨i.toNat, j.toNat, hi, hj, by omega, ?_⟩
    simpa [hi, hj, Codec.specInt] using eq

verify test_aborts_if_new_from_2 by
  all_goals have duplicateLaw := duplicate_u64 keys
  all_goals simp only [LeanerIR.Maps.AbortsNewFrom, LeanerIR.Maps.elementsOf,
    Array.toList_map, List.length_map, Array.length_toList] at *
  all_goals grind only

verify test_verify_next_key by
  all_goals simp_all [LeanerIR.Maps.nextKey?,
    LeanerIR.RuntimeValue.order_integer, compare, compareOfLessAndEq]

verify test_verify_prev_key by
  all_goals simp_all [LeanerIR.Maps.prevKey?,
    LeanerIR.RuntimeValue.order_integer, compare, compareOfLessAndEq]

open LeanerIR LeanerIR.Maps in
@[lir_denote_norm] theorem keyAt_built (layout : Layout) (entries : Entries) (index : Int) :
    keyAt (layout.build entries) index =
      (if index < 0 then .unit else ((entries[index.toNat]?).map Prod.fst).getD .unit) := by
  simp only [keyAt, entriesOf_build]

open LeanerIR LeanerIR.Maps in
@[lir_denote_norm] theorem valueAt_built (layout : Layout) (entries : Entries) (key : RuntimeValue) :
    valueAt (layout.build entries) key = get entries key := by simp [valueAt]

open LeanerIR LeanerIR.Maps in
@[lir_denote_norm] theorem rank_built (layout : Layout) (entries : Entries) (key : RuntimeValue) :
    rank (layout.build entries) key = indexOf entries key := by simp [rank]

-- Evaluate reads of the concrete entry lists before generating symbolic rank facts.
open LeanerIR LeanerIR.Maps in
@[lir_denote_norm] theorem indexOf_nil (key : RuntimeValue) :
    indexOf [] key = 0 := rfl

open LeanerIR LeanerIR.Maps in
@[lir_denote_norm] theorem indexOf_cons (entry : RuntimeValue × RuntimeValue)
    (entries : Entries) (key : RuntimeValue) :
    indexOf (entry :: entries) key = (if entry.1 = key then 0 else indexOf entries key + 1) := by
  by_cases same : entry.1 = key <;> simp [indexOf, same]

open LeanerIR LeanerIR.Maps in
@[lir_denote_norm] theorem get_nil (key : RuntimeValue) : get [] key = .unit := rfl

open LeanerIR LeanerIR.Maps in
@[lir_denote_norm] theorem get_cons (entry : RuntimeValue × RuntimeValue)
    (entries : Entries) (key : RuntimeValue) :
    get (entry :: entries) key = (if entry.1 = key then entry.2 else get entries key) := by
  by_cases same : entry.1 = key <;> simp [LeanerIR.Maps.get, same]

verify test_verify_enumeration_view by
  all_goals first
    | exact ⟨⟨1, by decide⟩, ⟨0, by decide, by decide, by decide, rfl⟩, rfl⟩
    | exact ⟨⟨2, by decide⟩, ⟨1, by decide, by decide, by decide, rfl⟩, rfl⟩
    | exact ⟨⟨3, by decide⟩, ⟨2, by decide, by decide, by decide, rfl⟩, rfl⟩
    | simp_all [lir_denote_norm, LeanerIR.Maps.Valid, LeanerIR.Maps.Ascending,
        LeanerIR.RuntimeValue.order_integer, compare, compareOfLessAndEq]
  all_goals leaner_denote_canonical_families
  all_goals simp_all [lir_denote_norm]

open LeanerIR LeanerIR.Maps in
@[lir_denote_norm] theorem remove_built (layout : Layout) (ranks : ValueRanks)
    (entries : Entries) (key : RuntimeValue) :
    remove layout (.ordered ranks) (layout.build entries) key =
      layout.build (ordDel key entries) := by
  simp [remove]

open LeanerIR LeanerIR.Maps in
@[lir_denote_norm] theorem ordDel_nil (key : RuntimeValue) : ordDel key [] = [] := rfl

open LeanerIR LeanerIR.Maps in
@[lir_denote_norm] theorem ordDel_cons (entry : RuntimeValue × RuntimeValue)
    (entries : Entries) (key : RuntimeValue) :
    ordDel key (entry :: entries) = (if entry.1 = key then entries else entry :: ordDel key entries) := by
  by_cases same : entry.1 = key <;> simp [ordDel, same]

verify test_verify_pop_rank by
  all_goals first
    | exact ⟨⟨1, by decide⟩, ⟨0, by decide, by decide, by decide, rfl⟩, rfl⟩
    | exact ⟨⟨2, by decide⟩, ⟨1, by decide, by decide, by decide, rfl⟩, rfl⟩
    | exact ⟨⟨3, by decide⟩, ⟨2, by decide, by decide, by decide, rfl⟩, rfl⟩
    | simp_all [lir_denote_norm, LeanerIR.Maps.Valid, LeanerIR.Maps.Ascending,
        LeanerIR.RuntimeValue.order_integer, compare, compareOfLessAndEq]
  all_goals leaner_denote_canonical_families
  all_goals simp_all [lir_denote_norm]

verify test_verify_upsert by
  all_goals simp_all [lir_denote_norm, LeanerIR.Maps.update, LeanerIR.Maps.Discipline.set,
    LeanerIR.Maps.ordSet, LeanerIR.Maps.Valid, LeanerIR.Maps.Ascending,
    LeanerIR.RuntimeValue.order_integer, compare, compareOfLessAndEq]
  all_goals leaner_denote_canonical_families
  all_goals simp_all [lir_denote_norm]
