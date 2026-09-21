-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# Automation corpus

Functions verified with no proof script: this file measures the automation
of the denotation rather than supplying fixture-specific proofs.

The first module is a mutable-reference and direct-call core. The second
holds storage functions (`replace`, `read_whole`, `remove`); `remove` is
checked against its function contract alone. The third holds an account's
`deposit` and `withdraw`: a field-focused global borrow
(`&mut Balance[addr].balance.value`) and a guarded abort.
-/

namespace LeanerLang.Tests.Check.Examples.Corpus

leaner module 0x42::verification_v2_callees where
  public fun bump(slot : &mut u64) -> Unit := do
    *slot := *slot + 1
  spec bump where
    ensures slot == old(slot) + 1
    aborts_if slot + 1 > MAX_U64

  public fun bump_twice(slot : &mut u64) -> Unit := do
    core.call bump::<>(&mut *slot)
    core.call bump::<>(&mut *slot)
  spec bump_twice where
    ensures slot == old(slot) + 2
    aborts_if slot + 2 > MAX_U64

  public fun take_and_bump(slot : &mut u64) -> u64 := do
    let before := *slot
    core.call bump::<>(&mut *slot)
    before
  spec take_and_bump where
    ensures result == old(slot)
    ensures slot == old(slot) + 1
    aborts_if slot + 1 > MAX_U64

  public fun set_pair(left : &mut u64, right : &mut u64) -> Unit := do
    *left := 10
    *right := 20
  spec set_pair where
    ensures left == 10
    ensures right == 20
    aborts_if false

  public fun forward_set_pair(left : &mut u64, right : &mut u64) -> Unit :=
    core.call set_pair::<>(&mut *left, &mut *right)
  spec forward_set_pair where
    ensures left == 10
    ensures right == 20
    aborts_if false

leaner module 0x42::verification_v2_storage where
  struct Counter has Key where
    value : u64

  public entry fun replace(addr : Address, amount : u64) -> Unit := do
    let counter := &mut Counter[addr]
    *counter := new Counter { value := amount }
  spec replace where
    requires exists<Counter>(addr)
    modifies global<Counter>(addr)
    ensures global<Counter>(addr).value == amount
    aborts_if false

  public fun read_whole(addr : Address) -> u64 := do
    let counter := &Counter[addr]
    let current := *counter
    current.value
  spec read_whole where
    requires exists<Counter>(addr)
    ensures result == old(global<Counter>(addr).value)
    aborts_if false

  public fun remove(addr : Address) -> u64 := do
    let Counter { value := value } := move_from<Counter>(addr)
    value
  spec remove where
    requires exists<Counter>(addr)
    modifies global<Counter>(addr)
    ensures result == old(global<Counter>(addr).value)

leaner module 0x42::verification_v2_account where
  struct BalanceValue has Copy, Drop, Store where
    value : u64

  struct Balance has Key where
    balance : BalanceValue

  public entry fun deposit(addr : Address, amount : u64) -> Unit := do
    let value := &mut Balance[addr].balance.value
    *value := *value + amount
  spec deposit where
    requires exists<Balance>(addr)
    modifies global<Balance>(addr)
    ensures global<Balance>(addr).balance.value
        == old(global<Balance>(addr).balance.value) + amount
    aborts_if old(global<Balance>(addr).balance.value) + amount
        > MAX_U64

  public entry fun withdraw(addr : Address, amount : u64) -> Unit := do
    let value := &mut Balance[addr].balance.value
    let current := *value
    if current < amount then abort(1)
    *value := *value - amount
  spec withdraw where
    requires exists<Balance>(addr)
    modifies global<Balance>(addr)
    ensures global<Balance>(addr).balance.value
        == old(global<Balance>(addr).balance.value) - amount
    aborts_if old(global<Balance>(addr).balance.value) < amount

end LeanerLang.Tests.Check.Examples.Corpus
