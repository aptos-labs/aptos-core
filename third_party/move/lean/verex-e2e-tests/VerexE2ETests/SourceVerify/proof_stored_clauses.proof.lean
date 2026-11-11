-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "proof_stored_clauses.move"
verex proofs where

verify save by
  on data invariant 1, stored invariant 1 {
    witness half
  }
  on _ { }
