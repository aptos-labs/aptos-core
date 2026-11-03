-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "proof_unfold_errors.move"
verex proofs where

verify undecided by
  on ensures 1 {
    apply sum_upto.unfold(n)
  }
  on _ { }

verify not_recursive by
  on ensures 1 {
    apply twice.unfold(n)
  }
  on _ { }

verify names by
  on ensures 1 {
    apply «spec_fold$lambda$0».unfold(v, out, len)
  }
