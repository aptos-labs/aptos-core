-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "proof_invariant_of.move"
verex proofs where

verify make by
  on data invariant 1 of Even {
    witness half
  }
  on data invariant 1 of Triple {
    witness third
  }
  on _ { }
