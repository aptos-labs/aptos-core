-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec uses_low where
  ensures low(_range) == low(_range)

spec incremented where
  requires a < 100
  ensures result == a + 1

spec uses_rebinding where
  requires a < 100
  ensures result == incremented(a)

spec uses_read where
  ensures result == read_through(place)
