-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec enumerate_sum where
  requires values.length <= 1000
  ensures result <= values.length * 1000
  loop 1 where
    invariant iter.iter.slice == values && iter.iter.end == values.length && iter.iter.pos <= iter.iter.end
    invariant total <= iter.count * 1000

spec position where
  pragma heartbeats = 480
  ensures result <= values.length
  ensures result < values.length ==> values[result + 1] == target
  ensures result == values.length ==> !contains(values, target)
  loop 1 where
    invariant iter.iter.slice == values && iter.iter.end == values.length && iter.iter.pos <= iter.iter.end
    invariant iter.count == iter.iter.pos
    invariant ∀ (k in 0 .. iter.iter.pos), values[k] != target

spec enumerate_range where
  requires n <= 1000
  ensures result < n * 1000
  loop 1 where
    invariant iter.iter.start <= iter.iter.end && iter.iter.end == n
    invariant iter.count == iter.iter.start
    invariant total <= iter.count * 1000

spec count_steps where
  ensures result == n / 2
  loop 1 where
    invariant iter.iter.end + total == (n + 1) / 2
    invariant iter.step_minus_one == 1

spec first_step where
  ensures n > 0 ==> result == 1

-- A step of zero panics, which the clause does not permit.
spec no_step where
  ensures result == n

spec enumerate_inclusive where
  requires n < 4294967295
  ensures result == n
  loop 1 where
    invariant iter.iter.end == n && !iter.iter.exhausted && iter.iter.start <= n + 1
    invariant iter.count == iter.iter.start
    invariant total == iter.count
