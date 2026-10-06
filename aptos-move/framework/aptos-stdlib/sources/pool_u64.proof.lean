-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- Relate the shares computed in the pre-state to the summaries at the
-- add_shares call after the pool totals have changed.
verify buy_in by
  all_goals leaner_denote_clear_computations
  all_goals (try clear native_add_shares)
  all_goals (try clear native_amount_to_shares)
  all_goals (try clear «native_0x1::error::canonical»)
  all_goals leaner_denote_scalar_equalities
  all_goals first | omega | grind -ring only

-- The ignored index_of flag leaves both search branches. Equal cardinality
-- and distinctness rule out a missing shareholder; a matching removal
-- preserves membership for every surviving shareholder.
verify deduct_shares by
  all_goals first
  | leaner_denote_decide_prepared
  | leaner_denote_map_coverage_prepared
