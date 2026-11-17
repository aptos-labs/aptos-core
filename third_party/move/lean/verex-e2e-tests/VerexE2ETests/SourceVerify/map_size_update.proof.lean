-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "map_size_update.move"
verex proofs where

-- Every cell has a block, the negative too: a cell left to the automatic
-- route names this file in its failure, which the snapshot of this module
-- (the same items among the module's own) words differently.

verify size_after_insert by
  on assert 1 {
    lean =>
      rw [value_0_eq, VerexIR.SpecMap.size_update_sequence_absent (by assumption)]
  }
  on _ { }

verify size_after_override by
  on assert 1 {
    lean =>
      rw [value_0_eq, VerexIR.SpecMap.size_update_sequence_present (by assumption)]
  }
  on _ { }

verify size_after_unknown by
  on _ { }
