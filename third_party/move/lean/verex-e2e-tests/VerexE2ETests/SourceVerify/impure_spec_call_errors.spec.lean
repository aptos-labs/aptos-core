-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec one where
  ensures result == (a <= b)

spec two where
  requires a < 100
  ensures result == (a < b)

-- This holds only with the second `next` read as nothing.
spec check where
  ensures one(a, b) == two(a, b)
