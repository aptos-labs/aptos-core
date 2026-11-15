-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec less_u64 where
  panics_if false
  ensures result == (a < b)

spec order_i8 where
  panics_if false
  ensures a < b ==> result == -1
  ensures a == b ==> result == 0
  ensures a > b ==> result == 1

spec order_usize where
  panics_if false
  ensures a < b ==> result == 0
  ensures a == b ==> result == 1
  ensures a > b ==> result == 2

spec pick_u128 where
  panics_if false
  ensures result == (if a > b then a else b)
