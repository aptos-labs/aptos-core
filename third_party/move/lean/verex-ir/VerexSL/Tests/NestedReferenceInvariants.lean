-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

set_option maxHeartbeats 180000
set_option verex.verifyHeartbeats 180000

verex namespace nested_reference_invariants where
  pragma verify = false
  struct Bounded where
    value : u64
  spec Bounded where
    invariant value < 10

  fun read(value : &mut &mut Bounded) -> u64 := (**value).value
  spec read where
    ensures result < 10
    panics_if false
  verify read

  fun set_valid(value : &mut &mut Bounded) -> Unit := do
    (**value).value := 7u64
  spec set_valid where
    ensures value.value == 7
    panics_if false
  verify set_valid

  fun set_invalid(value : &mut &mut Bounded) -> Unit := do
    (**value).value := 11u64
  spec set_invalid where
    panics_if false

/--
error: the specification clause `invariant value < 10` is not established
---
error: verex verification failed: the automatic verification of `set_invalid` failed; provide a proof in the module:
  verify set_invalid by lean =>
    verex_obligations
    verex_on
    | data_invariant_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify nested_reference_invariants::set_invalid
