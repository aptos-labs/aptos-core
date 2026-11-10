-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "proof_default_selector.move"
verex proofs where

verify pick by
  on _ when if 1 then {
    apply lemma_div_mul_le(x, c)
  }
  on _ { }
