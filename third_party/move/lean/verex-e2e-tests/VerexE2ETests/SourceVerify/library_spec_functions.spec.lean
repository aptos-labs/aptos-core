-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec power where
  panics_if u32::pow(base, exponent) > MAX_U32
  ensures result == u32::pow(base, exponent)
  ensures exponent == 0 ==> result == 1

spec signed_power where
  panics_if i8::pow(base, exponent) < MIN_I8 || i8::pow(base, exponent) > MAX_I8
  ensures result == i8::pow(base, exponent)

spec ones where
  ensures result == u8::count_ones(value)

spec signed_ones where
  ensures result == i8::count_ones(value)

spec sorted where
  ensures values.length == old(values).length
  ensures ∀ (x : u32), slice::count(values, x) == slice::count(old(values), x)
  ensures ∀ (i in 0 .. values.length; j in 0 .. values.length), i <= j ==> values[i] <= values[j]
