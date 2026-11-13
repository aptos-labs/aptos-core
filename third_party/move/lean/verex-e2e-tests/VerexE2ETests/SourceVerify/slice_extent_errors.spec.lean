-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec set_first where
  ensures old(values).length > 0 ==> values[0] == value

spec array_update where
  ensures result == 1 -- error: deliberately false postcondition

spec array_length_after where
  ensures result == 3 -- error: deliberately false postcondition

spec reborrow where
  ensures result != old(values).length -- error: deliberately false postcondition

spec sequential_reborrow where
  ensures result != old(values).length -- error: deliberately false postcondition

spec loop_reborrow where
  ensures result != old(values).length -- error: deliberately false postcondition
  loop 1 where
    invariant rounds <= 2

spec redirect_nested where
  ensures selected.length == old(selected).length -- error: deliberately false postcondition
