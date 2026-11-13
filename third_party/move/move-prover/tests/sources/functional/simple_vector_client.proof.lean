-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "simple_vector_client.move"
verex proofs where

-- Literal list normalization is supplied by the shared proof library.
verify test_contains by lean =>
  verex_obligations
  all_goals (first | verex_decide | (verex_prepare; all_goals try verex_decide))
  all_goals simp_all [vir_denote_norm]
  · exact ⟨_, ⟨3, by decide, by decide, rfl⟩, rfl⟩
  · intro x i lo hi eq
    have cases_i : i = 0 ∨ i = 1 ∨ i = 2 ∨ i = 3 := by omega
    rcases cases_i with rfl | rfl | rfl | rfl
    all_goals subst x; exact of_decide_eq_true rfl
  · exact ⟨_, ⟨2, by decide, by decide, rfl⟩, rfl⟩
  · exact ⟨_, ⟨1, by decide, by decide, rfl⟩, rfl⟩
  · exact ⟨_, ⟨0, by decide, by decide, rfl⟩, rfl⟩

verify test_index_of by lean =>
  verex_obligations
  all_goals (first | verex_decide | (verex_prepare; all_goals try verex_decide))
  all_goals simp_all [vir_denote_norm]
