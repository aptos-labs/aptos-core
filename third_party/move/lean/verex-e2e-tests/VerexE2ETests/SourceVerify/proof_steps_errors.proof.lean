-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "proof_steps_errors.move"
verex proofs where

verify bump by
  on ensures 1 {
    forall i : Int where i <= x {
      assert i < x
    }
  }
  on ensures 2 {
    forall i : Int where i <= x {
      calc(i + 1 <= x + 1 == 2 + x) by arith
    }
  }
  on _ {
    assume x > 0
  }

verify clamp by
  on ensures 1 {
    if x > 10 {
      assert x >= 12
    } else {
      calc(x <= 9 < 11)
    }
  }

verify pick by
  on ensures 1 {
    witness x + 1
  }
  on ensures 2 {
    witness i + 2
  }
  on ensures 3 {
    assert i < x
  }
  on _ { }

verify total by
  on ensures 1 {
    assert i <= p.b
  }
  on _ { }

verify twice by
  on ensures 1 {
    assert inc_result >= old_inc_result + 2
  }
  on _ { }

verify same_flag by
  on requires 1 of same {
    witness true, m
  }
  on _ { }
