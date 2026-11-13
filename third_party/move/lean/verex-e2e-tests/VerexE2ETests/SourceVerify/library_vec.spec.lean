-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec push_two where
  ensures result.length == 2
  ensures result[0] == 1 && result[1] == 2

spec length where
  ensures result == values.length

spec push_then_length where
  ensures result == old(values).length + 1
  ensures values[values.length - 1] == value

spec empty where
  ensures result == (values.length == 0)

spec pop_one where
  ensures old(values).length == 0 ==> result is None && values == old(values)
  ensures old(values).length > 0 ==> result is Some
  ensures old(values).length > 0 ==> result.0 == old(values)[old(values).length - 1]
  ensures old(values).length > 0 ==> values.length == old(values).length - 1

spec pop_after_push where
  ensures result == value
  ensures values == old(values)
  ensures values.length == old(values).length

spec pop_from_empty where
  ensures result

spec pass where
  ensures result == values

spec dropped where
  ensures result == 1

spec as_slice where
  ensures result == values

spec slice_length where
  ensures result == values.length
