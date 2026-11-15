-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec fill where
  ensures values.length == old(values).length
  ensures ∀ (k in 0 .. values.length), values[k] == value
  loop 1 where
    invariant iter.start <= iter.end && iter.end == values.length
    invariant values.length == old(values).length
    invariant ∀ (k in 0 .. iter.start + 1), values[k] == value

spec store where
  ensures values.length == old(values).length

spec set_first where
  ensures result == 0
