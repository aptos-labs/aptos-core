-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "proof_apply_no_parameters.move"
verex proofs where

verify halved by
  on ensures 1 {
    apply lemma_div_mul_le(Config[@0x1].rate, 2)
  }
  on _ { }
