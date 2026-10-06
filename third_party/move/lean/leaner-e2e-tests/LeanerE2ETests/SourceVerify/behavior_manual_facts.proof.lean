-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

verify caller by
  all_goals simp_all

verify wrong_final_value by
  all_goals (first | (solve | simp_all) | fail "incorrect final counter value")
