-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "staking_config.move"
verex proofs where

-- `e`, the rate, is at most one in fixed point; the denominator `q` it gives is at least the
-- numerator's bound, and any `d` up to `q` scales back to at most `d` and at most the bound.
lemma rate_quotient(q : Int, e : Int)
  requires 0 <= e && e != 0 && e <= 18446744073709551616
  requires q == 18446744073709551616000000 / e
  ensures 1000000 <= q && q <= 18446744073709551616000000
  ensures q * e <= 18446744073709551616000000
by
  apply lemma_le_div(18446744073709551616000000, 1000000, e)
  apply lemma_div_le(18446744073709551616000000, 18446744073709551616000000, e)
  apply lemma_div_mul_le(18446744073709551616000000, e)

lemma rate_scaled(d : Int, q : Int, e : Int)
  requires 0 <= e && e != 0 && e <= 18446744073709551616 && 0 <= d && d <= q
  requires q == 18446744073709551616000000 / e
  ensures (d * e) >> 64 <= 1000000
  ensures (d * e) >> 64 <= d
by
  apply rate_quotient(q, e)
  apply lemma_mul_nonneg(d, e)
  apply lemma_mul_mono(d, q, e, e)
  apply lemma_mul_mono(d, d, e, 18446744073709551616)
  apply lemma_shr_div(d * e, 64)
  apply lemma_div_le(d * e, 1000000, 18446744073709551616)
  apply lemma_div_le(d * e, d, 18446744073709551616)

-- The scaled rate fits a `FixedPoint64` of at most one: a numerator at most the
-- denominator gives a quotient at most `2^64`, a positive one a positive quotient.
verify initialize by
  at entry {
    if 0 < rewards_rate_denominator && rewards_rate <= rewards_rate_denominator {
      assert 0 <= rewards_rate << 64 && rewards_rate << 64 <= 18446744073709551616 * rewards_rate_denominator
        by apply lemma_shl_mul(rewards_rate, 64)
      apply lemma_div_le(rewards_rate << 64, 18446744073709551616, rewards_rate_denominator)
      assert (rewards_rate << 64) / rewards_rate_denominator <= 18446744073709551616
    } else { }
    if 0 < rewards_rate && 0 < rewards_rate_denominator {
      assert 0 <= rewards_rate << 64 && 1 * rewards_rate_denominator <= rewards_rate << 64
        by apply lemma_shl_mul(rewards_rate, 64)
      apply lemma_le_div(rewards_rate << 64, 1, rewards_rate_denominator)
      assert 1 <= (rewards_rate << 64) / rewards_rate_denominator
    } else { }
    assert 0 < rewards_rate || rewards_rate << 64 == 0 by apply lemma_shl_mul(rewards_rate, 64)
  }
  after call 1 of create_from_rational {
    assert old_create_from_rational_result.value <= 18446744073709551616
  }
  on _ { }

verify get_reward_rate by
  after call 1 of divide_u128 {
    let e := epoch_rewards_rate.value
    let q := divide_u128_result
    apply 0 <= e && e != 0 && e <= 18446744073709551616 ==> rate_quotient(q, e)
    apply 0 <= e && e != 0 && e <= 18446744073709551616 && 0 <= q ==> rate_scaled(q, q, e)
    apply 0 <= e && e != 0 && e <= 18446744073709551616 && q > 18446744073709551615 ==>
      rate_scaled(18446744073709551615, q, e)
  }
  -- The division's own overflow, on the paths that never reach the point above.
  on _ {
    let e := global<StakingRewardsConfig>(@aptos_framework).rewards_rate.value
    let q := 18446744073709551616000000 / e
    apply 0 <= e && e != 0 && e <= 18446744073709551616 ==> rate_quotient(q, e)
    apply 0 <= e && e != 0 && e <= 18446744073709551616 ==> rate_scaled(q, q, e)
    apply 0 <= e && e != 0 && e <= 18446744073709551616 && q > 18446744073709551615 ==>
      rate_scaled(18446744073709551615, q, e)
  }

-- As `get_reward_rate`, which this inlines: its quotient is stated from the stored rate, and
-- with the feature off the stored config's invariants give the clauses.
verify reward_rate by
  on ensures 1 {
    let e := global<StakingRewardsConfig>(@aptos_framework).rewards_rate.value
    let q := 18446744073709551616000000 / e
    apply 0 <= e && e != 0 && e <= 18446744073709551616 && 0 <= q ==> rate_scaled(q, q, e)
    apply 0 <= e && e != 0 && e <= 18446744073709551616 ==> rate_quotient(q, e)
    apply 0 <= e && e != 0 && e <= 18446744073709551616 && q > 18446744073709551615 ==>
      rate_scaled(18446744073709551615, q, e)
    if !spec_periodical_reward_rate_decrease_enabled() {
      assert global<StakingConfig>(@aptos_framework).rewards_rate <= MAX_REWARDS_RATE by solve
    } else { }
  }
  on ensures 2 {
    let e := global<StakingRewardsConfig>(@aptos_framework).rewards_rate.value
    apply 0 <= e && e != 0 && e <= 18446744073709551616 ==>
      rate_quotient(18446744073709551616000000 / e, e)
    if !spec_periodical_reward_rate_decrease_enabled() {
      assert global<StakingConfig>(@aptos_framework).rewards_rate_denominator > 0 by solve
    } else { }
  }
  on ensures 3 {
    let e := global<StakingRewardsConfig>(@aptos_framework).rewards_rate.value
    let q := 18446744073709551616000000 / e
    apply 0 <= e && e != 0 && e <= 18446744073709551616 && 0 <= q ==> rate_scaled(q, q, e)
    apply 0 <= e && e != 0 && e <= 18446744073709551616 ==> rate_quotient(q, e)
    apply 0 <= e && e != 0 && e <= 18446744073709551616 && q > 18446744073709551615 ==>
      rate_scaled(18446744073709551615, q, e)
    if !spec_periodical_reward_rate_decrease_enabled() {
      assert global<StakingConfig>(@aptos_framework).rewards_rate
        <= global<StakingConfig>(@aptos_framework).rewards_rate_denominator by solve
    } else { }
  }
  on aborts_if 1 {
    let e := global<StakingRewardsConfig>(@aptos_framework).rewards_rate.value
    let q := 18446744073709551616000000 / e
    apply 0 <= e && e != 0 && e <= 18446744073709551616 ==> rate_quotient(q, e)
    apply 0 <= e && e != 0 && e <= 18446744073709551616 && 0 <= q ==> rate_scaled(q, q, e)
    apply 0 <= e && e != 0 && e <= 18446744073709551616 && q > 18446744073709551615 ==>
      rate_scaled(18446744073709551615, q, e)
  }
  on _ { }
