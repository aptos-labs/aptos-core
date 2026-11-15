-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec head where
  ensures values.length > 0 ==> result is Some && result.0 == values[0]
  ensures values.length == 0 ==> result is None

spec tail where
  ensures values.length > 0 ==> result is Some && result.0 == values[values.length - 1]
  ensures values.length == 0 ==> result is None

spec parts where
  requires values.length <= 1000
  panics_if mid > values.length
  ensures result == mid + 2 * (values.length - mid)

spec flip where
  ensures values.length == old(values).length
  ensures ∀ (k in 0 .. values.length), values[k] == old(values)[old(values).length - 1 - k]

spec has where
  ensures result == (∃ (k in 0 .. values.length), values[k] == value)
  ensures values.length == 0 ==> !result

spec vec_head where
  ensures values.length > 0 ==> result is Some && result.0 == values[0]
