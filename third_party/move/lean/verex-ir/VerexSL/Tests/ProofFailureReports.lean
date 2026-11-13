-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
open Lean Meta Elab Tactic

verex module 0x42::failed_finalization_reports where
  pragma verify = false
  fun rejected() -> u64 := 1
  spec rejected where
    ensures result == 2
  fun recovered(x : u64) -> u64 := x
  spec recovered where
    ensures result == x

/--
error: the specification clause `ensures result == 2` is not established
---
error: verex verification failed: the automatic verification of `rejected` failed; provide a proof in the module:
  verify rejected by lean =>
    verex_obligations
    verex_on
    | ensures_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify 0x42::failed_finalization_reports::rejected

run_cmd do
  let env ← getEnv
  for name in [`«0x42».failed_finalization_reports.rejected.typedVerified,
      `«0x42».failed_finalization_reports.rejected.verified] do
    if env.contains name then throwError "rejected proof published {name}"

verify 0x42::failed_finalization_reports::recovered by lean =>
  first
  | (run_tac logError "discarded authored diagnostic"; fail "backtrack")
  | (verex_obligations; all_goals verex_decide)

run_cmd do
  unless (← getEnv).contains `«0x42».failed_finalization_reports.recovered.verified do
    throwError "recovered authored proof did not publish its verified artifact"
