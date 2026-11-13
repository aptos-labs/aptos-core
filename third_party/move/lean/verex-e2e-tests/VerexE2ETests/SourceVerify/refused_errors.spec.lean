-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- A refused function is an error at what refused it, and every other function
-- verifies or fails on its own. With a contract a caller assumes it.

spec promoted where
  ensures result == 5

spec reads_promoted where
  ensures result == 5

spec reads_limit where
  ensures result == 0

spec plain where
  ensures result == a / 2
