-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec insert_at where
  panics_if at > old(values).length
  ensures values.length == old(values).length + 1
  ensures values[at] == value
  ensures ∀ (k in 0 .. at), values[k] == old(values)[k]
  ensures ∀ (k in at .. old(values).length), values[k + 1] == old(values)[k]

spec remove_at where
  panics_if at >= old(values).length
  ensures result == old(values)[at]
  ensures values.length == old(values).length - 1
  ensures ∀ (k in 0 .. at), values[k] == old(values)[k]
  ensures ∀ (k in at .. values.length), values[k] == old(values)[k + 1]

spec keep_two where
  ensures old(values).length <= 2 ==> values == old(values)
  ensures old(values).length > 2 ==> values.length == 2
  ensures old(values).length > 2 ==> values[1] == old(values)[1]

spec clear_all where
  ensures values.length == 0

spec whole where
  ensures result == values

spec append where
  ensures values.length == old(values).length + other.length
  ensures ∀ (k in 0 .. old(values).length), values[k] == old(values)[k]
  ensures ∀ (k in 0 .. other.length), values[old(values).length + k] == other[k]

spec filled where
  ensures result.length == count
  ensures ∀ (k in 0 .. count), result[k] == 7
