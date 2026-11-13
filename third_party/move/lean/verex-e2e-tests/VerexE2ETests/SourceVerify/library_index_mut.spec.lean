-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec store where
  panics_if position >= old(values).length
  ensures values == update(old(values), position, value)

spec bump where
  requires position < values.length
  requires values[position] < MAX_U32
  ensures values[position] == old(values)[position] + 1
  ensures values.length == old(values).length

spec swap_first where
  requires values.length > 0
  ensures result == old(values)[0]
  ensures values[0] == value

spec fill where
  ensures values.length == old(values).length
  ensures ∀ (k in 0 .. values.length), values[k] == value
  loop 1 where
    invariant iter.start <= iter.end && iter.end == values.length
    invariant values.length == old(values).length
    invariant ∀ (k in 0 .. iter.start), values[k] == value
