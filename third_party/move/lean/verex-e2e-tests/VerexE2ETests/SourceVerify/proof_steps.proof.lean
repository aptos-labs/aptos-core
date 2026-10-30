-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "proof_steps.move"
verex proofs where

verify bump by
  on ensures 1 {
    forall i : Int where i <= x {
      assert i < x + 1
    }
  }
  on ensures 2 {
    forall i : Int where i <= x {
      calc(i + 1 <= x + 1 == 1 + x) by arith
    }
  }
  on _ { }

verify clamp by
  on ensures 1 {
    if x > 10 {
      assert x >= 11
    } else {
      calc(x <= 10 < 11)
    }
  }

verify pick by
  on ensures 1 {
    witness x
  }
  on ensures 2 {
    witness i + 1
  }
  on ensures 3 {
    assert i < x + 1
  }
  on _ { }

verify total by
  on ensures 1 {
    assert i <= p.a + p.b
  }
  on _ { }

-- The first call's fact is stated where its result is known, and holds for
-- every obligation after it.
verify twice by
  after call 1 of inc {
    assert inc_result >= x + 1
  }
  on ensures 1 {
    assert inc_result >= old_inc_result + 1
  }
  on _ { }

verify same_flag by
  on requires 1 of same {
    witness true, m + 1
  }
  on _ { }

-- One case per value of the index the precondition bounds.
verify pick3 by
  on ensures 1 {
    split i in 0..3
  }
  on _ { }

-- One case per variant of an expression's value.
verify made_size by
  on ensures 1 {
    split shape(k)
    assert size(shape(k)) <= k
  }
  on _ { }

-- The value is the variant, and its field is read by name.
verify radius by
  on ensures 1 {
    let Circle { r } := shape(k + 1)
    assert r == k + 1
  }
  on _ { }

-- Equal arguments, equal values of an uninterpreted function.
verify scaled by
  on ensures 1 {
    assert h(a * b) == h(2 * b) by congr
  }
  on _ { }

-- The fact's negation leads to a contradiction.
verify positive by
  on ensures 1 {
    assert x + 1 > 0 by contradiction {
      assert x + 1 <= 0
    }
  }
  on _ { }

-- The value the calls write back, the final one at the `ensures`, and the
-- value at the entry.
verify increment_twice by
  on ensures 1 {
    assert c'.n == old(c).n + 2
  }
  on _ { }

-- A lemma applied under the guard its premise needs.
verify double_small by
  on ensures 1 {
    apply x < 10 ==> double_bound(x)
    split x < 10
  }
  on _ { }
