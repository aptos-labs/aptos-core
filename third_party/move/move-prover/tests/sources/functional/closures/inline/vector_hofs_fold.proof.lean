-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- Unfold the accumulator's recursive specification, rewrite the goal from
-- those equations, and evaluate the finite index cases for overflow checks.
verify sum_concrete by
  verex_obligations
  all_goals (first | verex_decide | (verex_prepare; all_goals try verex_decide))
  all_goals (try verex_unfold_specs)
  all_goals (try verex_simp_by_context)
  all_goals first | omega | verex_decide_ranges

verify sum_inferred by
  verex_obligations
  all_goals (first | verex_decide | (verex_prepare; all_goals try verex_decide))
  all_goals (try verex_unfold_specs)
  all_goals (try verex_simp_by_context)
  all_goals first | omega | verex_decide_ranges

verify sum_scaled by
  verex_obligations
  all_goals (first | verex_decide | (verex_prepare; all_goals try verex_decide))
  all_goals (try verex_unfold_specs)
  all_goals (try verex_simp_by_context)
  all_goals first | omega | verex_decide_ranges

verify product_concrete by
  verex_obligations
  all_goals (first | verex_decide | (verex_prepare; all_goals try verex_decide))
  all_goals (try verex_unfold_specs)
  all_goals (try verex_simp_by_context)
  all_goals first | omega | verex_decide_ranges
