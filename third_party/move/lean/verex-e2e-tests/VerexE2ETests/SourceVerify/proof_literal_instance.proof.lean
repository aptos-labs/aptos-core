-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "proof_literal_instance.move"
verex proofs where

verify first by
  on ensures 1 {
    assert 0 <= v[0]
  }
  on _ { }
