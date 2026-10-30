-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "math_fixed64.move"
verex proofs where

-- `math128::sqrt` returns r with r² ≤ y < (r + 1)²: the root of a nonzero
-- u128 is between 1 and 2^64 - 1, so the scaled root does not wrap and is a
-- nonzero divisor, and the quotient of one Newton step from it stays below
-- (r + 3) 2^32.
lemma sqrt_root_range(y : Int, r : Int)
  requires (0 < y ==> r * r <= y) && (0 < y ==> y < (r + 1) * (r + 1)) && 0 < y
    && y <= 340282366920938463463374607431768211455 && 0 <= r
  ensures 1 <= r && r <= 18446744073709551615
by
  assert (r + 1) * (r + 1) == r * r + 2 * r + 1 by ring
  split r == 0
  apply r == 0 ==> lemma_mul_mono(r, 0, r, 0)
  split r >= 18446744073709551616
  apply r >= 18446744073709551616 ==>
    lemma_mul_mono(18446744073709551616, r, 18446744073709551616, r)

lemma scaled_root(y : Int, r : Int)
  requires (0 < y ==> r * r <= y) && (0 < y ==> y < (r + 1) * (r + 1)) && 0 < y
    && y <= 340282366920938463463374607431768211455 && 0 <= r
  ensures (r << 32) % 340282366920938463463374607431768211456 == r * 4294967296
    && 4294967296 <= r * 4294967296
by
  apply sqrt_root_range(y, r)
  apply lemma_shl_mul(r, 32)
  apply lemma_mod_small(r * 4294967296, 340282366920938463463374607431768211456)
  assert r << 32 == r * 4294967296 by congr
  assert (r << 32) % 340282366920938463463374607431768211456
    == (r * 4294967296) % 340282366920938463463374607431768211456 by congr

lemma newton_quotient(y : Int, r : Int)
  requires (0 < y ==> r * r <= y) && (0 < y ==> y < (r + 1) * (r + 1)) && 0 < y
    && y <= 340282366920938463463374607431768211455 && 0 <= r
  ensures (y << 64) / ((r << 32) % 340282366920938463463374607431768211456)
    < (r + 3) * 4294967296
by
  apply sqrt_root_range(y, r)
  apply scaled_root(y, r)
  apply lemma_shl_mul(y, 64)
  assert (r + 1) * (r + 1) == r * r + 2 * r + 1 by ring
  assert (r + 3) * 4294967296 * (r * 4294967296) == (r * r + 3 * r) * 18446744073709551616 by ring
  apply lemma_div_lt(y * 18446744073709551616, (r + 3) * 4294967296, r * 4294967296)
  assert (y << 64) / ((r << 32) % 340282366920938463463374607431768211456)
    == (y * 18446744073709551616) / (r * 4294967296) by congr

verify sqrt by
  after call 1 of sqrt {
    apply y != 0 ==> sqrt_root_range(y, sqrt_result)
    apply y != 0 ==> scaled_root(y, sqrt_result)
    apply y != 0 ==> newton_quotient(y, sqrt_result)
    apply lemma_shl_mul(y, 64)
    apply y != 0 ==> lemma_div_nonneg(y << 64, (sqrt_result << 32) % 340282366920938463463374607431768211456)
    apply y != 0 ==> lemma_mul_mono(y << 64, y << 64, 1, (sqrt_result << 32) % 340282366920938463463374607431768211456)
    apply y != 0 ==> lemma_div_le(y << 64, y << 64, (sqrt_result << 32) % 340282366920938463463374607431768211456)
    assert y * 18446744073709551616 <= 115792089237316195423570985008687907853269984665640564039457584007913129639935 by arith
    apply y != 0 ==> lemma_shr_div((sqrt_result << 32) % 340282366920938463463374607431768211456 + (y << 64) / ((sqrt_result << 32) % 340282366920938463463374607431768211456), 1)
    apply y != 0 ==> lemma_div_nonneg((sqrt_result << 32) % 340282366920938463463374607431768211456 + (y << 64) / ((sqrt_result << 32) % 340282366920938463463374607431768211456), 2)
    apply y != 0 ==> lemma_div_le((sqrt_result << 32) % 340282366920938463463374607431768211456 + (y << 64) / ((sqrt_result << 32) % 340282366920938463463374607431768211456), (sqrt_result << 32) % 340282366920938463463374607431768211456 + (y << 64) / ((sqrt_result << 32) % 340282366920938463463374607431768211456), 2)
  }
  on _ { }
