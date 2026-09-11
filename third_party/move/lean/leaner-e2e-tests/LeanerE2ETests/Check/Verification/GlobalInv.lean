-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# Global invariants

Port of v0's `Verification/GlobalInv.lean`.  The publishing entry takes the
resolved address directly because LeanerLang storage intrinsics do not project
an address from a signer.
-/

namespace LeanerLang.Tests.VerificationGlobalInv

set_option leaner.route "native"
set_option leaner.verifyHeartbeats 50000

leaner module 0x42::global_inv where
  struct Counter has Key where
    value : u64

  struct Ledger has Key where
    total : u64

  spec module where
    invariant forall (a : Address),
      0 < global<Counter>(a).value
    invariant [update] forall (a : Address),
      old(global<Counter>(a).value) <= global<Counter>(a).value
    invariant [update] forall (a : Address),
      old(global<Ledger>(a).total) <= global<Ledger>(a).total

  public entry fun increment(addr : Address) -> Unit := do
    let value := &mut Counter[addr].value
    *value := *value + 1

  spec increment where
    requires exists<Counter>(addr)
    modifies global<Counter>(addr)
    ensures global<Counter>(addr).value == old(global<Counter>(addr).value) + 1
    aborts_if old(global<Counter>(addr).value) + 1 > 18446744073709551615

  verify increment

  public entry fun publish(address : Address, amount : u64) -> Unit :=
    move_to<Counter>(address, new Counter { value := amount })

  spec publish where
    requires 0 < amount
    modifies global<Counter>(address)
    ensures global<Counter>(address).value == amount

  verify publish

  public fun is_published(addr : Address) -> Bool := exists<Counter>(addr)

  spec is_published where
    ensures result == exists<Counter>(addr)

  verify is_published

  public fun remove(addr : Address) -> u64 := do
    let Counter { value := value } := move_from<Counter>(addr)
    return value

  spec remove where
    requires exists<Counter>(addr)
    modifies global<Counter>(addr)
    ensures result == old(global<Counter>(addr).value)

  verify remove

  public entry fun record(addr : Address, amount : u64) -> Unit := do
    let value := &mut Counter[addr].value
    *value := *value + amount
    let total := &mut Ledger[addr].total
    *total := *total + amount

  spec record where
    requires exists<Counter>(addr) && exists<Ledger>(addr)
    modifies global<Counter>(addr)
    modifies global<Ledger>(addr)
    ensures global<Counter>(addr).value ==
        old(global<Counter>(addr).value) + amount &&
      global<Ledger>(addr).total == old(global<Ledger>(addr).total) + amount
    aborts_if old(global<Counter>(addr).value) + amount > 18446744073709551615 ||
      old(global<Ledger>(addr).total) + amount > 18446744073709551615

  verify record

end LeanerLang.Tests.VerificationGlobalInv
