-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec check where
  ensures result == a

verify check by
  on ensures 1 {
    assert advance(a) == 1
  }
