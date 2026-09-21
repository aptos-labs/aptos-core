-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# Global invariants

Module invariants over storage: a regular invariant holds in every state; an
update invariant relates the state before and after each write.
-/

namespace LeanerLang.Tests.Check.Storage.GlobalInv

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
    aborts_if old(global<Counter>(addr).value) + 1 > MAX_U64

  public entry fun publish(account : &Signer, amount : u64) -> Unit :=
    move_to<Counter>(account, new Counter { value := amount })
  spec publish where
    requires 0 < amount
    modifies global<Counter>(account.address)
    ensures global<Counter>(account.address).value == amount

  public fun is_published(addr : Address) -> Bool := exists<Counter>(addr)
  spec is_published where
    ensures result == exists<Counter>(addr)

  public fun remove(addr : Address) -> u64 := do
    let Counter { value := value } := move_from<Counter>(addr)
    value
  spec remove where
    requires exists<Counter>(addr)
    modifies global<Counter>(addr)
    ensures result == old(global<Counter>(addr).value)

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
    aborts_if old(global<Counter>(addr).value) + amount > MAX_U64 ||
      old(global<Ledger>(addr).total) + amount > MAX_U64

end LeanerLang.Tests.Check.Storage.GlobalInv
