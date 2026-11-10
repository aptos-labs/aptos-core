-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-! Optional storage observations retain the raw memory equation while converting
its found value. An unrelated resource invariant must not make that option
normalization recur at a concrete call of a generic function. -/
verex module 0x42::generic_storage_invariants where
  pragma verify = false

  struct Counter has Key where
    value : u64

  struct Coin {C : phantom type} has Key where
    value : u64

  struct Bounded has Key where
    value : u64

  spec Bounded where
    invariant value <= 100

  struct A where
    dummy_field : Bool

  struct B where
    dummy_field : Bool

  public fun deposit {C}(addr : Address, amount : u64) -> Unit := do
    let coin := &mut Coin<C>[addr]
    coin.value := coin.value + amount

  spec deposit where
    aborts_if !exists<Coin<C> >(addr)
    aborts_if global<Coin<C> >(addr).value + amount > MAX_U64
    ensures global<Coin<C> >(addr).value
        == old(global<Coin<C> >(addr).value) + amount
    modifies global<Coin<C> >(addr), *

  public fun deposit_a(addr : Address, amount : u64) -> Unit := do
    deposit::<A>(addr, amount)

  spec deposit_a where
    aborts_if !exists<Coin<A> >(addr)
    aborts_if global<Coin<A> >(addr).value + amount > MAX_U64
    ensures global<Coin<A> >(addr).value
        == old(global<Coin<A> >(addr).value) + amount
    ensures exists<Coin<B> >(addr) == old(exists<Coin<B> >(addr))
    ensures global<Counter>(addr) == old(global<Counter>(addr))
    modifies *

  public fun break_bounded(addr : Address) -> Unit := do
    let value := &mut Bounded[addr].value
    *value := 101
  spec break_bounded where
    requires exists<Bounded>(addr)
    modifies global<Bounded>(addr)
    aborts_if false

verify 0x42::generic_storage_invariants::deposit
verify 0x42::generic_storage_invariants::deposit_a

/--
error: the data invariant `invariant value <= 100` does not hold where a mutation ends
---
error: verex verification failed: the automatic verification of `break_bounded` failed; provide a proof in the module:
  verify break_bounded by lean =>
    verex_obligations
    verex_on
    | data_invariant_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify 0x42::generic_storage_invariants::break_bounded
