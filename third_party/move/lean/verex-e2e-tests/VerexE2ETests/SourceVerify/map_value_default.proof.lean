-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "map_value_default.move"
verex proofs where

-- Every cell has a block, the negative too: a cell left to the automatic
-- route names this file in its failure, which the snapshot of this module
-- (the same items among the module's own) words differently.

verify value_at_set by
  on assert 1 {
    apply lemma_get_set(old(m), k, v, k)
  }
  on _ { }

verify value_at_erase by
  on assert 1 {
    apply lemma_get_remove(old(m), k, p)
  }
  on _ { }

verify value_at_missing by
  on _ { }
