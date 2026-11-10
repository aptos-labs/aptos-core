-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec magnitude where
  ensures result >= 0 -- error: it panics at the minimum

spec quotient where
  ensures result is Some -- error: not by zero

spec signed_quotient where
  ensures left == -7 && right == 2 ==> result is Some && result.0 == -4 -- error: it truncates

spec wrapped where
  ensures result == left % right -- error: it panics by zero

spec signed_wrapped where
  panics_if right == 0 || (left == MIN_I32 && right == -1)
  ensures left == -7 && right == 2 ==> result == -1 -- error: it is never negative

spec single_bit where
  ensures result == (∃ (k in 1 .. 8), value == (1 << k)) -- error: one is a power of two

spec high_zeros where
  ensures value == 0 ==> result == 7 -- error: all eight are

spec low_zeros where
  ensures value == 0 ==> result == 7 -- error: all eight are
