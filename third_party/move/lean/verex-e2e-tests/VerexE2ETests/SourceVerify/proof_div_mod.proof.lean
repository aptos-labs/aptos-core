-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "proof_div_mod.move"
verex proofs where

verify position by
  at entry {
    apply lemma_div_mod(i, s)
  }
  on _ { }
