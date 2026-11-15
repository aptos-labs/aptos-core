-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "nonlinear_arithm.move"
verex proofs where

-- Five increasing unsigned integers have product zero, or at least
-- 1 * 2 * 3 * 4 * 5 = 120. In either case their product cannot be 72.
verify mul5 by
  on ensures 1 {
    split a == 0
    apply a != 0 ==> lemma_mul_mono(1, a, 2, b)
    apply a != 0 ==> lemma_mul_mono(2, a * b, 3, c)
    apply a != 0 ==> lemma_mul_mono(6, a * b * c, 4, d)
    apply a != 0 ==> lemma_mul_mono(24, a * b * c * d, 5, e)
  }
