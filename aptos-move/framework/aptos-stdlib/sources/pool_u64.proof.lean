-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "pool_u64.move"
verex proofs where

open VerexIR VerexIR.Maps VerexIR.Proofs VerexIR.Proofs.Denote

-- The shares result supplies both the shareholder balance and its bounds.
-- Products are nonnegative, so truncating and Euclidean division agree.
verify balance by
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
verify buy_in by
  verex_obligations
  verex_on
  | ensures_3 =>
    verex_paths
    all_goals try verex_unname
    all_goals try verex_prepare_leaf
    all_goals try verex_scalar_equalities
    all_goals verex_intro_any
    all_goals grind -ring only
  | _ => skip
  all_goals verex_prepare only
  all_goals try verex_decide
  all_goals verex_scalar_equalities
  all_goals try simp (disch := verex_arithmetic_only) only [if_pos, Int.zero_mul, Int.zero_ediv] at *
  all_goals try verex_decide
  case aborts_if_1_8 =>
    exact Or.inl (Or.inl (Or.inl (by verex_arithmetic_only)))
  case aborts_if_1_10 =>
    have positive : 0 < coins_amount.val := by omega
    rename_i aborted
    rcases aborted with ⟨present, overflow⟩ | ⟨missing, nonzero, full⟩
    · exact Or.inl (Or.inr ⟨positive, present, overflow⟩)
    · exact Or.inr ⟨positive, missing, nonzero, full⟩
  case after_add_shares_1 =>
    grind -ring only

-- A successful deduction supplies nonnegative remaining totals. Separate
-- the membership flag before the final arithmetic check.
verify redeem_shares by
  verex_obligations
  all_goals verex_prepare only
  all_goals try verex_decide
  all_goals verex_scalar_equalities
  all_goals try simp (disch := verex_arithmetic_only) only [if_pos, Int.zero_mul, Int.zero_ediv] at *
  all_goals try verex_decide
  all_goals cases present : hasKey self_shares (RuntimeValue.address shareholder)
  all_goals simp only [present, Bool.false_eq_true, Bool.true_eq_false, false_or] at *
  all_goals omega

-- The ignored index_of flag leaves both search branches. Equal cardinality
-- and distinctness rule out a missing shareholder; a matching removal
-- preserves membership for every surviving shareholder. Use this argument
-- before general search on the quantified map facts.
verify deduct_shares by
  verex_obligations
  all_goals (first | verex_decide | (verex_prepare; all_goals try verex_decide))
  all_goals first
  | verex_map_coverage_prepared
  | verex_decide_prepared
