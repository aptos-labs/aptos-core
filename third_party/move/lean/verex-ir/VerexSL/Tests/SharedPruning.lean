-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-! Infeasible incoming branches need checked proofs. Their preparation
costs, including failed checks, belong to every downstream source clause. -/

set_option Elab.async false

open Lean Elab Command in
elab "#guard_pruning " checks:num " removed " removed:num " owners " owners:num
    " in " command:command : command => do
  let environment ← getEnv
  let (output, ()) ← IO.FS.withIsolatedStreams (isolateStderr := false) <|
    elabCommand (← `(command|
      set_option verex.sharedContinuations true in
      set_option verex.solverProfile true in $command))
  setEnv environment
  let lines := (output.splitOn "\n").filter (·.startsWith "  join pruning:")
  let expected := s!"  join pruning: checks {checks.getNat}, pruned {removed.getNat}, debited owners {owners.getNat}"
  unless lines == [expected] do
    throwError "unexpected pruning or debit ownership: {lines}"

verex module 0x42::shared_pruning where
  pragma verify = false

  fun bounded(n : u64) -> u64 := do
    let mut x : u64 := 0
    if n < 20 then x := n + 1 else x := n + 2
    x
  spec bounded where
    requires n < 10
    aborts_if false
    ensures result == n + 1
    ensures result < 12

  fun followed(n : u64, b : Bool) -> u64 := do
    let mut x : u64 := 0
    if n < 20 then x := n + 1 else x := n + 2
    if b then x := x + 3 else x := x + 4
    x
  spec followed where
    requires n < 10
    aborts_if false
    ensures result == (if b then n + 4 else n + 5)
    ensures result < 15

  fun nested(n : u64, b : Bool) -> u64 := do
    let mut x : u64 := 0
    if b then
      if n < 20 then x := 1 else x := 2
    else
      if n < 30 then x := 3 else x := 4
    x
  spec nested where
    requires n < 10
    aborts_if false
    ensures result == (if b then 1 else 3)
    ensures result < 4

  fun wrong(b : Bool) -> u64 := do
    let mut x : u64 := 0
    if b then x := 1 else x := 2
    x
  spec wrong where
    aborts_if false
    ensures result == 1

#guard_pruning 2 removed 1 owners 2 in
verify 0x42::shared_pruning::bounded

#guard_pruning 4 removed 1 owners 4 in
verify 0x42::shared_pruning::followed

#guard_pruning 5 removed 2 owners 2 in
verify 0x42::shared_pruning::nested

/--
error: the specification clause `ensures result == 1` is not established
---
error: verex verification failed: the automatic verification of `wrong` failed; provide a proof in the module:
  verify wrong by
    verex_obligations
    verex_on
    | ensures_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
#guard_pruning 2 removed 0 owners 1 in
verify 0x42::shared_pruning::wrong
