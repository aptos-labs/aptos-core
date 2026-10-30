-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-! State-label witnesses retain the values and memories of incoming edges.
A source clause still gets one automatic allowance across its alternatives. -/

set_option Elab.async false

open Lean Elab Command in
elab "#guard_label_summary " certified:num " in " command:command : command => do
  let environment ← getEnv
  let (output, ()) ← IO.FS.withIsolatedStreams (isolateStderr := false) <|
    elabCommand (← `(command|
      set_option verex.sharedContinuations true in
      set_option verex.solverProfile true in $command))
  setEnv environment
  let lines := (output.splitOn "\n").filter (·.startsWith "  joins:")
  unless lines.length == 1 &&
      lines.all (·.endsWith s!"summaries 1, proofs {certified.getNat}") do
    throwError "unexpected state-label summary profile: {lines}"

verex module 0x42::shared_labels where
  pragma verify = false
  struct Counter has Copy, Drop, Store where
    value : u64
  spec fun increases(old_c : Counter, c : Counter) : Bool := old_c.value < c.value

  fun twice(c : &mut Counter, b : Bool) -> Unit := do
    if b then c.value := c.value + 1 else c.value := c.value + 2
    c.value := c.value + 1
  spec twice where
    requires c.value + 3 < MAX_U64
    aborts_if false
    ensures ∃ (S : StateDomain),
      ((..S |~ increases(old(c), c))) && (S.. |~ increases(old(c), c))

  fun once(c : &mut Counter, b : Bool) -> Unit := do
    if b then c.value := c.value + 1 else c.value := c.value + 1
  spec once where
    requires c.value + 3 < MAX_U64
    aborts_if false
    ensures ∃ (S : StateDomain),
      ((..S |~ increases(old(c), c))) && (S.. |~ increases(old(c), c))

-- The witness is the counter after either branch and before the final update.
#guard_label_summary 1 in
verify 0x42::shared_labels::twice

-- One increment cannot contain two strict increases. A rejected summary
-- remains a verification failure and never counts as a certified proof.
/--
error: the specification clause `ensures ∃ (S : StateDomain),
      ((..S |~ increases(old(c), c))) && (S.. |~ increases(old(c), c))` is not established
---
error: verex verification failed: the automatic verification of `once` failed; provide a proof in the module:
  verify once by
    verex_obligations
    verex_on
    | ensures_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
#guard_label_summary 0 in
verify 0x42::shared_labels::once
