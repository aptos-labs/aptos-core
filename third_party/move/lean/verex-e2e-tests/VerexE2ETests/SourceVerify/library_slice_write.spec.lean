-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec fill where
  ensures values.length == old(values).length
  ensures ∀ (k in 0 .. values.length), values[k] == value
  loop 1 where
    invariant iter.start <= iter.end && iter.end == values.length
    invariant values.length == old(values).length
    invariant ∀ (k in 0 .. iter.start), values[k] == value

spec store where
  panics_if position >= old(values).length
  ensures values == update(old(values), position, value)

spec set_first where
  ensures result == old(values).length
  ensures result > 0 ==> values[0] == value
