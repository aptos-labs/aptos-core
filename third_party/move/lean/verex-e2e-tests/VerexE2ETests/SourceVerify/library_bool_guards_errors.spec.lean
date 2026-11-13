-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec guarded where
  requires _flag ==> x == 1
  requires !_flag ==> x == 2
  ensures result >= 2

spec two_flags where
  requires _a ==> x == 1
  requires !_a && _b ==> x == 2
  requires !_a && !_b ==> x == 3
  ensures result >= 1 && result <= 2

spec count_inclusive where
  ensures result == n
  loop 1 where
    invariant iter.end == n
    invariant !iter.exhausted ==> iter.start <= n + 1 && total == iter.start
    invariant iter.exhausted ==> n == 4294967295 && total == n + 1
