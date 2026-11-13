-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- Each contract is false of its function.

spec array_length where
  ensures result == 2

spec array_first where
  ensures result == a[0] + 1

spec borrowed_array_first where
  ensures result == a[1]
