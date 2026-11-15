-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec magnitude where
  panics_if value == MIN_I32
  ensures result >= 0
  ensures result == value || result == -value

spec quotient where
  ensures right != 0 ==> result is Some && result.0 == left / right
  ensures right == 0 ==> result is None

spec signed_quotient where
  ensures right == 0 ==> result is None
  ensures left == MIN_I32 && right == -1 ==> result is None
  ensures left == -7 && right == 2 ==> result is Some && result.0 == -3

spec wrapped where
  panics_if right == 0
  ensures result == left % right

spec signed_wrapped where
  panics_if right == 0 || (left == MIN_I32 && right == -1)
  ensures left == -7 && right == 2 ==> result == 1

spec single_bit where
  ensures result == (∃ (k in 0 .. 8), value == (1 << k))

spec high_zeros where
  ensures value == 0 ==> result == 8
  ensures value > 0 ==> (1 << (7 - result)) <= value && value < (1 << (8 - result))

spec low_zeros where
  ensures value == 0 ==> result == 8
  ensures value != 0 ==> value % (1 << result) == 0 && value % (1 << (result + 1)) != 0
