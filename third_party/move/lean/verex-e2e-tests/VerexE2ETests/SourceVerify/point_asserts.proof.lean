-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "point_asserts.move"
verex proofs where

verify double_steps by
  at entry {
    assert x + x == 2 * x
  }

verify twice by
  after call 1 of inc {
    assert inc_result == x + 1 by arith
  }
  on ensures 1 { }

verify successor by
  at entry {
    assert x > 100
  }
