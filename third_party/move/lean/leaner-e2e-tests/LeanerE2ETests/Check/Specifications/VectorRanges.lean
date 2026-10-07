-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

set_option maxHeartbeats 25000
set_option leaner.verifyHeartbeats 25000

leaner module 0x42::vector_ranges where
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
error: leaner verification failed: the automatic verification of `wrong_upper` failed; provide a proof: `verify wrong_upper by …` in the module (`verify wrong_upper by skip` shows the obligations it leaves)
-/
#guard_msgs in
verify 0x42::vector_ranges::wrong_upper
