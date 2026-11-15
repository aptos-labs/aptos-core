-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec max where
  ensures result == a

spec average where
  ensures result == (a + b + 1) / 2

spec gcd where
  ensures result > 0
  loop 1 where
    invariant a > 0 || b > 0 ==> large > 0

spec lcm where
  ensures a == 0 || b == 0 ==> result == 0

spec mul_div_overflow where
  panics_if c == 0
  ensures result == a * b / c

spec clamp where
  ensures x < lower ==> result == lower

spec ceil_div where
  panics_if y == 0
  ensures x > 0 ==> result == x / y
