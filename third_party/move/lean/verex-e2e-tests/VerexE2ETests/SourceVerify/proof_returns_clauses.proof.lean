-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "proof_returns_clauses.move"
verex proofs where

verify rounded by
  on aborts_if 1 when returns {
    apply lemma_div_mul_le(x, c)
  }
  on _ {
    apply lemma_div_mul_le(x, c)
    apply lemma_div_nonneg(x, c)
    apply lemma_mul_nonneg(x / c, c)
    apply lemma_mul_mono(x, x, 1, c)
    apply lemma_div_le(x, x, c)
  }

verify two by
  on aborts_if 1 when returns {
    apply lemma_div_mul_le(x, c)
  }
  on _ {
    apply lemma_div_mul_le(x, c)
    apply lemma_div_nonneg(x, c)
    apply lemma_mul_nonneg(x / c, c)
    apply lemma_mul_mono(x, x, 1, c)
    apply lemma_div_le(x, x, c)
  }
