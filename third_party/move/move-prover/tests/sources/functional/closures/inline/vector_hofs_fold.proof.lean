-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- Unfold the accumulator's recursive specification, rewrite the goal from
-- those equations, and evaluate the finite index cases for overflow checks.
verify sum_concrete by
  all_goals (try leaner_denote_unfold_specs)
  all_goals (try leaner_denote_simp_by_context)
  all_goals first | omega | leaner_denote_decide_ranges

verify sum_inferred by
  all_goals (try leaner_denote_unfold_specs)
  all_goals (try leaner_denote_simp_by_context)
  all_goals first | omega | leaner_denote_decide_ranges

verify sum_scaled by
  all_goals (try leaner_denote_unfold_specs)
  all_goals (try leaner_denote_simp_by_context)
  all_goals first | omega | leaner_denote_decide_ranges

verify product_concrete by
  all_goals (try leaner_denote_unfold_specs)
  all_goals (try leaner_denote_simp_by_context)
  all_goals first | omega | leaner_denote_decide_ranges
