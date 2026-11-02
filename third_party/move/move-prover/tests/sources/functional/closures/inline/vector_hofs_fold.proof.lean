-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "vector_hofs_fold.move"
verex proofs where

open VerexIR VerexIR.Proofs VerexIR.Proofs.Denote

verify count_even_concrete by lean =>
  have even_prefix (i : Int) (low : 0 ≤ i) (high : i ≤ 6) :
      «spec_fold$lambda$0».spec unit (Skolems.runtime unit)
        (({ values := #[⟨1, by decide⟩, ⟨2, by decide⟩, ⟨3, by decide⟩, ⟨4, by decide⟩,
            ⟨5, by decide⟩, ⟨6, by decide⟩], bounded := by decide } :
          SpecVector (SpecInt (.bits 64) false)), 0, i, ()) = i / 2 := by
    have cases_i : i = 0 ∨ i = 1 ∨ i = 2 ∨ i = 3 ∨ i = 4 ∨ i = 5 ∨ i = 6 := by omega
    rcases cases_i with rfl | rfl | rfl | rfl | rfl | rfl | rfl
    all_goals simp [«spec_fold$lambda$0».spec.unfold, vir_denote_norm]
  verex_obligations
  all_goals verex_prepare only
  all_goals simp (disch := omega) only [even_prefix] at *
  all_goals try verex_decide
  all_goals
    have index_cases : i.val = 0 ∨ i.val = 1 ∨ i.val = 2 ∨ i.val = 3 ∨ i.val = 4 ∨ i.val = 5 := by omega
    rcases index_cases with at_i | at_i | at_i | at_i | at_i | at_i
    all_goals simp_all only [vir_denote_norm]
    all_goals omega
