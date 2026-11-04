-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec swap where
  ensures left == old(right) && right == old(left)

spec swap_fields where
  ensures pair.first == old(pair.second) && pair.second == old(pair.first)

spec clear where
  ensures pair.first == 0 && pair.second == old(pair.second)

spec clear_twice where
  ensures pair.first == 0 && pair.second == old(pair.second)
