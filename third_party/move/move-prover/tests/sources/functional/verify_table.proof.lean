-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "verify_table.move"
verex proofs where

open VerexIR VerexIR.Proofs VerexIR.Proofs.Denote

-- Normalize the short sequence of writes before deciding its size and reads.
-- Position search is unnecessary for these concrete table operations.
verify add by
  verex_obligations
  all_goals verex_prepare only
  all_goals verex_decide

verify remove by
  verex_obligations
  all_goals verex_prepare only
  all_goals try verex_decide
  all_goals subst old_value''' old_value'' old_value' old_value new_result
  all_goals simp [Maps.remove, Maps.update, Maps.Discipline.del, Maps.Discipline.set,
    Maps.seqSet, Maps.seqDel, Maps.swapRemove, Maps.indexOf, Maps.get, vir_denote_norm]

verify contains_and_length by
  verex_obligations
  all_goals verex_prepare only
  all_goals try verex_decide
  all_goals subst old_value' old_value new_result
  all_goals simp [Maps.update, Maps.Discipline.set, Maps.seqSet, vir_denote_norm] at *
  all_goals omega
