-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "proof_assert_keys.move"
verex proofs where

verify blocks by
  on assert 1 {
    apply lemma_mul_nonneg(x, x)
  }
  on assert 2 {
    apply lemma_mul_nonneg(x + 1, x + 1)
  }
  on assert 3 {
    apply lemma_mul_nonneg(x + 2, x + 2)
  }
  on _ { }

verify after_closure by
  on assert 1 {
    apply lemma_mul_nonneg(x + 1, x + 1)
  }
  on assert 2 {
    apply lemma_mul_nonneg(x + 2, x + 2)
  }
  on _ { }
