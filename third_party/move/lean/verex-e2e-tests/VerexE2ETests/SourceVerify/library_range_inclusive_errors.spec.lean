-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec count_inclusive where
  requires n < 4294967295
  ensures result == n
  loop 1 where
    invariant iter.end == n && !iter.exhausted
    invariant iter.start <= n + 1 && total == iter.start

-- The last step sets `exhausted`.
spec count_to_max where
  ensures result == 256 - n
  loop 1 where
    invariant iter.end == 255 && n <= 255 && !iter.exhausted
    invariant total == iter.start - n
    invariant n <= iter.start

spec sum_inclusive where
  requires n <= 1000
  ensures result == (n + 1) * 1000
  loop 1 where
    invariant iter.end == n && !iter.exhausted && iter.start <= n + 1
    invariant total <= iter.start * 1000

spec last_inclusive where
  requires a <= b
  ensures result + 1 == b
  loop 1 where
    invariant iter.end == b
    invariant iter.exhausted || (a <= iter.start && iter.start <= b + 1)
    invariant last == (if iter.exhausted then b else if iter.start == a then 0 else iter.start - 1)

-- One element when the bounds meet.
spec empty_inclusive where
  ensures result == (a >= b)

spec first_inclusive where
  ensures result == a
