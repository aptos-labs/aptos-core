-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- The callee promises only its element effect. Extent follows from its type.
spec set_first where
  ensures old(values).length > 0 ==> values[0] == value

spec array_update where
  ensures result == 9

spec array_length_after where
  ensures result == 2
  ensures values[0] == 9

spec reborrow where
  ensures result == old(values).length
  ensures result > 0 ==> values[0] == 9

spec redirect where
  ensures result == old(right)
  ensures right == final(result)
  ensures left == old(left)

spec sequential_reborrow where
  ensures result == old(values).length
  ensures result > 0 ==> values[0] == 9

spec loop_reborrow where
  ensures result == old(values).length
  ensures result > 0 ==> values[0] == 11
  loop 1 where
    invariant rounds <= 2
    invariant rounds > 0 && values.length > 0 ==> values[0] == 11

spec redirect_nested where
  ensures selected == old(replacement)
