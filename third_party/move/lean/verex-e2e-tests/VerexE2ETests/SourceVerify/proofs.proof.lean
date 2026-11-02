-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "proofs.move"
verex proofs where

-- The expansion of the square proves the result; its sign proves the abort
-- conditions.
verify square_of_sum by
  at entry {
    assert (a + b) * (a + b) == a * a + 2 * a * b + b * b by ring
    apply lemma_mul_nonneg(a + b, a + b)
  }
  on _ { }
