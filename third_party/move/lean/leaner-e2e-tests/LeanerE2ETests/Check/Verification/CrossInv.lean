-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# Cross-resource module invariant

Port of v0's `Verification/CrossInv.lean`.
-/

namespace LeanerLang.Tests.VerificationCrossInv

leaner module 0x42::cross_inv where
  struct Debit has Key where
    value : u64

  struct Credit has Key where
    value : u64

  spec module where
    invariant forall (a : Address),
      global<Debit>(a).value <= global<Credit>(a).value

  public entry fun shift(addr : Address, amount : u64) -> Unit := do
    let Debit { value := debit } := move_from<Debit>(addr)
    let Credit { value := credit } := move_from<Credit>(addr)
    move_to<Debit>(addr, new Debit { value := debit - amount })
    move_to<Credit>(addr, new Credit { value := credit + amount })

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

end LeanerLang.Tests.VerificationCrossInv
