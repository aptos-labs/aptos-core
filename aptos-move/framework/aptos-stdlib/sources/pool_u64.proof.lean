-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- Relate the shares computed in the pre-state to the summaries at the
-- add_shares call after the pool totals have changed.
verify buy_in by
  verex_obligations
  all_goals (first | verex_decide | (verex_prepare; all_goals try verex_decide))
  all_goals verex_clear_computations
  all_goals (try clear native_add_shares)
  all_goals (try clear native_amount_to_shares)
  all_goals (try clear «native_0x1::error::canonical»)
  all_goals verex_scalar_equalities
  all_goals first | omega | grind -ring only

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
