-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# Verification-cost benchmark

A curated set of targets, one per cost class, each proved by a bare
`verify`.  `#leaner_perf` compares what they cost with
[`Performance.exp`](Performance.exp) and fails when a target regresses,
naming whether search or term size moved.

The set is deliberately separate from the behavioural fixtures: a benchmark
whose targets change when a feature test changes measures nothing.  Add a
class here only when it costs differently, and record why below.

| Target | Cost class |
|---|---|
| `bump` | a scalar mutable reference: the floor |
| `bump_twice` | repeated nested calls: write-back reconciliation |
| `take_and_bump` | a call whose result feeds arithmetic |
| `reborrow` | a mutable reference returned across a call boundary |
| `forward_reborrow` | a mutable reference forwarded across two call boundaries |
| `set_then_read` | returned-loan mutation followed by explicit lifetime death |
| `set_through_forward` | mutation through a twice-forwarded returned loan |
| `replace` | a whole-resource storage borrow |
| `deposit` | a field-focused storage borrow through a holder |
| `guarded` | checked arithmetic with a declared abort |
| `withdraw` | a branch guarding an abort, over a focused borrow |
| `balance_of` | a field read through a shared storage borrow |
| `is_published` | a storage existence test |
| `carry_u64` | a generic callee at a concrete instantiation |
-/

namespace LeanerLang.Tests.Performance

/- Measurement needs the proof to be finished when the command returns:
asynchronous elaboration would report the cost of handing the theorem to a
task rather than the cost of proving it. -/
set_option Elab.async false

#leaner_measure

leaner module 0x42::perf_calls where
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

  public fun take_and_bump(slot : &mut u64, amount : u64) -> u64 := do
    core.call bump::<>(&mut *slot)
    return *slot + amount

  spec take_and_bump where
    ensures result == old(slot) + 1 + amount
    aborts_if slot + 1 > 18446744073709551615
    aborts_if slot + 1 + amount > 18446744073709551615

  public fun guarded(value : u64, limit : u64) -> u64 := value + limit

  spec guarded where
    ensures result == value + limit
    aborts_if value + limit > 18446744073709551615

  verify bump
  verify bump_twice
  verify take_and_bump
  verify guarded

leaner module 0x42::perf_references where
  public fun reborrow(slot : &mut u64) -> &mut u64 := &mut *slot

  spec reborrow where
    ensures result == old(slot) && slot == result
    aborts_if false

  public fun forward_reborrow(slot : &mut u64) -> &mut u64 :=
    core.call reborrow::<>(&mut *slot)

  spec forward_reborrow where
    ensures result == old(slot) && slot == result
    aborts_if false

  public fun set_then_read(slot : &mut u64) -> u64 := do
    let returned := core.call reborrow::<>(&mut *slot)
    *returned := 7
    return *slot

  spec set_then_read where
    ensures result == 7 && slot == 7
    aborts_if false

  public fun set_through_forward(slot : &mut u64) -> Unit := do
    let returned := core.call forward_reborrow::<>(&mut *slot)
    *returned := 9

  spec set_through_forward where
    ensures slot == 9
    aborts_if false

  verify reborrow
  verify forward_reborrow
  verify set_then_read
  verify set_through_forward

leaner module 0x42::perf_storage where
  struct Amount has Copy, Drop, Store where
    value : u64

  struct Coin has Key where
    amount : Amount

  public entry fun replace(addr : Address, value : u64) -> Unit := do
    let coin := &mut Coin[addr]
    *coin := new Coin { amount := new Amount { value := value } }

  spec replace where
    requires exists<Coin>(addr)
    modifies global<Coin>(addr)
    ensures global<Coin>(addr).amount.value == value
    aborts_if false

  public entry fun deposit(addr : Address, amount : u64) -> Unit := do
    let value := &mut Coin[addr].amount.value
    *value := *value + amount

  spec deposit where
    requires exists<Coin>(addr)
    modifies global<Coin>(addr)
    ensures global<Coin>(addr).amount.value
        == old(global<Coin>(addr).amount.value) + amount
    aborts_if old(global<Coin>(addr).amount.value) + amount
        > 18446744073709551615

  public entry fun withdraw(addr : Address, amount : u64) -> Unit := do
    let value := &mut Coin[addr].amount.value
    let current := *value
    if current < amount then abort(1)
    *value := *value - amount

  spec withdraw where
    requires exists<Coin>(addr)
    modifies global<Coin>(addr)
    ensures global<Coin>(addr).amount.value
        == old(global<Coin>(addr).amount.value) - amount
    aborts_if old(global<Coin>(addr).amount.value) < amount

  public fun balance_of(addr : Address) -> u64 := Coin[addr].amount.value

  spec balance_of where
    aborts_if !exists<Coin>(addr)
    ensures result == global<Coin>(addr).amount.value

  public fun is_published(addr : Address) -> Bool := exists<Coin>(addr)

  spec is_published where
    ensures result == exists<Coin>(addr)

  verify replace
  verify deposit
  verify withdraw
  verify balance_of
  verify is_published

leaner module 0x42::perf_generics where
  public fun carry {T : type}(value : T) -> T := value

  spec carry where
    pragma aborts_if_is_strict
    ensures result == value

  public fun carry_u64(value : u64) -> u64 :=
    core.call carry::<u64>(value)

  spec carry_u64 where
    pragma aborts_if_is_strict
    ensures result == value

  verify carry
  verify carry_u64


#leaner_perf "LeanerLang/Tests/Performance.exp"

end LeanerLang.Tests.Performance
