-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "proof_unfold.move"
verex proofs where

verify step by
  on ensures 1 {
    apply sum_upto.unfold(n)
  }
  on _ { }

verify base by
  on ensures 1 {
    apply sum_upto.unfold(0)
  }
  on _ { }

verify generic_step by
  on ensures 1 {
    apply count.unfold(v, n)
  }
  on _ { }
