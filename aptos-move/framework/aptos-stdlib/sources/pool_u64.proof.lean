-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "pool_u64.move"
verex proofs where

open VerexIR VerexIR.Maps VerexIR.Proofs VerexIR.Proofs.Denote

-- The shares result supplies both the shareholder balance and its bounds.
-- Products are nonnegative, so truncating and Euclidean division agree.
verify balance by lean =>
  verex_obligations
  all_goals verex_prepare only
  all_goals try verex_decide
  all_goals have nonneg : 0 ≤ shares_result.val * self_total_coins.val :=
    Int.mul_nonneg (by omega) (by omega)
  all_goals simp only [← shares_ensures_1, Int.tdiv_eq_ediv_of_nonneg nonneg] at *
  all_goals try simp (disch := omega) only [if_pos] at *
  all_goals verex_decide

-- Preserve the typed-map equality for the zero-share case before naming
-- abstract maps; the remaining obligations use the callee's scalar summaries.
verify buy_in by lean =>
  verex_obligations
  verex_on
  | ensures_3 =>
    verex_paths
    all_goals try verex_unname
    all_goals try verex_prepare_leaf
    all_goals try verex_scalar_equalities
    all_goals verex_intro_any
    all_goals grind -ring only
  | aborts_if 1 when aborts at op 3 =>
    verex_prepare only
    all_goals verex_scalar_equalities
    all_goals try simp (disch := verex_arithmetic_only) only [if_pos, Int.zero_mul, Int.zero_ediv] at *
    exact Or.inl (Or.inl (Or.inl (by omega)))
  | aborts_if 1 when aborts at call 1 of add_shares =>
    verex_prepare only
    all_goals verex_scalar_equalities
    all_goals try simp (disch := verex_arithmetic_only) only [if_pos, Int.zero_mul, Int.zero_ediv] at *
    have positive : 0 < coins_amount.val := by omega
    rename_i aborted
    rcases aborted with ⟨present, overflow⟩ | ⟨missing, nonzero, full⟩
    · exact Or.inl (Or.inr ⟨positive, present, overflow⟩)
    · exact Or.inr ⟨positive, missing, nonzero, full⟩
  | _ => skip
  all_goals verex_prepare only
  all_goals try verex_decide
  all_goals verex_scalar_equalities
  all_goals try simp (disch := verex_arithmetic_only) only [if_pos, Int.zero_mul, Int.zero_ediv] at *
  all_goals try verex_decide
  case after_call_1_of_add_shares_1 =>
    grind -ring only

-- A successful deduction supplies nonnegative remaining totals. Separate
-- the membership flag before the final arithmetic check.
verify redeem_shares by lean =>
  verex_obligations
  all_goals verex_prepare only
  all_goals try verex_decide
  all_goals verex_scalar_equalities
  all_goals try simp (disch := verex_arithmetic_only) only [if_pos, Int.zero_mul, Int.zero_ediv] at *
  all_goals try verex_decide
  all_goals split at shares_ensures_1
  all_goals rename_i present
  all_goals simp only [present, Bool.not_eq_true, Bool.false_eq_true, Bool.true_eq_false, false_or] at *
  all_goals omega
