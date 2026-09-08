-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! A contract its implementation proves: the check is clean and has no
expectation file. -/

leaner module 0x42::increment where
  public fun increment(value : u64) -> u64 := value + 1

  spec increment where
    ensures result == value + 1
    aborts_if value + 1 > 18446744073709551615

  verify increment
