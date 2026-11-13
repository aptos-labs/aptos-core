-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- Mutable references held in aggregates: each function has a false twin, whose
-- clause is reported here.

spec ret_destruct where
  ensures result.x == 1 && result.y == 2

spec ret_destruct_false where
  ensures result.x == 2 -- error: `p` writes `a`

spec ret_swap where
  ensures result.x == 2 && result.y == 1

spec ret_swap_false where
  ensures result.x == 1 -- error: the callee swapped them

spec ret_cond_swap where
  ensures c ==> result.x == 2 && result.y == 1
  ensures !c ==> result.x == 1 && result.y == 2

spec ret_cond_swap_false where
  ensures result.x == 1 -- error: not where `c` holds

spec ret_write_first where
  ensures result.x == 1 && result.y == 6

spec ret_write_first_false where
  ensures result.y == b -- error: the callee wrote `b`

spec ret_mixed where
  ensures result.x == 1 && result.y == a

spec ret_mixed_false where
  ensures result.x == a -- error: `p` wrote `a`

spec take where
  ensures result == old(h).r

spec take_false where
  ensures result == old(h).r + 1 -- error: it writes nothing

spec field_ref where
  ensures result == old(it).r

spec field_ref_false where
  ensures result == old(it).r + 1 -- error: it writes nothing

spec tup_take where
  ensures true

spec tup_take_false where
  ensures false -- error: the path is not vacuous

-- A nested reference's clause value is the scalar it holds, not its loan carrier.
spec replace_nested where
  requires selected == 0
  ensures selected == old(replacement)

spec replace_nested_false where
  requires selected == 0
  ensures selected != old(replacement)
