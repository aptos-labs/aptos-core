-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# Shared global reads and conditional aborts

Both source contracts are verified; execution is left to the differential
suite.
-/

namespace LeanerLang.Tests.Check.Storage.Read

leaner module 0x42::read where
  struct Reading has Key where
    value : u64

  fun read(addr : Address) -> u64 := do
    let value := &Reading[addr].value
    *value
  spec read where
    requires exists<Reading>(addr)
    ensures result == old(global<Reading>(addr).value)
    aborts_if false

  fun read_at_least(addr : Address, minimum : u64) -> u64 := do
    let value := &Reading[addr].value
    let current := *value
    if current < minimum then
      abort(7)
    current
  spec read_at_least where
    requires exists<Reading>(addr)
    ensures result == old(global<Reading>(addr).value) && minimum <= result
    aborts_if old(global<Reading>(addr).value) < minimum with 7

end LeanerLang.Tests.Check.Storage.Read
