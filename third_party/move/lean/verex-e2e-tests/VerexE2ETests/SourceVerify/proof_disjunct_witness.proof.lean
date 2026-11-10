-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "proof_disjunct_witness.move"
verex proofs where

verify bad by
  on ensures 1 {
    assert ∃ (k : u64), k < len(v) - 1 && v[k] >= v[k + 1] by witness i - 1
  }
  on _ { }
