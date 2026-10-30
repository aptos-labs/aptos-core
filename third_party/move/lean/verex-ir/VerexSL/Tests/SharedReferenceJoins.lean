-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-! Joined reference carriers retain their lender correlations. Opening an
entry must link its tuple and constructor equalities before general search;
returned loans also retain their destinations through a loop break. -/

set_option maxHeartbeats 180000
set_option verex.verifyHeartbeats 180000
set_option verex.sharedContinuations true

verex module 0x42::shared_reference_joins where
  fun choose_three(takeFirst : Bool, takeSecond : Bool, first : &mut u64,
      second : &mut u64, third : &mut u64) -> &mut u64 :=
    if takeFirst then first
    else if takeSecond then second else third
  spec choose_three where
    ensures result == (if takeFirst then old(first)
        else if takeSecond then old(second) else old(third)) &&
      first == old(first) && second == old(second) && third == old(third)
    aborts_if false

  fun write_chosen(takeFirst : Bool, takeSecond : Bool, first : &mut u64,
      second : &mut u64, third : &mut u64) -> Unit := do
    let returned := choose_three(takeFirst, takeSecond, first, second, third)
    *returned := 31
  spec write_chosen where
    ensures if takeFirst then
        (first == 31 && second == old(second) && third == old(third))
      else if takeSecond then
        (first == old(first) && second == 31 && third == old(third))
      else (first == old(first) && second == old(second) && third == 31)
    aborts_if false

  fun return_pair(left : &mut u64, right : &mut u64) -> (&mut u64, &mut u64) :=
    (left, right)
  spec return_pair where
    ensures spec.result[0] == old(left) && spec.result[1] == old(right) &&
      left == old(left) && right == old(right)
    aborts_if false

  fun break_pair(left : &mut u64, right : &mut u64, stop : Bool) -> Unit := do
    let mut i : u64 := 0
    while i < 2 do
      let (returnedLeft, returnedRight) := return_pair(left, right)
      if stop then break
      *returnedLeft := 7
      *returnedRight := 8
      i := i + 1
    where
      invariant i <= 2
    *left := 10
    *right := 11
  spec break_pair where
    ensures left == 10 && right == 11
    aborts_if false

  fun wrong(takeFirst : Bool, takeSecond : Bool, first : &mut u64,
      second : &mut u64, third : &mut u64) -> Unit := do
    let returned := choose_three(takeFirst, takeSecond, first, second, third)
    *returned := 31
  spec wrong where
    pragma verify = false
    ensures first == old(first) && second == old(second) && third == old(third)
    aborts_if false

/--
error: the specification clause `ensures first == old(first) && second == old(second) && third == old(third)` is not established
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
verify 0x42::shared_reference_joins::wrong
