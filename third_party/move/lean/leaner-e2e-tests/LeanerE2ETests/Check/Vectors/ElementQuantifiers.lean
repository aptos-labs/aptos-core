-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

set_option maxHeartbeats 25000
set_option leaner.verifyHeartbeats 25000

leaner module 0x42::element_quantifiers where
  fun first_by_value(v : Vector<u64>) -> u64 := v[0]
  spec first_by_value where
    requires v.length > 0
    requires forall (x in v), x < 10
    aborts_if false
    ensures result < 10
  verify first_by_value

  fun first_by_reference(v : &Vector<u64>) -> u64 := v[0]
  spec first_by_reference where
    requires v.length > 0
    requires forall (x in v), x < 10
    aborts_if false
    ensures result < 10
  verify first_by_reference

  fun signed_at(v : &Vector<i8>, index : u64) -> i8 := v[index]
  spec signed_at where
    requires index < v.length
    requires forall (x in v), x >= -5
    aborts_if false
    ensures result >= -5
  verify signed_at

  fun first_bool(v : &Vector<Bool>) -> Bool := v[0]
  spec first_bool where
    requires v.length > 0
    requires forall (x in v), x
    aborts_if false
    ensures result
  verify first_bool

  fun generic_at {T has Copy, Drop, Store}(v : &Vector<T>, index : u64, expected : T) -> T := v[index]
  spec generic_at where
    requires index < v.length
    requires forall (x in v), x == expected
    aborts_if false
    ensures result == expected
  verify generic_at

  fun too_strong(v : &Vector<u64>) -> u64 := v[0]
  spec too_strong where
    pragma verify = false
    requires v.length > 0
    requires forall (x in v), x < 10
    ensures result < 5

  fun missing_entry(v : &Vector<u64>) -> Unit := ()
  spec missing_entry where
    pragma verify = false
    requires forall (x in v), x > 0
    ensures v[0] > 0

/--
error: the specification clause `ensures result < 5` is not established
---
error: leaner verification failed: the automatic verification of `too_strong` failed; provide a proof: `verify too_strong by …` in the module (`verify too_strong by skip` shows the obligations it leaves)
-/
#guard_msgs in
verify 0x42::element_quantifiers::too_strong

/--
error: the specification clause `ensures v[0] > 0` is not established
---
error: leaner verification failed: the automatic verification of `missing_entry` failed; provide a proof: `verify missing_entry by …` in the module (`verify missing_entry by skip` shows the obligations it leaves)
-/
#guard_msgs in
verify 0x42::element_quantifiers::missing_entry
