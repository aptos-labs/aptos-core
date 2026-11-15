-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec larger where
  ensures result == a

spec smaller where
  ensures result == b

spec narrow where
  ensures result == value

spec repeated where
  ensures result.length == length
  ensures ∀ (k in 0 .. length), !result[k]

spec take where
  ensures result == old(values)[position]

spec keep_order where
  panics_if position >= old(values).length
  ensures ∀ (k in position .. values.length), values[k] == old(values)[k + 1]

spec exchange where
  panics_if first >= old(values).length || second >= old(values).length
  ensures values == old(values)

spec clear where
  ensures ∀ (k in 0 .. values.length), values[k]
