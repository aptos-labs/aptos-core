-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "calculator.move"
verex proofs where

open VerexIR.Proofs.Denote

-- Compare stored closures by their captures and masks, then preserve the
-- stored invariants through the calculator's removal and publication.
verify process by
  verex_obligations
  all_goals (first | verex_decide | (verex_prepare; all_goals try verex_decide))
  all_goals simp_all [vir_denote_norm, NTy.encode_function,
    Weave.mask,
    Which.project?_eq_some_iff,
    Which.inject_here, Which.inject_there,
    packResults_single]
  all_goals verex_split_goal
  all_goals try verex_memory_invariants
  all_goals try dsimp only [storedInvariant]
  all_goals try simp_all [vir_denote_norm, Proofs.Obligation_iff]
  all_goals verex_leaf
