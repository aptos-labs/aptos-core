-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "proof_negations.move"
verex proofs where

verify sorted_pair by
  on ensures 1 {
    assert v[x] < v[x + 1]
  }
  on _ { }
