-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "amm_example.move"
verex proofs where

-- The denominator contains the input, so division cannot exceed the output
-- reserve; a positive input reserve makes that bound strict.
verify constant_product by
  verex_obligations
  all_goals (first | verex_decide | (verex_prepare; all_goals try verex_decide))
  all_goals
    have bound : reserve_out.val * amount_in.val / (reserve_in.val + amount_in.val)
        ≤ reserve_out.val := by
      apply Int.ediv_le_of_le_mul (by omega)
      exact Int.mul_le_mul_of_nonneg_left (by omega) (by omega)
    first
      | omega
      | (intro positive_in positive_out positive_amount
         apply Int.ediv_lt_of_lt_mul (by omega)
         exact Int.mul_lt_mul_of_pos_left (by omega) positive_out)
