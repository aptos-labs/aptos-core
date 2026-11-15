-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "amm_example.move"
verex proofs where

-- The denominator contains the input, so division cannot exceed the output
-- reserve; a positive input reserve makes that bound strict.
verify constant_product by
  at entry {
    apply lemma_mul_nonneg(reserve_out, amount_in)
    apply lemma_mul_mono(reserve_out, 18446744073709551615, amount_in, 18446744073709551615)
    apply lemma_mul_mono(reserve_out, reserve_out, amount_in, reserve_in + amount_in)
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
