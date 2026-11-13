-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "proof_disjuncts.move"
verex proofs where

verify positive_at by
  on ensures 1 {
    assert n != 0
    assert v[i] > 0
  }
  on _ { }
