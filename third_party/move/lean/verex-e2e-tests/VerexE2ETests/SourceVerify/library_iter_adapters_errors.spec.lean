-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec rev_range_sum where
  requires n <= 1000
  ensures 2 * result == n * (n + 1)
  loop 1 where
    invariant iter.iter.start == 0 && iter.iter.end <= n
    invariant total <= (n - iter.iter.end) * 1000
    invariant 2 * total == n * (n - 1) - iter.iter.end * (iter.iter.end - 1)

spec rev_first where
  ensures n > 0 ==> result == 0
  ensures n == 0 ==> result == 0

spec rev_inclusive_count where
  ensures result == n
  loop 1 where
    invariant iter.iter.start == 0
    invariant !iter.iter.exhausted ==> iter.iter.end <= n && total == n - iter.iter.end
    invariant iter.iter.exhausted ==> total == n + 1

spec rev_slice_count where
  ensures result == values.length
  loop 1 where
    invariant iter.iter.pos == 0 && iter.iter.slice == values && iter.iter.end == values.length
    invariant n == values.length - iter.iter.end

spec rev_slice_first where
  ensures values.length > 0 ==> result == values[0]
  ensures values.length == 0 ==> result == default

spec enumerate_sum where
  requires values.length <= 1000
  ensures result <= values.length * 1000
  loop 1 where
    invariant iter.iter.slice == values && iter.iter.end == values.length && iter.iter.pos <= iter.iter.end
    invariant total <= iter.count * 1000

spec position where
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

spec find_entry where
  ensures result <= entries.length
  ensures result < entries.length ==> entries[result].value == key
  ensures ∀ (k in 0 .. result), entries[k].key != key
  loop 1 where
    invariant iter.iter.slice == entries && iter.iter.end == entries.length && iter.iter.pos <= iter.iter.end
    invariant iter.count == iter.iter.pos
    invariant ∀ (k in 0 .. iter.iter.pos), entries[k].key != key

spec last_rank where
  ensures values.length > 0 ==> result == values.length - 1

spec last_of_enumerated where
  ensures values.length > 0 ==> result == 0

-- The search runs from the back, so the match is the last one, not the first.
spec last_position where
  ensures result < values.length ==> ∀ (k in 0 .. result), values[k] != target
  loop 1 where
    invariant iter.iter.count == 0 && iter.iter.iter.pos == 0 && iter.iter.iter.slice == values
    invariant iter.iter.iter.end <= values.length
    invariant ∀ (k in iter.iter.iter.end .. values.length), values[k] != target

spec rev_contains where
  ensures result == contains(values, target)
  loop 1 where
    invariant iter.iter.pos == 0 && iter.iter.slice == values
    invariant iter.iter.end <= values.length
    invariant ∀ (k in 0 .. iter.iter.end), values[k] != target

spec rev_ranks where
  ensures result == values.length + 1
  loop 1 where
    invariant iter.iter.iter.pos == 0 && iter.iter.iter.slice == values
    invariant iter.iter.iter.end <= values.length
    invariant iter.count == values.length - iter.iter.iter.end
    invariant total == iter.count

spec enumerate_inclusive where
  requires n < 4294967295
  ensures result == n
  loop 1 where
    invariant iter.iter.end == n && !iter.iter.exhausted && iter.iter.start <= n + 1
    invariant iter.count == iter.iter.start
    invariant total == iter.count
