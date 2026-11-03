-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "smart_table.move"
verex proofs where

-- The search of a bucket: earlier positions by the invariant, the current one
-- by the element the iteration read. `lean =>` because a step cannot state a
-- position's element from an equation of positions.
verify contains by
  on invariant 3 step {
    lean =>
      verex_prepare
      all_goals (
        by_cases same : j = i.val
        · subst same
          have stated := ‹(_ : Array _)[_]? = some _›
          simp only [stated, Option.getD_some]
          exact fun ⟨first, _, third⟩ => ‹¬(_ ∧ _)› ⟨first, third⟩
        · exact invariant_3 j ‹_› (by omega))
  }
  on assert 1 {
    lean =>
      verex_prepare
      all_goals first
        | exact invariant_3 _ ‹_› (by omega)
        | (have stated := ‹(_ : Array _)[_]? = some _›
           refine ⟨fun found => ⟨i.val, by omega, by omega, ?_⟩, fun _ => by assumption⟩
           simp only [stated, Option.getD_some]
           exact ⟨found, found, by first | rfl | assumption⟩)
  }
  on _ { }
