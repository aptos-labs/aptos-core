-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "bitwise_table.move"
verex proofs where

-- The table is closed: both additions and the write through the borrow,
-- which stores `22 & 1` at the first key, evaluate.
verify add_R by
  on ensures 1 {
    assert spec_len(result.t) == 2 by eval
  }
  on ensures 2 {
    assert spec_contains(result.t, new Key { v := concat(vec(1), vec(2)) }) by eval
    assert spec_contains(result.t, new Key { v := concat(vec(2), vec(3)) }) by eval
  }
  on ensures 3 {
    assert spec_get(result.t, new Key { v := concat(vec(1), vec(2)) }) == 0 by eval
  }
  on ensures 4 {
    assert spec_get(result.t, new Key { v := concat(vec(2), vec(3)) }) == 23 by eval
  }
  on _ { }
