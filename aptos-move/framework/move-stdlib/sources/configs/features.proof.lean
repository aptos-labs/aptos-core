-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "features.move"
verex proofs where

open VerexIR VerexIR.Maps VerexIR.Proofs VerexIR.Proofs.Denote

-- Popping the last item preserves every earlier read. Reuse the reversed
-- prefix invariant at the same index after rewriting that read.
verify apply_diff by lean =>
  verex_obligations
  all_goals verex_prepare only
  all_goals try verex_decide
  all_goals verex_clear_computations
  all_goals verex_lookups_after_writes
  all_goals try verex_decide
  case invariant_8_of_loop_2_step =>
    simpa only [if_pos (by omega : 0 ≤ j.toNat ∧ j.toNat < disable.values.size)] using
      invariant_8 j (by omega) (by omega)
  case invariant_3_of_loop_1_step =>
    simpa only [if_pos (by omega : 0 ≤ j.toNat ∧ j.toNat < enable.values.size)] using
      invariant_3 j (by omega) (by omega)
