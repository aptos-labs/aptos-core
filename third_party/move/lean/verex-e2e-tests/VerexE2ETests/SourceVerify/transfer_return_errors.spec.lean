-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- A return between a move out of a holder and the holder's overwrite.

spec step where
  ensures final(result) == r
  ensures result == old(r)

spec return_between where
  ensures c ==> a == 1
  ensures !c ==> a == 2

spec return_between_false where
  ensures a == 2 -- error: not on the return

spec return_between_vacuous where
  ensures false -- error: the paths are not vacuous
