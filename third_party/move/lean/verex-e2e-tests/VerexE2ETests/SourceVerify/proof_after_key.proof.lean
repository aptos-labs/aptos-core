-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "proof_after_key.move"
verex proofs where

verify caller by
  after call 1 of same {
    apply lemma_div_mul_le(x, c)
  }
  on _ { }
