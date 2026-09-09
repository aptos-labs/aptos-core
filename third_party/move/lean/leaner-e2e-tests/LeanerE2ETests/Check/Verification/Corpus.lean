-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# Verification-v2 automation measurement

Direct ports from the frozen v0 acceptance corpus, all proved by a bare
`verify`: this file measures the automation delivered by the native
denotation rather than supplying fixture-specific proof scripts.

The first module is the mutable-reference and direct-call core of
`Move/Tests/Verification/Callees.lean`.  The second ports the storage
functions of `GlobalBorrows.lean` (`replace`, `read_whole`) and
`GlobalInv.lean` (`remove`); the v0 module invariants have no leaner
surface yet, so `remove` is ported as its function contract alone.
The third ports `Account.deposit` and `Account.withdraw`, which complete
the corpus: the field-focused global borrow
(`&mut Balance[addr].balance.value`) is the shape V6 added, and
`withdraw`'s guarded abort is the branch the denotation subset gained
with it.
-/

namespace LeanerLang.Tests.VerificationV2

leaner module 0x42::verification_v2_callees where
  public fun bump(slot : &mut u64) -> Unit := do
    *slot := *slot + 1

  spec bump where
    ensures slot == old(slot) + 1
    aborts_if slot + 1 > 18446744073709551615

  public fun bump_twice(slot : &mut u64) -> Unit := do
    core.call bump::<>(&mut *slot)
    core.call bump::<>(&mut *slot)

  spec bump_twice where
    ensures slot == old(slot) + 2
    aborts_if slot + 2 > 18446744073709551615

  public fun take_and_bump(slot : &mut u64) -> u64 := do
    let before := *slot
    core.call bump::<>(&mut *slot)
    return before

  spec take_and_bump where
    ensures result == old(slot)
    ensures slot == old(slot) + 1
    aborts_if slot + 1 > 18446744073709551615

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

  verify bump
  verify bump_twice
  verify take_and_bump
  verify set_pair
  verify forward_set_pair

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
    return current.value

  spec read_whole where
    requires exists<Counter>(addr)
    ensures result == old(global<Counter>(addr).value)
    aborts_if false

  public fun remove(addr : Address) -> u64 := do
    let Counter { value := value } := move_from<Counter>(addr)
    return value

  spec remove where
    requires exists<Counter>(addr)
    modifies global<Counter>(addr)
    ensures result == old(global<Counter>(addr).value)

  verify replace
  verify read_whole
  verify remove

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
        > 18446744073709551615

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

  verify deposit
  verify withdraw


end LeanerLang.Tests.VerificationV2
