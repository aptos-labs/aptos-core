-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "behavior_manual_facts.move"
verex proofs where

verify caller by
  verex_obligations
  all_goals (first | verex_decide | (verex_prepare; all_goals try verex_decide))
  all_goals simp_all

verify wrong_final_value by
  verex_obligations
  all_goals (first | verex_decide | (verex_prepare; all_goals try verex_decide))
  all_goals (first | (solve | simp_all) | fail "incorrect final counter value")
