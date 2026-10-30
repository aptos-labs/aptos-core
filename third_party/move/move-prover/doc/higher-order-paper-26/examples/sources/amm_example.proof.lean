-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "amm_example.move"
verex proofs where

open VerexIR.Proofs.Denote

-- The denominator contains the input, so division cannot exceed the output
-- reserve; a positive input reserve makes that bound strict.
verify constant_product by
  at entry {
    apply lemma_mul_mono(reserve_out, reserve_out, amount_in, reserve_in + amount_in)
    apply lemma_mul_nonneg(reserve_out, amount_in)
    apply lemma_mul_mono(reserve_out, 18446744073709551615, amount_in, 18446744073709551615)
    apply reserve_in + amount_in > 0 ==>
      lemma_div_nonneg(reserve_out * amount_in, reserve_in + amount_in)
    apply reserve_in + amount_in > 0 ==>
      lemma_div_le(reserve_out * amount_in, reserve_out, reserve_in + amount_in)
    apply reserve_in > 0 && reserve_out > 0 ==>
      lemma_mul_strict_mono(amount_in, reserve_in + amount_in, reserve_out)
    assert amount_in * reserve_out == reserve_out * amount_in by ring
    assert (reserve_in + amount_in) * reserve_out == reserve_out * (reserve_in + amount_in) by ring
    apply reserve_in > 0 && reserve_out > 0 ==>
      lemma_div_lt(reserve_out * amount_in, reserve_out, reserve_in + amount_in)
  }
  on _ { }

-- A product scaled down by at most one stays within the factor.
lemma effective_bound(a : Int, scale : Int)
  requires 0 <= a && 0 <= scale && scale <= 10000
  ensures 0 <= a * scale / 10000 && a * scale / 10000 <= a
by
  apply lemma_mul_nonneg(a, scale)
  apply lemma_mul_mono(a, a, scale, 10000)
  apply lemma_div_nonneg(a * scale, 10000)
  apply lemma_div_le(a * scale, a, 10000)

-- A rounded constant-product swap of a positive effective input `e <= a`
-- stays within the output reserve and preserves the reserves' product.
lemma market_bounds(r : Int, o : Int, a : Int, e : Int)
  requires 0 <= r && 0 <= o && o <= 18446744073709551615
  requires a <= 18446744073709551615 && 0 < e && e <= a
  ensures 0 <= o * e && o * e <= 340282366920938463463374607431768211455
  ensures 0 <= o * e / (r + e) && o * e / (r + e) <= o
  ensures r * o <= (r + a) * (o - o * e / (r + e))
by
  let q := o * e / (r + e)
  apply lemma_mul_nonneg(o, e)
  apply lemma_mul_mono(o, 18446744073709551615, e, 18446744073709551615)
  apply lemma_mul_mono(o, o, e, r + e)
  apply lemma_div_nonneg(o * e, r + e)
  apply lemma_div_le(o * e, o, r + e)
  apply lemma_div_mul_le(o * e, r + e)
  apply lemma_mul_mono(r + e, r + a, o - q, o - q)
  assert (r + e) * (o - q) == r * o + e * o - q * (r + e) by ring
  assert e * o == o * e by ring

-- A swap of the input scaled by `s / 10000`: the scaled input's bounds, and,
-- where it is positive, the market bounds; without input the product of the
-- reserves is preserved as well.
lemma scaled_market(r : Int, o : Int, a : Int, s : Int)
  requires 0 <= r && 0 <= o && o <= 18446744073709551615
  requires 0 <= a && a <= 18446744073709551615 && 0 <= s && s <= 10000
  ensures 0 <= a * s && a * s <= 340282366920938463463374607431768211455
  ensures 0 <= a * s / 10000 && a * s / 10000 <= a
  ensures a * s / 10000 > 0 ==> 0 <= o * (a * s / 10000)
  ensures a * s / 10000 > 0 ==> o * (a * s / 10000) <= 340282366920938463463374607431768211455
  ensures a * s / 10000 > 0 ==> 0 <= o * (a * s / 10000) / (r + a * s / 10000)
  ensures a * s / 10000 > 0 ==> o * (a * s / 10000) / (r + a * s / 10000) <= o
  ensures a * s / 10000 > 0 ==>
    r * o <= (r + a) * (o - o * (a * s / 10000) / (r + a * s / 10000))
  ensures r * o <= (r + a) * o
