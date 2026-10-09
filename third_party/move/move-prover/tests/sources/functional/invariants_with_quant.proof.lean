-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- All quantified positions lie in the three-element result. Enumerate those
-- positions, then let the kernel compute the literal vector reads.
verify vector_of_proper_positives by
  all_goals simp only [List.nil_append, List.cons_append, List.length_cons, List.length_nil]
  · intro i lo hi
    have cases_i : i = 0 ∨ i = 1 ∨ i = 2 := by omega
    rcases cases_i with rfl | rfl | rfl <;> decide
  · intro i ilo ihi j jlo jhi eq _ iend _ jend
    have cases_i : i = 0 ∨ i = 1 ∨ i = 2 := by omega
    have cases_j : j = 0 ∨ j = 1 ∨ j = 2 := by omega
    rcases cases_i with rfl | rfl | rfl
    all_goals rcases cases_j with rfl | rfl | rfl
    all_goals first | rfl | contradiction
  · intro i ilo iend j jlo jend eq
    have cases_i : i = 0 ∨ i = 1 ∨ i = 2 := by omega
    have cases_j : j = 0 ∨ j = 1 ∨ j = 2 := by omega
    rcases cases_i with rfl | rfl | rfl
    all_goals rcases cases_j with rfl | rfl | rfl
    all_goals first | rfl | contradiction
  · intro i lo hi
    have cases_i : i = 0 ∨ i = 1 ∨ i = 2 := by omega
    rcases cases_i with rfl | rfl | rfl
    all_goals rintro x ⟨h, rfl⟩
    all_goals simp
