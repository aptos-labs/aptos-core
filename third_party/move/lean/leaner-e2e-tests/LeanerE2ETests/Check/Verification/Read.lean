-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# Shared global reads and conditional aborts

Port of v0's `Verification/Read.lean`. The execution assertions belong to
the differential suite; both source contracts are verified here.
-/

namespace LeanerLang.Tests.VerificationRead

leaner module 0x42::read where
  struct Reading has Key where
    value : u64

  fun read(addr : Address) -> u64 := do
    let value := &Reading[addr].value
    return *value

  spec read where
    requires exists<Reading>(addr)
    ensures result == old(global<Reading>(addr).value)
    aborts_if false

  fun read_at_least(addr : Address, minimum : u64) -> u64 := do
    let value := &Reading[addr].value
    let current := *value
    if current < minimum then
      abort(7)
    return current

  spec read_at_least where
    requires exists<Reading>(addr)
    ensures result == old(global<Reading>(addr).value) && minimum <= result
    aborts_if old(global<Reading>(addr).value) < minimum with 7

  verify read
  verify read_at_least

end LeanerLang.Tests.VerificationRead
