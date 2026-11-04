-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec array_length where
  ensures result == 3

spec array_first where
  ensures result == a[0]

spec borrowed_array_first where
  ensures result == a[0]
