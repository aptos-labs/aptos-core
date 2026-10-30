-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "bitwise_table.move"
verex proofs where

-- The table holds 22 at the first key after both additions; the borrow
-- reads it, and the write stores `22 & 1`.
verify add_R by lean =>
  verex_obligations
  verex_on
  | ensures_1 =>
    verex_prepare only
    subst old_value'' old_value' old_value
    simp only [vir_denote_norm] at *
    rfl
  | ensures_3 =>
    verex_prepare only
    subst old_value'' old_value' old_value
    simp only [VerexIR.SpecMap.valueAt_update, vir_denote_norm] at *
    rw [← (‹22 = borrow_mut_result_0.val› : 22 = borrow_mut_result_0.val)]
    decide
  | _ => first | verex_decide | (verex_prepare only; all_goals verex_decide)
