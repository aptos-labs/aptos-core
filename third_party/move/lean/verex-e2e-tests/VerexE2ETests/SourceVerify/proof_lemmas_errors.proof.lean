-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "proof_lemmas_errors.move"
verex proofs where

-- The conclusion does not follow without the product's sign.
lemma unsigned_square(a : Int)
  ensures 0 <= a * a
by
  assert a * a == a * a by ring

-- A built-in lemma's premise the facts do not give.
lemma unbounded_quotient(x : Int, c : Int)
  requires 0 < c
  ensures 0 <= x / c
by
  apply lemma_div_nonneg(x, c)
