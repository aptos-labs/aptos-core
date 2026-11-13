-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "proof_implication_facts.move"
verex proofs where

verify at by
  on aborts_if 1 {
    assert n != 0
    split a == b
    if a < b {
      assert len(v) == s
    } else { }
  }
  on _ { }