by
  apply lemma_mul_nonneg(a, s)
  apply lemma_mul_mono(a, 18446744073709551615, s, 10000)
  apply effective_bound(a, s)
  apply a * s / 10000 > 0 ==> market_bounds(r, o, a, a * s / 10000)
  apply lemma_mul_mono(r, r + a, o, o)

-- The swap at a fee of `fee` basis points.
lemma fee_market(r : Int, o : Int, a : Int, fee : u64)
  requires 0 <= r && 0 <= o && o <= 18446744073709551615
  requires 0 <= a && a <= 18446744073709551615 && fee <= 10000
  ensures 0 <= a * (10000 - fee) && a * (10000 - fee) <= 340282366920938463463374607431768211455
  ensures 0 <= a * (10000 - fee) / 10000 && a * (10000 - fee) / 10000 <= a
  ensures a * (10000 - fee) / 10000 > 0 ==> 0 <= o * (a * (10000 - fee) / 10000)
  ensures a * (10000 - fee) / 10000 > 0 ==>
    o * (a * (10000 - fee) / 10000) <= 340282366920938463463374607431768211455
  ensures a * (10000 - fee) / 10000 > 0 ==>
    0 <= o * (a * (10000 - fee) / 10000) / (r + a * (10000 - fee) / 10000)
  ensures a * (10000 - fee) / 10000 > 0 ==>
    o * (a * (10000 - fee) / 10000) / (r + a * (10000 - fee) / 10000) <= o
  ensures a * (10000 - fee) / 10000 > 0 ==>
    r * o <= (r + a) * (o - o * (a * (10000 - fee) / 10000) / (r + a * (10000 - fee) / 10000))
by
  apply scaled_market(r, o, a, 10000 - fee)

verify constant_product_with_fee by
  at entry {
    apply scaled_market(reserve_in, reserve_out, amount_in, 9500)
  }
  on aborts_if 1, ensures 1, ensures 2, ensures 3, ensures 4, assert 1 when if 1 then {
    apply fee_market(reserve_in, reserve_out, amount_in, global<Fee>(owner).bps)
  }
  on _ { }

-- The stored fee, where the owner has one within 100%.
verify constant_product_with_fee_non_compliant by
  on _ {
    apply exists<Fee>(owner) && global<Fee>(owner).bps <= 10000 ==>
      fee_market(reserve_in, reserve_out, amount_in, global<Fee>(owner).bps)
  }

-- Rounded constant-product pricing is monotone in its input.
lemma ratio_monotone(r : Int, o : Int, a : Int, b : Int)
  requires 0 <= r && 0 <= o && 0 <= a && a <= b && 0 < r + a
  ensures o * a / (r + a) <= o * b / (r + b)
by
  let q := o * a / (r + a)
  apply lemma_mul_nonneg(o, a)
  apply lemma_div_mul_le(o * a, r + a)
  apply lemma_mul_mono(o, o, a, r + a)
  apply lemma_div_le(o * a, o, r + a)
  apply lemma_div_nonneg(o * a, r + a)
  apply lemma_mul_mono(q, o, b - a, b - a)
  assert q * (r + b) == q * (r + a) + q * (b - a) by ring
  assert o * b == o * a + o * (b - a) by ring
  apply lemma_mul_nonneg(o, b)
  apply lemma_le_div(o * b, q, r + b)

verify create_constant_product_pool by
  on data invariant 3 {
    apply r_in + a1 > 0 ==> ratio_monotone(r_in, r_out, a1, a2)
  }
  on data invariant 4 {
    split amt > 0
    apply amt > 0 ==> market_bounds(r_in, r_out, amt, amt)
    if amt > 0 { assert !(r_in == 0 && amt == 0) }
    if amt <= 0 { assert amt == 0 }
    split amt <= 0 && r_in == 0
  }
  on _ { }

