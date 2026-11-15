-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec power where
  ensures result == u32::pow(base, exponent) -- error: it panics when it overflows

spec signed_power where
  panics_if i8::pow(base, exponent) > MAX_I8 -- error: it also overflows below

spec ones where
  ensures result == u8::count_ones(value) + 1 -- error: one more than it has

spec signed_ones where
  ensures result == u8::count_ones(value) -- error: a negative value is its two's complement

spec sorted where
  ensures values == old(values) -- error: it reorders them
