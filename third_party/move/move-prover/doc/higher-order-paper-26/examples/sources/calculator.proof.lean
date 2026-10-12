-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- Compare stored closures by their captures and masks, then preserve the
-- stored invariants through the calculator's removal and publication.
verify process by
  verex_obligations
  all_goals (first | verex_decide | (verex_prepare; all_goals try verex_decide))
  all_goals simp_all [vir_denote_norm, VerexIR.Proofs.Denote.NTy.encode_function,
    VerexIR.Proofs.Denote.Weave.mask,
    VerexIR.Proofs.Denote.Which.project?_eq_some_iff,
    VerexIR.Proofs.Denote.Which.inject_here, VerexIR.Proofs.Denote.Which.inject_there,
    VerexIR.packResults_single]
  all_goals verex_split_goal
  all_goals try verex_memory_invariants
  all_goals try dsimp only [storedInvariant]
  all_goals try simp_all [vir_denote_norm, VerexIR.Proofs.Obligation_iff]
  all_goals verex_leaf
