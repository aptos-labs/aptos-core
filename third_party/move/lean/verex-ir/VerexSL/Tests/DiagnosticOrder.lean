-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-! Automatic reports and suggested alternatives follow source clauses even
when traversal encounters the abort first. Replay both automatic schedulers;
the branch join must not affect the order or duplicate a failed clause. -/

set_option Elab.async false

verex module 0x42::diagnostic_order where
  pragma verify = false

  fun wrong(b : Bool, stop : Bool) -> u64 := do
    let mut x : u64 := 0
    if b then x := 1 else x := 2
    if stop then abort(7)
    x
  spec wrong where
    ensures result == 3
    aborts_if false

/--
error: the specification clause `ensures result == 3` is not established
---
error: the specification clause `aborts_if false` is not established
---
error: verex verification failed: the automatic verification of `wrong` failed; provide a proof in the module:
  verify wrong by lean =>
    verex_obligations
    verex_on
    | ensures_1 => verex_prepare; sorry
    | aborts_if_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
set_option verex.sharedContinuations false in
verify 0x42::diagnostic_order::wrong

/--
error: the specification clause `ensures result == 3` is not established
---
error: the specification clause `aborts_if false` is not established
---
error: verex verification failed: the automatic verification of `wrong` failed; provide a proof in the module:
  verify wrong by lean =>
    verex_obligations
    verex_on
    | ensures_1 => verex_prepare; sorry
    | aborts_if_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
set_option verex.sharedContinuations true in
verify 0x42::diagnostic_order::wrong
