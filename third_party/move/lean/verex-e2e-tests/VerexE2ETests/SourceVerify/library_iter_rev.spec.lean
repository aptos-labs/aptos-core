-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec rev_range_sum where
  requires n <= 1000
  ensures 2 * result == n * (n - 1)
  loop 1 where
    invariant iter.iter.start == 0 && iter.iter.end <= n
    invariant total <= (n - iter.iter.end) * 1000
    invariant 2 * total == n * (n - 1) - iter.iter.end * (iter.iter.end - 1)

spec rev_first where
  ensures n > 0 ==> result == n - 1
  ensures n == 0 ==> result == 0

spec rev_inclusive_count where
  ensures result == n + 1
  loop 1 where
    invariant iter.iter.start == 0
    invariant !iter.iter.exhausted ==> iter.iter.end <= n && total == n - iter.iter.end
    invariant iter.iter.exhausted ==> total == n + 1

spec rev_slice_count where
  ensures result == values.length
  loop 1 where
    invariant iter.iter.pos == 0 && iter.iter.slice == values && iter.iter.end <= values.length
    invariant n == values.length - iter.iter.end

spec rev_slice_first where
  ensures values.length > 0 ==> result == values[values.length - 1]
  ensures values.length == 0 ==> result == default

spec rev_contains where
  ensures result == contains(values, target)
  loop 1 where
    invariant iter.iter.pos == 0 && iter.iter.slice == values
    invariant iter.iter.end <= values.length
    invariant ∀ (k in iter.iter.end .. values.length), values[k] != target
