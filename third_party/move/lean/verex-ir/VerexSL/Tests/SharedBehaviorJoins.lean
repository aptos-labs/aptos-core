-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-! A joined closure keeps the behavior of the selected implementation.
The invocation's premises can already be in the context when the entry
relation is opened. They must be linked before deriving its contract. -/

set_option maxHeartbeats 180000
set_option verex.verifyHeartbeats 180000
set_option verex.sharedContinuations true

/--
error: the specification clause `ensures result == x + 1` is not established
---
error: verex verification failed: the automatic verification of `wrong` failed; provide a proof in the module:
  verify wrong by lean =>
    verex_obligations
    verex_on
    | ensures_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verex module 0x42::shared_behavior_joins where
  fun add(x : u64, y : u64) -> u64 := x + y
  spec add where
    aborts_if x + y > 18446744073709551615
    ensures result == x + y

  fun mul(x : u64, y : u64) -> u64 := x * y
  spec mul where
    aborts_if x * y > 18446744073709551615
    ensures result == x * y

  struct Op has Copy, Drop where
    f : Fn(u64) -> u64 has Copy, Drop

  fun choose(flag : Bool, x : u64) -> u64 := do
    let op := if flag then
        new Op { f := function[Fn(u64) -> u64 has Copy, Drop](add, _, 1) }
      else new Op { f := function[Fn(u64) -> u64 has Copy, Drop](mul, _, 2) }
    invoke(op.f, x)
  spec choose where
    pragma opaque
    aborts_if flag && x + 1 > 18446744073709551615
    aborts_if !flag && x * 2 > 18446744073709551615
    ensures flag ==> result == x + 1
    ensures !flag ==> result == x * 2

  -- A caller may use only the behavior proved for its selected branch.
  fun wrong(flag : Bool, x : u64) -> u64 := choose(flag, x)
  spec wrong where
    aborts_if flag && x + 1 > 18446744073709551615
    aborts_if !flag && x * 2 > 18446744073709551615
    ensures result == x + 1
