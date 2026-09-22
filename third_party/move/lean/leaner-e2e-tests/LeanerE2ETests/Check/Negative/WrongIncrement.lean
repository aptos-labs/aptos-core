-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! A contract its implementation does not prove: the verification
failure, as `lean` reports it, is the expectation. -/

leaner module 0x42::wrong_increment where
  public fun wrong_increment(value : u64) -> u64 := value + 1

  spec wrong_increment where
    ensures result == value
    aborts_if value + 1 > 18446744073709551615

  verify wrong_increment
