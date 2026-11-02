-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexE2ETests.CheckSupport

set_option maxHeartbeats 25000
set_option verex.verifyHeartbeats 25000

verex module 0x42::vector_ranges where
  fun length_of {T has Drop}(values : &Vector<T>) -> u64 := values.length
  spec length_of where
    aborts_if false
    ensures ∀ (index in range(values)), 0 <= index && index < result
  verify length_of

  fun nonempty(values : &Vector<u64>) -> Unit := ()
  spec nonempty where
    requires values.length > 0
    ensures ∃ (index in range(values)), index == 0
  verify nonempty

  fun empty() -> Vector<u64> := vector<u64>[]
  spec empty where
    ensures ∀ (index in range(result)), false
  verify empty

  fun clear(values : &mut Vector<u64>) -> Unit := do
    *values := vector<u64>[]
  spec clear where
    requires values.length > 0
    ensures ∃ (index in range(old(values))), index == 0
    ensures ∀ (index in range(values)), false
  verify clear

  fun literal() -> Vector<u64> := vector<u64>[7, 9]
  spec literal where
    ensures spec.sliceVector(result, range(result)) == result
    ensures ∀ (index in range(result)), result[index] == 7 || result[index] == 9
  verify literal

  fun wrong_upper(values : &Vector<u64>) -> Unit := ()
  spec wrong_upper where
    pragma verify = false
    ensures ∃ (index in range(values)), index == values.length

/--
error: the specification clause `ensures ∃ (index in range(values)), index == values.length` is not established
---
error: verex verification failed: the automatic verification of `wrong_upper` failed; provide a proof in the module:
  verify wrong_upper by lean =>
    verex_obligations
    verex_on
    | ensures_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify 0x42::vector_ranges::wrong_upper