-- The pool's laws at the swap's reserves: the pricing call does not abort,
-- stays within the output reserve, and preserves the reserves' product.
verify swap by
  on aborts_if 1 {
    assert !aborts_of<pool.pricing>(pool.reserve_x, pool.reserve_y, amount_in)
  }
  after invoke 1 {
    assert result_of<pool.pricing>(pool.reserve_x, pool.reserve_y, amount_in) <= pool.reserve_y
    assert pool.reserve_x * pool.reserve_y <= (pool.reserve_x + amount_in) *
      (pool.reserve_y - result_of<pool.pricing>(pool.reserve_x, pool.reserve_y, amount_in))
  }
  on ensures 5 {
    assert pool'.reserve_x == pool.reserve_x + amount_in
    assert pool'.reserve_y == pool.reserve_y -
      result_of<pool.pricing>(pool.reserve_x, pool.reserve_y, amount_in)
  }
  on _ { }

-- The fee-adjusted swap is monotone in its input: the effective inputs are,
-- and the rounded constant-product ratio is.
lemma fee_monotone(r : Int, o : Int, a : Int, b : Int, fee : u64)
  requires 0 <= r && 0 <= o && 0 <= a && a <= b && fee <= 10000
  ensures a * (10000 - fee) / 10000 > 0 ==>
    o * (a * (10000 - fee) / 10000) / (r + a * (10000 - fee) / 10000) <=
      o * (b * (10000 - fee) / 10000) / (r + b * (10000 - fee) / 10000)
  ensures b * (10000 - fee) / 10000 > 0 ==>
    0 <= o * (b * (10000 - fee) / 10000) / (r + b * (10000 - fee) / 10000)
  ensures a * (10000 - fee) / 10000 <= b * (10000 - fee) / 10000
by
  apply lemma_mul_nonneg(a, 10000 - fee)
  apply lemma_mul_mono(a, b, 10000 - fee, 10000 - fee)
  apply lemma_div_mono(a * (10000 - fee), b * (10000 - fee), 10000)
  apply lemma_div_nonneg(a * (10000 - fee), 10000)
  apply a * (10000 - fee) / 10000 > 0 ==>
    ratio_monotone(r, o, a * (10000 - fee) / 10000, b * (10000 - fee) / 10000)
  apply lemma_mul_nonneg(o, b * (10000 - fee) / 10000)
  apply b * (10000 - fee) / 10000 > 0 ==>
    lemma_div_nonneg(o * (b * (10000 - fee) / 10000), r + b * (10000 - fee) / 10000)

-- At the fee the state holds, or the default.
verify create_compliant_fee_pool by
  on data invariant 3 {
    split S |~ (exists<Fee>(account.address) && global<Fee>(account.address).bps <= 10000)
    apply (S |~ (exists<Fee>(account.address) && global<Fee>(account.address).bps <= 10000)) ==>
      fee_monotone(r_in, r_out, a1, a2, (S |~ global<Fee>(account.address).bps))
    apply fee_monotone(r_in, r_out, a1, a2, 500)
    if S |~ (exists<Fee>(account.address) && global<Fee>(account.address).bps <= 10000) {
      let fee := S |~ global<Fee>(account.address).bps
      apply lemma_mul_nonneg(a1, 10000 - fee)
      apply lemma_div_nonneg(a1 * (10000 - fee), 10000)
      split 0 < a1 * (10000 - fee) / 10000
      if 0 < a1 * (10000 - fee) / 10000 { assert 0 < a2 * (10000 - fee) / 10000 }
      if a1 * (10000 - fee) / 10000 <= 0 { assert a1 * (10000 - fee) / 10000 == 0 }
    }
    if !(S |~ (exists<Fee>(account.address) && global<Fee>(account.address).bps <= 10000)) {
      split 0 < a1 * 9500 / 10000
      if 0 < a1 * 9500 / 10000 { assert 0 < a2 * 9500 / 10000 }
      if a1 * 9500 / 10000 <= 0 { assert a1 * 9500 / 10000 == 0 }
    }
  }
  on _ { }
