-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "pure_spec_map_fact.move"
verex proofs where

verify v_len_set by
  on assert 1 {
    apply lemma_len_set(m, k, v)
  }
  on _ { }

verify v_set_param by
  on assert 1 {
    apply lemma_get_set(m, k, v, k)
  }
  on _ { }

verify v_rank_nonneg by
  on assert 1 {
    apply lemma_rank_nonneg(m, k)
  }
  on _ { }
