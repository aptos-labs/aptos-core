-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "proof_lemma_quotient.move"
verex proofs where

lemma scaled_down(x : Int, c : Int)
  requires 0 <= x && 0 < c
  ensures x / c * c <= x
by
  lean =>
    have := Int.ediv_mul_le x (Int.ne_of_gt ‹0 < c›)
    omega

verify scaled by
  on ensures 1 {
    apply scaled_down(x, c)
  }
  on _ { }
