-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec find_entry where
  pragma heartbeats = 730
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
  pragma heartbeats = 400
  ensures result < values.length ==> ∀ (k in 0 .. result), values[k] != target
  loop 1 where
    invariant iter.iter.count == 0 && iter.iter.iter.pos == 0 && iter.iter.iter.slice == values
    invariant iter.iter.iter.end <= values.length
    invariant ∀ (k in iter.iter.iter.end .. values.length), values[k] != target

spec rev_ranks where
  ensures result == values.length + 1
  loop 1 where
    invariant iter.iter.iter.pos == 0 && iter.iter.iter.slice == values
    invariant iter.iter.iter.end <= values.length
    invariant iter.count == values.length - iter.iter.iter.end
    invariant total == iter.count
