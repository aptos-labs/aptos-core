-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "proof_invariant_witness.move"
verex proofs where

verify make by
  on data invariant 1 {
    witness half
  }
  on _ { }

verify save by
  on data invariant 1 {
    witness half
  }
  on stored invariant {
    witness half
  }
  on _ { }
