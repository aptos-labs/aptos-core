-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/-! Sequential focused
writes compose across distinct typed resource families. -/

namespace LeanerLang.Tests.Check.Storage.ResourceComposition

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
      old(global<Credit>(addr).value) + amount <= MAX_U64
    modifies global<Debit>(addr)
    modifies global<Credit>(addr)
    ensures global<Debit>(addr).value == old(global<Debit>(addr).value) - amount &&
      global<Credit>(addr).value == old(global<Credit>(addr).value) + amount
    aborts_if false

-- Run `shift` in the interpreter: both families are
-- updated at the address.
open LeanerIR LeanerE2ETests.CheckSupport in
run_cmd do
  let state (debit credit : Int) (nextLoan : Nat) :=
    resourceState `«0x42».resource_composition
      #[("Debit", "0x3", #[.integer debit]), ("Credit", "0x3", #[.integer credit])] nextLoan
  assertRunsState `«0x42».resource_composition #[
    ⟨"shift", #[.address "0x3", .integer 3], .returned #[], ← state 10 4 0, ← state 7 7 4⟩]

end LeanerLang.Tests.Check.Storage.ResourceComposition
