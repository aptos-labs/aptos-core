-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "proof_lemmas.move"
verex proofs where

-- A product scaled down by at most one stays within the factor.
lemma effective_bound(a : Int, scale : Int)
  requires 0 <= a && 0 <= scale && scale <= 10000
  ensures 0 <= a * scale / 10000 && a * scale / 10000 <= a
by
  apply lemma_mul_nonneg(a, scale)
  apply lemma_mul_mono(a, a, scale, 10000)
  apply lemma_div_nonneg(a * scale, 10000)
  apply lemma_div_le(a * scale, a, 10000)

lemma square_mono(a : Int, b : Int)
  requires 0 <= a && a <= b
  ensures a * a <= b * b
by
  apply lemma_mul_mono(a, b, a, b)

verify square by
  at entry {
    apply lemma_mul_nonneg(a, a)
    apply square_mono(a, b)
  }
  on _ { }

verify drop_at by
  on ensures 1 {
    apply lemma_vector_len_remove(v, i)
  }
  on ensures 2 {
    apply lemma_vector_index_remove(v, i, k)
  }
  on ensures 3 {
    apply lemma_vector_index_remove(v, i, k)
  }
  on _ { }

verify find by
  on ensures 1, ensures 2 {
    apply contains(v, e) ==> lemma_vector_index_of(v, e)
  }
  on _ { }

-- The fact of a `forall` block is what it asserts, in every case of its
-- split.
verify drop_first by
  on ensures 1 {
    forall j : Int where 0 <= j && j < len(old(v)) - 1 {
      split j < 0
      apply lemma_vector_index_remove(old(v), 0, j)
      assert v'[j] == old(v)[j + 1]
      assert old(v)[j + 1] != old(v)[j + 1] + 1
    }
  }
  on _ { }

verify shift by
  after call 1 of same {
    assert y == x + 1
  }
  on _ { }
