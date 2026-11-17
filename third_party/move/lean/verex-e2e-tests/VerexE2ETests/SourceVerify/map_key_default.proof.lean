-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "map_key_default.move"
verex proofs where

-- Every cell has a block, the negatives too: a cell left to the automatic
-- route names this file in its failure, which the snapshot of this module
-- (the same items among the module's own) words differently.

verify key_at_rank by
  on assert 1 {
    apply lemma_key_at_rank(m, k)
  }
  on _ { }

verify key_at_ground by
  on assert 1 {
    apply lemma_key_at_rank(m, 7)
  }
  on _ { }

verify key_at_rank_narrow by
  on assert 1 {
    apply lemma_key_at_rank(m, k)
  }
  on _ { }

verify key_at_rank_generic by
  on assert 1 {
    apply lemma_key_at_rank(m, k)
  }
  on _ { }

verify key_at_other by
  on assert 1 {
    apply lemma_key_at_rank(m, 7)
  }
  on _ { }

verify key_at_outside by
  on _ { }

verify key_at_lt_generic by
  on assert 1 {
    apply lemma_key_at_lt(m, 0, 1)
  }
  on _ { }
