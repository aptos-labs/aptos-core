-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- Unfold the accumulator's recursive specification, rewrite the goal from
-- those equations, and evaluate the finite index cases for overflow checks.
verify sum_concrete by
  leaner_obligations
  all_goals (first | leaner_decide | (leaner_prepare; all_goals try leaner_decide))
  all_goals (try leaner_unfold_specs)
  all_goals (try leaner_simp_by_context)
  all_goals first | omega | leaner_decide_ranges

verify sum_inferred by
  leaner_obligations
  all_goals (first | leaner_decide | (leaner_prepare; all_goals try leaner_decide))
  all_goals (try leaner_unfold_specs)
  all_goals (try leaner_simp_by_context)
  all_goals first | omega | leaner_decide_ranges

verify sum_scaled by
  leaner_obligations
  all_goals (first | leaner_decide | (leaner_prepare; all_goals try leaner_decide))
  all_goals (try leaner_unfold_specs)
  all_goals (try leaner_simp_by_context)
  all_goals first | omega | leaner_decide_ranges

verify product_concrete by
  leaner_obligations
  all_goals (first | leaner_decide | (leaner_prepare; all_goals try leaner_decide))
  all_goals (try leaner_unfold_specs)
  all_goals (try leaner_simp_by_context)
  all_goals first | omega | leaner_decide_ranges
