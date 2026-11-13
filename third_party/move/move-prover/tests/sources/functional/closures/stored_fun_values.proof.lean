-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "stored_fun_values.move"
verex proofs where

-- The call changes only `Counter`, whose invariant every value satisfies:
-- `Config` is kept, and the stored invariants hold after it.
verify use_counter_modifier by
  on ensures 1 {
    assert global<Config>(s.address) == old(global<Config>(s.address)) by solve
  }
  on stored invariant {
    apply lemma_stored_invariants_frame()
  }

verify use_config_aware_modifier by
  on ensures 1 {
    assert global<Config>(s.address) == old(global<Config>(s.address)) by solve
  }
  on stored invariant {
    apply lemma_stored_invariants_frame()
  }
