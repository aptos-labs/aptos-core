-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Solve

/-! Typed map observations at the authored-proof boundary. -/

namespace VerexIR.Proofs.Denote
open Lean Elab Tactic Meta

/-- Duplicates of a list, by indices. -/
private theorem duplicate_indices {α : Type} (xs : List α) :
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

/-- Duplicate images under a map of an array's elements, by integer indices
into the array. -/
theorem duplicate_indices_map {α β : Type} (f : α → β) (xs : Array α) (absent : α) :
    (∃ i : Int, 0 ≤ i ∧ i < xs.size ∧ ∃ j : Int, 0 ≤ j ∧ j < xs.size ∧ i ≠ j ∧
        f (xs[i.toNat]?.getD absent) = f (xs[j.toNat]?.getD absent)) ↔
      ¬(xs.toList.map f).Nodup := by
  rw [duplicate_indices, Iff.comm]
  simp only [List.length_map, Array.length_toList, List.getElem_map, Array.getElem_toList]
  constructor
  · rintro ⟨i, j, hi, hj, ne, eq⟩
    refine ⟨i, by omega, by omega, j, by omega, by omega, by omega, ?_⟩
    simpa [hi, hj] using eq
  · rintro ⟨i, ilo, ihi, j, jlo, jhi, ne, eq⟩
    have hi : i.toNat < xs.size := by omega
    have hj : j.toNat < xs.size := by omega
    refine ⟨i.toNat, j.toNat, hi, hj, by omega, ?_⟩
    simpa [hi, hj] using eq

/-- A value of a two-variant enum named by its second variant is that
variant's payload. -/
theorem variantCarrier.second_of_name {unit : Validation.ValidatedUnit} [Skolems unit]
    (first second : String) (distinct : first ≠ second)
    {leading trailing : NRow} {value : variantCarrier [first, second] (.cons leading (.cons trailing .nil))}
    (named : variantName [first, second] (.cons leading (.cons trailing .nil)) value = second) :
    ∃ payload : HList trailing,
      value = @Sum.inr (HList leading) (variantCarrier [second] (.cons trailing .nil)) (.inl payload) := by
  rcases value with payload | payload | empty
  · exact absurd named distinct
  · exact ⟨payload, rfl⟩
  · cases empty

/-- The inexpensive authored preparation, ending with typed map observations. -/
macro "verex_prepare" &"values" : tactic => `(tactic| (
  verex_paths
  all_goals verex_canonical_families
  all_goals verex_clear_computations
  all_goals try verex_unname
  all_goals verex_split_hypotheses
  all_goals verex_subst_vars
  all_goals verex_reduce_projections
  all_goals try verex_trivial
  all_goals verex_normalize_context
  all_goals verex_split_hypotheses
  all_goals verex_subst_vars
  all_goals verex_reduce_projections
  all_goals verex_intro_any
  all_goals verex_type_bounds
  all_goals verex_finish_prepare
  all_goals try verex_trivial))

macro "verex_prepare" &"typed" : tactic => `(tactic| (
  verex_prepare values
  all_goals verex_name_maps))

end VerexIR.Proofs.Denote
