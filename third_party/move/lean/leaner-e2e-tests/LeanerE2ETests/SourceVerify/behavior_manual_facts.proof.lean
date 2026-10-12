-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

verify caller by
  leaner_obligations
  all_goals (first | leaner_decide | (leaner_prepare; all_goals try leaner_decide))
  all_goals simp_all

verify wrong_final_value by
  leaner_obligations
  all_goals (first | leaner_decide | (leaner_prepare; all_goals try leaner_decide))
  all_goals (first | (solve | simp_all) | fail "incorrect final counter value")
