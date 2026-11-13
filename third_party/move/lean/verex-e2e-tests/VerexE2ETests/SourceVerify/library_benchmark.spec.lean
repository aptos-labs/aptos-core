-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec order_u8 where
  ensures spec.result[0] >= a && spec.result[0] >= b
  ensures spec.result[1] <= a && spec.result[1] <= b
  ensures spec.result[0] == a || spec.result[0] == b
  ensures spec.result[1] == a || spec.result[1] == b

spec order_u16 where
  ensures spec.result[0] >= a && spec.result[0] >= b
  ensures spec.result[1] <= a && spec.result[1] <= b
  ensures spec.result[0] == a || spec.result[0] == b
  ensures spec.result[1] == a || spec.result[1] == b

spec order_u32 where
  ensures spec.result[0] >= a && spec.result[0] >= b
  ensures spec.result[1] <= a && spec.result[1] <= b
  ensures spec.result[0] == a || spec.result[0] == b
  ensures spec.result[1] == a || spec.result[1] == b

spec order_u64 where
  ensures spec.result[0] >= a && spec.result[0] >= b
  ensures spec.result[1] <= a && spec.result[1] <= b
  ensures spec.result[0] == a || spec.result[0] == b
  ensures spec.result[1] == a || spec.result[1] == b

spec order_u128 where
  ensures spec.result[0] >= a && spec.result[0] >= b
  ensures spec.result[1] <= a && spec.result[1] <= b
  ensures spec.result[0] == a || spec.result[0] == b
  ensures spec.result[1] == a || spec.result[1] == b

spec order_usize where
  ensures spec.result[0] >= a && spec.result[0] >= b
  ensures spec.result[1] <= a && spec.result[1] <= b
  ensures spec.result[0] == a || spec.result[0] == b
  ensures spec.result[1] == a || spec.result[1] == b

spec order_i8 where
  ensures spec.result[0] >= a && spec.result[0] >= b
  ensures spec.result[1] <= a && spec.result[1] <= b
  ensures spec.result[0] == a || spec.result[0] == b
  ensures spec.result[1] == a || spec.result[1] == b

spec order_i16 where
  ensures spec.result[0] >= a && spec.result[0] >= b
  ensures spec.result[1] <= a && spec.result[1] <= b
  ensures spec.result[0] == a || spec.result[0] == b
  ensures spec.result[1] == a || spec.result[1] == b

spec order_i32 where
  ensures spec.result[0] >= a && spec.result[0] >= b
  ensures spec.result[1] <= a && spec.result[1] <= b
  ensures spec.result[0] == a || spec.result[0] == b
  ensures spec.result[1] == a || spec.result[1] == b

spec order_i64 where
  ensures spec.result[0] >= a && spec.result[0] >= b
  ensures spec.result[1] <= a && spec.result[1] <= b
  ensures spec.result[0] == a || spec.result[0] == b
  ensures spec.result[1] == a || spec.result[1] == b

spec order_i128 where
  ensures spec.result[0] >= a && spec.result[0] >= b
  ensures spec.result[1] <= a && spec.result[1] <= b
  ensures spec.result[0] == a || spec.result[0] == b
  ensures spec.result[1] == a || spec.result[1] == b

spec order_isize where
  ensures spec.result[0] >= a && spec.result[0] >= b
  ensures spec.result[1] <= a && spec.result[1] <= b
  ensures spec.result[0] == a || spec.result[0] == b
  ensures spec.result[1] == a || spec.result[1] == b

spec narrow where
  panics_if value > MAX_U64
  ensures result == value

spec widen where
  ensures result == value

spec repeated where
  ensures result.length == length
  ensures ∀ (k in 0 .. length), result[k]

spec reserved where
  ensures result.length == 0

spec take where
  panics_if position >= old(values).length
  ensures result == old(values)[position]
  ensures values.length == old(values).length - 1
  ensures ∀ (k in 0 .. values.length), k != position ==> values[k] == old(values)[k]
  ensures position < values.length ==> values[position] == old(values)[old(values).length - 1]

spec none where
  ensures result == (values.length == 0)

spec exchange where
  panics_if first >= old(values).length || second >= old(values).length
  ensures values[first] == old(values)[second]
  ensures values[second] == old(values)[first]
  ensures values.length == old(values).length

spec clear where
  ensures values.length == old(values).length
  ensures ∀ (k in 0 .. values.length), !values[k]
