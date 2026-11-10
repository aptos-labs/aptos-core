-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- The references with contracts their code does not meet: each failure is
-- reported at the clause in this file.

spec swap where
  ensures left == old(left)

spec swap_fields where
  ensures pair.first == old(pair.first)

spec clear where
  ensures pair.first == 0 && pair.second == old(pair.second)

-- The two calls are kept: without them the fields would be unchanged.
spec clear_twice where
  ensures pair.first == old(pair.first)
