-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- Contracts the code does not meet: each failure is reported at the clause in
-- this file.

spec read where
  requires x < 100
  ensures result == x + 1

spec get where
  requires h.r < 100
  ensures result == h.r + 1

spec through where
  requires x < 100
  ensures result == x + 1

spec lender_written where
  ensures result == 2

spec slot where
  ensures result == 0
