-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

set_option maxHeartbeats 25000
set_option leaner.verifyHeartbeats 25000

leaner module 0x42::certified_reads where
  fun unsigned_bounds(values : &Vector<u8>, index : u64) -> Unit := ()
  spec unsigned_bounds where
    ensures 0 <= values[index] && values[index] <= 255
  verify unsigned_bounds

  fun signed_bounds(values : &Vector<i8>, index : u64) -> Unit := ()
  spec signed_bounds where
    ensures -128 <= values[index] && values[index] <= 127
  verify signed_bounds

  fun missing_is_zero() -> u64 := 0
  spec missing_is_zero where
    ensures vector<i8>[][17] == result
  verify missing_is_zero

  fun signed_negative() -> i8 := -1
  spec signed_negative where
    ensures vector<i8>[-1][0] == result
    ensures result < 0
  verify signed_negative

  fun wrong_nonnegative(values : &Vector<i8>, index : u64) -> Unit := ()
  spec wrong_nonnegative where
    pragma verify = false
    requires index < values.length
    ensures values[index] >= 0

/--
error: the specification clause `ensures values[index] >= 0` is not established
---
error: leaner verification failed: the automatic verification of `wrong_nonnegative` failed; provide a proof: `verify wrong_nonnegative by …` in the module (`verify wrong_nonnegative by skip` shows the obligations it leaves)
-/
#guard_msgs in
verify 0x42::certified_reads::wrong_nonnegative
