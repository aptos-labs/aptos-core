-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "simple_map.move"
verex proofs where

open VerexIR VerexIR.Maps VerexIR.Proofs VerexIR.Proofs.Denote

-- Both vectors lose only their last item. Their earlier entries still
-- satisfy the reversed-prefix invariants, including generic encoded values.
verify add_all by lean =>
  verex_obligations
  verex_on
  | invariant_6_step, invariant_5_step =>
    verex_prepare only
    verex_clear_computations
    verex_lookups_after_writes
    first
    | (simpa only [if_pos (by omega : 0 ≤ j.toNat ∧ j.toNat < values.values.size)] using
        invariant_6 j (by omega) (by omega))
    | (simpa only [if_pos (by omega : 0 ≤ j.toNat ∧ j.toNat < keys.values.size)] using
        invariant_5 j (by omega) (by omega))
  | _ =>
    verex_prepare only
    all_goals try verex_decide
    all_goals verex_clear_computations
    all_goals verex_lookups_after_writes
    all_goals verex_decide
