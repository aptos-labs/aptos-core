-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-! A join shares its suffix only among arrivals agreeing on the ground
skeleton of their arguments: the constructors and literals of their values,
slots and memory. Arrivals of a class abstract only positions that are
ground in none of them. A class of one arrival resumes as a single
predecessor, with its concrete state and no summary, as the default walk
would. -/

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
  -- disagree on their constructor, so each walks the suffix alone.
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

  -- Two of three branches assign a parameter, agreeing on the skeleton,
  -- and share one summary; the third assigns a literal and walks alone.
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

-- Each join's two arrivals are two classes of one: no summary, no proof.
-- A candidate whose suffix holds no program step is not folded
-- (`carriesProgram`).
#guard_joins "nodes 1, edges 2, summaries 0, proofs 0" in
verify 0x42::shared_classes::disagreeing

-- The join's three arrivals (`a`, `b`, `7`) are a class of two and one of
-- one; a candidate whose suffix holds no program step is not folded
-- (`carriesProgram`).
#guard_joins "nodes 1, edges 3, summaries 1, proofs 1" in
verify 0x42::shared_classes::mixed
