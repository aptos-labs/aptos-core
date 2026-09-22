-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! Standard math utilities missing in the Move Language. -/
leaner module 0x1::math64 where
  use 0x1::std::error::invalid_argument
  use 0x1::std::fixed_point32::FixedPoint32
  use 0x1::std::fixed_point32::create_from_raw_value

  /--
  Cannot log2 the value 0
  -/
  const EINVALID_ARG_FLOOR_LOG2 : u64 := 1

  /--
  Return the largest of two numbers.
  -/
  public fun max(a : u64, b : u64) -> u64 :=
    if a >= b then a else b

  /--
  Return the smallest of two numbers.
  -/
  public fun min(a : u64, b : u64) -> u64 :=
    if a < b then a else b

  /--
  Return the average of two.
  -/
  public fun average(a : u64, b : u64) -> u64 :=
    if a < b then a + (b - a) / 2 else b + (a - b) / 2

  /--
  Return x clamped to the interval [lower, upper].
  -/
  public fun clamp(x : u64, lower : u64, upper : u64) -> u64 :=
    min(upper, max(lower, x))

  /--
  Return the value of n raised to power e
  -/
  public fun pow(n : u64, e : u64) -> u64 :=
    if e == 0 then 1
    else
      let p := 1
      while e > 1 do
        if e % 2 == 1 then p := p * n
        e := e / 2
        n := n * n
      return p * n

  /--
  Returns floor(lg2(x))
  -/
  public fun floor_log2(x : u64) -> u8 := do
    let res := 0u8
    assert!(x != 0, invalid_argument(EINVALID_ARG_FLOOR_LOG2))
    let n := 32u8
    while n > 0u8 do
      if x >= 1 << n then
        x := x >> n
        res := res + n
      n := n >> 1u8
    where
      invariant res + 2 * n <= 64
      invariant n == 0 ==> res <= 63
    return res

  -- Effectively the position of the most significant set bit
  -- Returns log2(x)
  public fun log2(x : u64) -> FixedPoint32 := do
    let integer_part := floor_log2(x)
    let y :=
      (if x >= 1 << 32u8 then x >> integer_part - 32u8
      else x << 32u8 - integer_part) as u128
    let frac := 0
    let delta := 1 << 31u8
    while delta != 0 do
      y := y * y >> 32u8
      if y >= 2u128 << 32u8 then
        frac := frac + delta
        y := y >> 1u8
      delta := delta >> 1u8
    return create_from_raw_value((integer_part as u64 << 32u8) + frac)

  -- Normalize x to [1, 2) in fixed point 32.
  -- log x = 1/2 log x^2
  -- x in [1, 2)
  -- x is now in [1, 4)
  -- if x in [2, 4) then log x = 1 + log (x / 2)
  /--
  Returns square root of x, precisely floor(sqrt(x))
  -/
  public fun sqrt(x : u64) -> u64 := do
    if x == 0 then return 0;
    let res := 1 << (floor_log2(x) + 1u8 >> 1u8)
    res := res + x / res >> 1u8
    res := res + x / res >> 1u8
    res := res + x / res >> 1u8
    res := res + x / res >> 1u8
    return min(res, x / res)

  -- Note the plus 1 in the expression. Let n = floor_lg2(x) we have x in [2^n, 2^(n+1)> and thus the answer in
  -- the half-open interval [2^(n/2), 2^((n+1)/2)>. For even n we can write this as [2^(n/2), sqrt(2) 2^(n/2)>
  -- for odd n [2^((n+1)/2)/sqrt(2), 2^((n+1)/2>. For even n the left end point is integer for odd the right
  -- end point is integer. If we choose as our first approximation the integer end point we have as maximum
  -- relative error either (sqrt(2) - 1) or (1 - 1/sqrt(2)) both are smaller then 1/2.
  -- We use standard newton-rhapson iteration to improve the initial approximation.
  -- The error term evolves as delta_i+1 = delta_i^2 / 2 (quadratic convergence).
  -- It turns out that after 4 iterations the delta is smaller than 2^-32 and thus below the treshold.
  -- No overflow
  -- Note that ordering other way is imprecise.
  -- idx + log2 (1 - 1/2^idx) = idx + ln (1-1/2^idx)/ln2
  -- Use 3rd order taylor to approximate expected result
  -- verify it matches to 8 significant digits
