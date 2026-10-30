-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "for_loop_invariants.move"
verex proofs where

-- Triangular sums of nonnegative loop indices: the product is nonnegative,
-- so the quotient is the Euclidean one, and one step adds the index.
lemma triangular_nonneg(i : Int)
  requires 0 <= i
  ensures 0 <= i * (i - 1)
by
  split i == 0
  apply i != 0 ==> lemma_mul_nonneg(i, i - 1)

lemma triangular_step(i : Int)
  requires 0 <= i
  ensures i * (i - 1) / 2 + i == (i + 1) * (i + 1 - 1) / 2
by
  apply triangular_nonneg(i)
  assert (i + 1) * (i + 1 - 1) == i * (i - 1) + i * 2 by ring
  apply lemma_div_add_mul(i * (i - 1), i, 2)

lemma triangular_bounds(i : Int)
  requires 0 <= i && i <= 1000
  ensures 0 <= i * (i - 1) / 2 && i * (i - 1) / 2 <= 500000
by
  apply triangular_nonneg(i)
  split i == 0
  apply i != 0 ==> lemma_mul_mono(i, 1000, i - 1, 1000)
  apply lemma_div_nonneg(i * (i - 1), 2)
  apply lemma_div_le(i * (i - 1), 500000, 2)

verify sum_range by
  at loop 1 step {
    apply triangular_step(i)
    apply triangular_bounds(i)
  }
  on invariant 2 entry {
    assert 0 * -1 / 2 == 0 by eval
  }
  on ensures 1 {
    assert i == n
    assert i * (i - 1) / 2 == n * (n - 1) / 2 by congr
  }
  on _ { }

verify sum_from by
  at loop 1 step {
    apply triangular_step(i)
    apply triangular_bounds(i)
    apply triangular_bounds(a)
  }
  on ensures 1 {
    assert i == b
    assert i * (i - 1) / 2 == b * (b - 1) / 2 by congr
  }
  on _ { }

verify sum_skip_first by
  at loop 1 step {
    apply triangular_step(i)
    apply triangular_bounds(i)
  }
  on invariant 2 of loop 1 entry {
    assert 0 * -1 / 2 == 0 by eval
  }
  on ensures 1 {
    assert i == n
    assert i * (i - 1) / 2 == n * (n - 1) / 2 by congr
  }
  on _ { }
