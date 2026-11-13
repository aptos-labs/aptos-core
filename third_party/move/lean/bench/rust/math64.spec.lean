-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec fun spec_pow(n : Int, e : Int) : Int decreases e :=
  if e <= 0 then 1 else n * spec_pow(n, e - 1)

spec max where
  ensures a >= b ==> result == a
  ensures a < b ==> result == b

spec min where
  ensures a < b ==> result == a
  ensures a >= b ==> result == b

spec average where
  ensures result == (a + b) / 2

spec gcd where
  ensures a > 0 || b > 0 ==> result > 0
  loop 1 where
    invariant a > 0 || b > 0 ==> large > 0

spec lcm where
  requires a <= MAX_U32 && b <= MAX_U32
  ensures a == 0 || b == 0 ==> result == 0

spec mul_div where
  panics_if c == 0
  panics_if a * b / c > MAX_U64
  ensures result == a * b / c

spec clamp where
  requires lower <= upper
  ensures lower <= x && x <= upper ==> result == x
  ensures x < lower ==> result == lower
  ensures upper < x ==> result == upper

-- Move's `pow` and `sqrt` are not verified either; `floor_log2` proves its range,
-- not Move's unverified `spec_pow` bounds.
spec pow where
  pragma verify = false
  panics_if spec_pow(n, e) > MAX_U64
  ensures result == spec_pow(n, e)

spec floor_log2 where
  panics_if x == 0
  ensures result <= 63
  loop 1 where
    invariant n <= 32
    invariant res + 2 * n <= 64
    invariant n == 0 ==> res <= 63

spec sqrt where
  pragma verify = false
  ensures x > 0 ==> result * result <= x
  ensures x > 0 ==> x < (result + 1) * (result + 1)
  ensures x == 0 ==> result == 0

spec ceil_div where
  panics_if y == 0
  ensures x == 0 ==> result == 0
  ensures x > 0 ==> result == (x - 1) / y + 1
