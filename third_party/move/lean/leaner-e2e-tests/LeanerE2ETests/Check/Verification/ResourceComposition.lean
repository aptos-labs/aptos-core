-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! Port of v0's `Verification/ResourceComposition.lean`: sequential focused
writes compose across distinct typed resource families. -/

namespace LeanerLang.Tests.VerificationResourceComposition

leaner module 0x42::resource_composition where
  struct Debit has Key where
    value : u64
  struct Credit has Key where
    value : u64

  fun shift(addr : Address, amount : u64) -> Unit := do
    let debit := &mut Debit[addr].value
    *debit := *debit - amount
    let credit := &mut Credit[addr].value
    *credit := *credit + amount

  spec shift where
    requires exists<Debit>(addr) && exists<Credit>(addr) &&
      amount <= old(global<Debit>(addr).value) &&
      old(global<Credit>(addr).value) + amount < 18446744073709551616
    modifies global<Debit>(addr)
    modifies global<Credit>(addr)
    ensures global<Debit>(addr).value == old(global<Debit>(addr).value) - amount &&
      global<Credit>(addr).value == old(global<Credit>(addr).value) + amount
    aborts_if false

  verify shift

end LeanerLang.Tests.VerificationResourceComposition
