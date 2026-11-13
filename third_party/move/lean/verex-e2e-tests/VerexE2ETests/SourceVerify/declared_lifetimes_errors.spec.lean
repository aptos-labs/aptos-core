-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec write_other where
  ensures result.x == 2 -- error: the result writes `a`

spec pair_take_x where
  ensures result == a -- error: `b` is written

spec pair_take_y where
  ensures result.y == 1 -- error: the reference writes 2

spec tuple_write_second_lender where
  ensures result.y == 2 -- error: the lender is written last

spec pick_write where
  ensures result.x == 1 -- error: not when `!c`

spec elided_write where
  ensures result == a -- error: the reference wrote 4

spec method_write_other where
  ensures result.x == 2 -- error: the reference writes 1

spec generic_caller where
  ensures result == a -- error: the reference wrote 1
