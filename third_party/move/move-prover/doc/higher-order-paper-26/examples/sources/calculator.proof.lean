-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- Compare stored closures by their captures and masks, then preserve the
-- stored invariants through the calculator's removal and publication.
verify process by
  all_goals simp_all [lir_denote_norm, LeanerIR.Proofs.Denote.NTy.encode_function,
    LeanerIR.Proofs.Denote.Weave.mask,
    LeanerIR.Proofs.Denote.Which.project?_eq_some_iff,
    LeanerIR.Proofs.Denote.Which.inject_here, LeanerIR.Proofs.Denote.Which.inject_there,
    LeanerIR.packResults_single]
  all_goals leaner_denote_split_goal
  all_goals try leaner_denote_memory_invariants
  all_goals try dsimp only [storedInvariant]
  all_goals try simp_all [lir_denote_norm, LeanerIR.Proofs.Obligation_iff]
  all_goals leaner_denote_leaf
