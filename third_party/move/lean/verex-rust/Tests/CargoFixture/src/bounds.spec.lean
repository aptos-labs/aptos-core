-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec within where
  ensures result == (value < 100)

mod compare where
  spec below where
    ensures result == (value < limit)
