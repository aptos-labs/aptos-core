-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-! Literal bounds from opaque callees still expose finite positions, including
large literals, strict bounds, generic instantiation and mutable results. -/

set_option maxHeartbeats 25000
set_option verex.verifyHeartbeats 25000

-- The constant-list shortcut must leave differing entries alone.
example (i : Nat) (h : i < 2) :
    ([7, 8] : List Nat)[i] = if i = 0 then 7 else 8 := by
  try simp only [vir_denote_norm]
  rcases (show i = 0 ∨ i = 1 by omega) with rfl | rfl <;> rfl

verex module 0x42::call_range_results where
  fun choose(flag : Bool) -> u64 := if flag then 0 else 1
  spec choose where
    pragma opaque
    aborts_if false
    ensures 0 <= result
    ensures result <= 1

  fun choose_large() -> u64 := 1000000000000
  spec choose_large where
    pragma opaque
    aborts_if false
    ensures 999999999999 < result
    ensures result < 1000000000001

  fun write_choice(slot : &mut u64, flag : Bool) -> Unit :=
    *slot := if flag then 0 else 1
  spec write_choice where
    pragma opaque
    aborts_if false
    ensures 0 <= *slot
    ensures *slot <= 1

  fun read_choice(flag : Bool) -> u64 := do
    let index := choose(flag)
    let values := vector<u64>[7, 7]
    values[index]
  spec read_choice where
    aborts_if false
    ensures result == 7

  fun read_large() -> u64 := do
    let index := choose_large()
    let values := vector<u64>[7]
    values[index - 1000000000000]
  spec read_large where
    aborts_if false
    ensures result == 7

  fun read_written(flag : Bool) -> u64 := do
    let mut index := 0u64
    write_choice(&mut index, flag)
    let values := vector<u64>[7, 7]
    values[index]
  spec read_written where
    aborts_if false
    ensures result == 7

  fun choose_generic {T}(flag : Bool) -> u64 := if flag then 0 else 1
  spec choose_generic where
    pragma opaque
    aborts_if false
    ensures 0 <= result
    ensures result <= 1

  fun read_generic(flag : Bool) -> u64 := do
    let index := choose_generic::<Bool>(flag)
    let values := vector<u64>[7, 7]
    values[index]
  spec read_generic where
    aborts_if false
    ensures result == 7

-- Computed positions retain the finite range through arithmetic temporaries.
verex module 0x42::computed_call_range_results where
  fun choose(flag : Bool) -> u64 := if flag then 10 else 11
  spec choose where
    pragma opaque
    aborts_if false
    ensures 10 <= result
    ensures result <= 11

  fun choose_large() -> u64 := 1000000000000
  spec choose_large where
    pragma opaque
    aborts_if false
    ensures 999999999999 < result
    ensures result < 1000000000001

  fun write_choice(slot : &mut u64, flag : Bool) -> Unit :=
    *slot := if flag then 10 else 11
  spec write_choice where
    pragma opaque
    aborts_if false
    ensures 10 <= *slot
    ensures *slot <= 11

  fun read_choice(flag : Bool) -> u64 := do
    let index := choose(flag)
    let values := vector<u64>[7, 7]
    values[index - 10]
  spec read_choice where
    aborts_if false
    ensures result == 7

  fun read_large() -> u64 := do
    let index := choose_large()
    let values := vector<u64>[7]
    values[index - 1000000000000]
  spec read_large where
    aborts_if false
    ensures result == 7

  fun read_written(flag : Bool) -> u64 := do
    let mut index := 0u64
    write_choice(&mut index, flag)
    let values := vector<u64>[7, 7]
    values[index - 10]
  spec read_written where
    aborts_if false
    ensures result == 7

  fun choose_generic {T}(flag : Bool) -> u64 := if flag then 10 else 11
  spec choose_generic where
    pragma opaque
    aborts_if false
    ensures 10 <= result
    ensures result <= 11

  fun read_generic(flag : Bool) -> u64 := do
    let index := choose_generic::<Bool>(flag)
    let values := vector<u64>[7, 7]
    values[index - 10]
  spec read_generic where
    aborts_if false
    ensures result == 7

  fun wrong_value(flag : Bool) -> u64 := read_choice(flag)
  spec wrong_value where
    pragma verify = false
    aborts_if false
    ensures result == 8

  fun repeated_choices(flag : Bool) -> u64 := do
    let first := choose(flag)
    let second := choose(flag)
    let total := first + second
    spec do
      assert total <= 22
      assert 20 <= total
    total
  spec repeated_choices where
    pragma verify = false
    aborts_if false
    ensures result <= 22

  fun wrong_index(flag : Bool) -> u64 := do
    let index := choose(flag)
    let values := vector<u64>[7, 7]
    values[index - 9]
  spec wrong_index where
    pragma verify = false
    aborts_if false

-- Earlier call results remain addressable when a later call uses the same callee.
-- Even a one-thousand-heartbeat automatic leaf cap does not constrain an
-- authored proof, including its explicitly requested cheap deciders.
set_option verex.leafHeartbeats 1 in
verify 0x42::computed_call_range_results::repeated_choices by
  verex_obligations
  verex_group range := assert_1, assert_2
  verex_on
  | range =>
    verex_prepare only
    have first_bound : old_choose_result.val <= 11 := by assumption
    have second_bound : choose_result.val <= 11 := by assumption
    constructor <;> omega
  | _ => first | verex_decide | (verex_prepare only; all_goals omega)

/--
error: the specification clause `ensures result == 8` is not established
---
error: verex verification failed: the automatic verification of `wrong_value` failed; provide a proof in the module:
  verify wrong_value by
    verex_obligations
    verex_on
    | ensures_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify 0x42::computed_call_range_results::wrong_value

/--
error: the specification clause `aborts_if false` is not established
---
error: verex verification failed: the automatic verification of `wrong_index` failed; provide a proof in the module:
  verify wrong_index by
    verex_obligations
    verex_on
    | aborts_if_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify 0x42::computed_call_range_results::wrong_index
