-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-! A join shares its suffix among all arrivals, whatever constructors and
literals their values, slots and memory hold: a position where they differ is
abstracted, and the entry relation states the disjunction of what they bring.
A single arrival resumes as a single predecessor, with its concrete state and
no summary, as the default walk would. -/

set_option Elab.async false

open Lean Elab Command in
elab "#guard_joins " expected:str " in " command:command : command => do
  let environment ← getEnv
  let (output, ()) ← IO.FS.withIsolatedStreams (isolateStderr := false) <|
    elabCommand (← `(command|
      set_option verex.sharedContinuations true in
      set_option verex.solverProfile true in $command))
  setEnv environment
  let lines := (output.splitOn "\n").filter (·.startsWith "  joins:")
  unless lines == ["  joins: " ++ expected.getString] do
    throwError "unexpected shared join profile: {lines}"

verex module 0x42::shared_classes where
  pragma verify = false

  -- Each branch borrows a different local: the slots of the two arrivals
  -- disagree on their constructor, and they still share the suffix.
  fun disagreeing(b : Bool, x : &mut u64, y : &mut u64) -> Unit := do
    if b then
      let first := x
      *first := 1
    else
      let second := y
      *second := 2
  spec disagreeing where
    aborts_if false
    ensures b ==> x == 1 && y == old(y)
    ensures !b ==> x == old(x) && y == 2

  -- Two branches assign a parameter and the third a literal; all three
  -- share one summary.
  fun mixed(k : u8, a : u64, b : u64) -> u64 := do
    let mut x : u64 := 0
    if k == 0 then x := a
    else if k == 1 then x := b
    else x := 7
    x + 1
  spec mixed where
    requires a < 100 && b < 100
    aborts_if false
    ensures result == (if k == 0 then a else if k == 1 then b else 7) + 1

-- A candidate whose suffix holds no program step is not folded
-- (`carriesProgram`).
#guard_joins "nodes 1, edges 2, summaries 1, proofs 1" in
verify 0x42::shared_classes::disagreeing

-- The join's three arrivals (`a`, `b`, `7`) are one class; a candidate whose
-- suffix holds no program step is not folded (`carriesProgram`).
#guard_joins "nodes 1, edges 3, summaries 1, proofs 1" in
verify 0x42::shared_classes::mixed
