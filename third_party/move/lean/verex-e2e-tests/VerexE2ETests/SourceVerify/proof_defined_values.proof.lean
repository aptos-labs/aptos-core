-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "proof_defined_values.move"
verex proofs where

verify instance by
  on ensures 1 {
    apply lemma_div_nonneg(i, self.size)
    assert contained(self.entries, i / self.size)
  }
  on _ { }

verify named by
  on ensures 1 {
    apply lemma_div_nonneg(i, self.size)
    let cut : u64 where i / self.size == cut && contained(self.entries, cut)
    assert spec_get(self.entries, cut) == 7
    assert cut <= MAX_U64
  }
  on _ { }
