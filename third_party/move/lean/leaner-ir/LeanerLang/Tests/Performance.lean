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
| `vector_get` | checked indexing into a local vector literal |
| `vector_set` | checked indexed assignment to a local vector literal |
| `value_or` | a shared reference matched through an enum payload |
| `scale` | a mutable reference matched before payload-field updates |
| `shift` | a cross-resource module invariant over two modified families |
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

set_option leaner.route "native" in
#leaner_verify 0x42::perf_calls::guarded

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

set_option leaner.route "native" in
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

leaner module 0x42::perf_vectors where
  public fun vector_get(index : u64) -> u64 := do
    let values : Vector<u64> := vector<u64>[10, 20, 30]
    return values[index]

  spec vector_get where
    ensures index == 0 ==> result == 10
    ensures index == 1 ==> result == 20
    ensures index == 2 ==> result == 30
    aborts_if index >= 3

  public fun vector_set(index : u64) -> Vector<u64> := do
    let mut values : Vector<u64> := vector<u64>[10, 20, 30]
    values[index] := 7
    return values

  spec vector_set where
    ensures index == 0 ==> result == vector<u64>[7, 20, 30]
    ensures index == 1 ==> result == vector<u64>[10, 7, 30]
    ensures index == 2 ==> result == vector<u64>[10, 20, 7]
    aborts_if index >= 3

  verify vector_get
  verify vector_set

leaner module 0x42::perf_enum_refs where
  enum Slot has Copy, Drop where
    | Empty
    | Filled (value : u64)

  enum Shape has Copy, Drop where
    | Circle (radius : u64)
    | Rectangle (width : u64, height : u64)

  public fun value_or(slot : &Slot, default : u64) -> u64 :=
    match slot with
    | Slot::Filled { value := value } => *value
    | Slot::Empty {} => default

  spec value_or where
    ensures result == (match slot with
      | Slot::Filled { value := value } => value
      | Slot::Empty {} => default)
    aborts_if false

  public fun scale(shape : &mut Shape, factor : u64) -> Unit :=
    match shape with
    | Shape::Circle { radius := radius } =>
        *radius := *radius * factor
    | Shape::Rectangle { width := width, height := height } => do
        *width := *width * factor
        *height := *height * factor

  spec scale where
    pragma aborts_if_is_partial
    ensures shape == (match old(shape) with
      | Shape::Circle { radius := radius } =>
          new Shape::Circle { radius := radius * factor }
      | Shape::Rectangle { width := width, height := height } =>
          new Shape::Rectangle {
            width := width * factor, height := height * factor })

  verify value_or
  verify scale

leaner module 0x42::perf_invariant where
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


#leaner_perf "LeanerLang/Tests/Performance.exp"

end LeanerLang.Tests.Performance
