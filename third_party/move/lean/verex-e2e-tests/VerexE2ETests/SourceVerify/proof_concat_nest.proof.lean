-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "proof_concat_nest.move"
verex proofs where

verify stated by
  on ensures 1 {
    apply lemma_mul_mono(half, half, 1, k)
    apply lemma_div_le(half, half, k)
    assert len(a) + len(b) + len(c) + len(d) <= MAX_U64
  }
  on _ { }
