-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec below_limit where
  ensures result == (value < 1024)

spec maximum where
  ensures result == 18446744073709551615

spec origin where
  ensures result == 3

spec count where
  ensures result == 7
