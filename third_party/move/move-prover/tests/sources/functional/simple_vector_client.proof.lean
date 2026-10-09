-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- Keep literal list spines computational while preparing vector obligations.
-- These hints belong to this fixture; each theorem is kernel checked.
@[lir_denote_norm] theorem literal_nil_append {α : Type} (xs : List α) :
    [] ++ xs = xs := List.nil_append xs
@[lir_denote_norm] theorem literal_cons_append {α : Type} (x : α) (xs ys : List α) :
    (x :: xs) ++ ys = x :: (xs ++ ys) := rfl
@[lir_denote_norm] theorem literal_length_cons {α : Type} (x : α) (xs : List α) :
    (x :: xs).length = xs.length + 1 := rfl
@[lir_denote_norm] theorem literal_length_nil {α : Type} :
    ([] : List α).length = 0 := rfl
@[lir_denote_norm] theorem literal_get_zero {α : Type} (x : α) (xs : List α) :
    (x :: xs)[0] = x := rfl
@[lir_denote_norm] theorem literal_get_succ {α : Type} (x : α) (xs : List α)
    (i : Nat) (h : i + 1 < (x :: xs).length) :
    (x :: xs)[i + 1] = xs[i]'(by simpa using h) := rfl

verify test_contains by
  all_goals simp_all [lir_denote_norm]
  · exact ⟨_, ⟨3, by decide, by decide, rfl⟩, rfl⟩
  · intro x i lo hi eq
    have cases_i : i = 0 ∨ i = 1 ∨ i = 2 ∨ i = 3 := by omega
    rcases cases_i with rfl | rfl | rfl | rfl
    all_goals subst x; exact of_decide_eq_true rfl
  · exact ⟨_, ⟨2, by decide, by decide, rfl⟩, rfl⟩
  · exact ⟨_, ⟨1, by decide, by decide, rfl⟩, rfl⟩
  · exact ⟨_, ⟨0, by decide, by decide, rfl⟩, rfl⟩

verify test_index_of by
  all_goals simp_all [lir_denote_norm]

verify option_type by
  all_goals simp_all [lir_denote_norm]
