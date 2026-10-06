-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! Division at a divisor that is not a literal: the quotient times the
divisor and the remainder add up to the dividend, the remainder lies below
a divisor a hypothesis makes positive, and a quotient of a dividend the
divisor divides cancels. -/

leaner module 0x42::division where
  fun divide_back(x : u64, y : u64) -> u64 := if y == 0 then 0 else x / y * y
  spec divide_back where
    ensures result <= x

  fun divide_self(x : u64) -> u64 := x / x
  spec divide_self where
    aborts_if x == 0
    ensures result == 1

  fun cancel_product(x : u64, y : u64) -> u64 := x * y / y
  spec cancel_product where
    aborts_if x * y > MAX_U64
    aborts_if y == 0
    ensures result == x

  fun remainder_below(x : u64, y : u64) -> u64 := x % y
  spec remainder_below where
    aborts_if y == 0
    ensures result < y

  fun remainder_below_positive(x : u64, y : u64) -> u64 := x % y
  spec remainder_below_positive where
    requires y > 0
    ensures result < y

  -- The quotient times the divisor lies within a divisor of the dividend,
  -- read without the remainder.
  fun multiple_within(x : u64, y : u64) -> u64 := x / y * y
  spec multiple_within where
    aborts_if y == 0
    ensures result + y > x

  spec lemma quotient_of_self(a : u64) where
    requires a > 0
    ensures a / a == 1

  -- ## Claims that do not hold

  fun divide_back_strictly(x : u64, y : u64) -> u64 := if y == 0 then 0 else x / y * y
  spec divide_back_strictly where
    ensures result < x -- error: equal where `y` divides `x`
