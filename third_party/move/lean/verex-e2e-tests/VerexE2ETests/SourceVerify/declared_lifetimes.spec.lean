-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec write_other where
  ensures result.x == 1 && result.y == 2

spec pair_take_x where
  ensures result == 2

spec pair_take_y where
  ensures result.x == 1 && result.y == 2

spec tuple_write_second_lender where
  ensures result.x == 1 && result.y == 3

spec pick_write where
  ensures c ==> result.x == 1 && result.y == b
  ensures !c ==> result.y == 1 && result.x == a

spec elided_write where
  ensures result == 4

spec method_write_other where
  ensures result.x == 1 && result.y == 2

spec generic_caller where
  ensures result == 1
